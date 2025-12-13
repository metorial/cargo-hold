use axum::{
    body::Body,
    http::{header, Request, StatusCode},
    Router,
};
use cargo_hold::{
    handlers_private, handlers_public, handlers_unauthenticated, models::*, schema::*, startup,
    test_utils::*,
};
use diesel::prelude::*;
use serde_json::json;
use std::sync::Mutex;
use tower::ServiceExt;

static TEST_MUTEX: Mutex<()> = Mutex::new(());

async fn setup_test_router() -> (
    Router,
    cargo_hold::app_state::AppState,
    std::sync::MutexGuard<'static, ()>,
) {
    let guard = TEST_MUTEX.lock().unwrap();
    let state = create_test_app_state();

    cleanup_test_db(&state.db_pool);

    let mut conn = state.db_pool.get().unwrap();
    startup::upsert_purposes(
        &mut conn,
        &state.snowflake_gen,
        &state.config.allowed_purposes,
    )
    .unwrap();

    let router = Router::new()
        .route("/files", axum::routing::post(handlers_public::upload_file))
        .route(
            "/files/:file_id",
            axum::routing::get(handlers_public::get_file),
        )
        .route(
            "/files/:file_id/content",
            axum::routing::get(handlers_public::get_file_content),
        )
        .route(
            "/f/:link_key",
            axum::routing::get(handlers_unauthenticated::get_file_by_link),
        )
        .route(
            "/admin/files/:file_id",
            axum::routing::delete(handlers_private::delete_file),
        )
        .route(
            "/admin/files/:file_id",
            axum::routing::put(handlers_private::update_file),
        )
        .route(
            "/admin/files/:file_id",
            axum::routing::get(handlers_private::get_file_private),
        )
        .route(
            "/admin/files",
            axum::routing::get(handlers_private::list_files),
        )
        .route(
            "/admin/links",
            axum::routing::post(handlers_private::create_link),
        )
        .route(
            "/admin/links/:link_id",
            axum::routing::get(handlers_private::get_link),
        )
        .route(
            "/admin/links/:link_id",
            axum::routing::delete(handlers_private::delete_link),
        )
        .with_state(state.clone());

    (router, state, guard)
}

#[tokio::test]
async fn test_upload_file_missing_tenant_header() {
    let (router, state, _guard) = setup_test_router().await;

    let boundary = "----WebKitFormBoundary";
    let body = format!(
        "--{}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"test.txt\"\r\n\r\ntest content\r\n--{}--\r\n",
        boundary, boundary
    );

    let request = Request::builder()
        .uri("/files")
        .method("POST")
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={}", boundary),
        )
        .body(Body::from(body))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    cleanup_test_db(&state.db_pool);
}

#[tokio::test]
async fn test_get_file_not_found() {
    let (router, state, _guard) = setup_test_router().await;

    let request = Request::builder()
        .uri("/files/file_nonexistent")
        .method("GET")
        .header("X-Tenant-ID", "test-tenant")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    cleanup_test_db(&state.db_pool);
}

#[tokio::test]
async fn test_list_files_pagination() {
    let (router, state, _guard) = setup_test_router().await;

    let mut conn = state.db_pool.get().unwrap();

    let tenant_oid = state.snowflake_gen.generate().unwrap();
    let tenant = diesel::insert_into(tenants::table)
        .values(cargo_hold::models::NewTenant {
            oid: tenant_oid,
            id: cargo_hold::snowflake::generate_prefixed_id("tenant", tenant_oid),
            name: "Test Tenant".to_string(),
        })
        .get_result::<cargo_hold::models::Tenant>(&mut conn)
        .unwrap();

    let purpose: cargo_hold::models::Purpose = purposes::table
        .filter(purposes::slug.eq("test-purpose"))
        .first(&mut conn)
        .unwrap();

    for i in 0..5 {
        let file_oid = state.snowflake_gen.generate().unwrap();
        diesel::insert_into(files::table)
            .values(cargo_hold::models::NewFile {
                oid: file_oid,
                id: cargo_hold::snowflake::generate_prefixed_id("file", file_oid),
                tenant_oid: tenant.oid,
                filename: format!("test{}.txt", i),
                purpose_oid: purpose.oid,
                bytes: 100,
                storage_key: format!("test-key-{}", i),
            })
            .execute(&mut conn)
            .unwrap();
    }

    let request = Request::builder()
        .uri(format!("/admin/files?tenant_id={}&limit=3", tenant.id))
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let list_response: ListFilesResponse = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(list_response.items.len(), 3);
    assert!(list_response.pagination.has_more_after);

    cleanup_test_db(&state.db_pool);
}

#[tokio::test]
async fn test_update_file() {
    let (router, state, _guard) = setup_test_router().await;

    let mut conn = state.db_pool.get().unwrap();

    let tenant_oid = state.snowflake_gen.generate().unwrap();
    let tenant = diesel::insert_into(tenants::table)
        .values(cargo_hold::models::NewTenant {
            oid: tenant_oid,
            id: cargo_hold::snowflake::generate_prefixed_id("tenant", tenant_oid),
            name: "Test Tenant".to_string(),
        })
        .get_result::<cargo_hold::models::Tenant>(&mut conn)
        .unwrap();

    let purpose: cargo_hold::models::Purpose = purposes::table
        .filter(purposes::slug.eq("test-purpose"))
        .first(&mut conn)
        .unwrap();

    let file_oid = state.snowflake_gen.generate().unwrap();
    let file = diesel::insert_into(files::table)
        .values(cargo_hold::models::NewFile {
            oid: file_oid,
            id: cargo_hold::snowflake::generate_prefixed_id("file", file_oid),
            tenant_oid: tenant.oid,
            filename: "old-name.txt".to_string(),
            purpose_oid: purpose.oid,
            bytes: 100,
            storage_key: "test-key".to_string(),
        })
        .get_result::<cargo_hold::models::File>(&mut conn)
        .unwrap();

    let update_body = json!({
        "filename": "new-name.txt",
    });

    let request = Request::builder()
        .uri(format!("/admin/files/{}", file.id))
        .method("PUT")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&update_body).unwrap()))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let file_response: FileResponse = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(file_response.filename, "new-name.txt");

    cleanup_test_db(&state.db_pool);
}
