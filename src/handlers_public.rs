use crate::app_state::AppState;
use crate::models::*;
use crate::schema::*;
use axum::{
    extract::{Multipart, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use bytes::Bytes;
use chrono::Utc;
use diesel::prelude::*;

pub async fn upload_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<FileResponse>, AppError> {
    let tenant_id = headers
        .get("X-Tenant-ID")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::BadRequest("Missing X-Tenant-ID header".to_string()))?;

    let mut file_data: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;
    let mut purpose_slug: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Failed to read multipart field: {}", e)))?
    {
        let field_name = field.name().unwrap_or("").to_string();

        match field_name.as_str() {
            "file" => {
                let field_filename = field.file_name().map(|s| s.to_string());
                filename = field_filename;

                let data = field.bytes().await.map_err(|e| {
                    AppError::BadRequest(format!("Failed to read file data: {}", e))
                })?;

                if data.len() as i64 > state.config.max_file_size_bytes {
                    return Err(AppError::BadRequest(format!(
                        "File size exceeds maximum of {} bytes",
                        state.config.max_file_size_bytes
                    )));
                }

                file_data = Some(data.to_vec());
            }
            "purpose" => {
                let text = field.text().await.map_err(|e| {
                    AppError::BadRequest(format!("Failed to read purpose field: {}", e))
                })?;
                purpose_slug = Some(text);
            }
            _ => {}
        }
    }

    let file_data = file_data.ok_or_else(|| AppError::BadRequest("Missing file".to_string()))?;
    let filename = filename.ok_or_else(|| AppError::BadRequest("Missing filename".to_string()))?;
    let purpose_slug =
        purpose_slug.ok_or_else(|| AppError::BadRequest("Missing purpose".to_string()))?;

    let mut conn = state.db_pool.get().map_err(|_| AppError::DatabaseError)?;

    let tenant = get_or_create_tenant(&mut conn, tenant_id, &state)?;

    let purpose: Purpose = purposes::table
        .filter(purposes::slug.eq(&purpose_slug))
        .first(&mut conn)
        .map_err(|_| AppError::BadRequest(format!("Invalid purpose: {}", purpose_slug)))?;

    let file_oid = state
        .snowflake_gen
        .generate()
        .map_err(|_| AppError::InternalError)?;
    let file_id = crate::snowflake::generate_prefixed_id("file", file_oid);
    let storage_key = format!("{}/{}", tenant.id, file_id);

    state
        .storage_client
        .upload(&storage_key, Bytes::from(file_data.clone()), None)
        .await
        .map_err(|e| AppError::StorageError(e.to_string()))?;

    let new_file = NewFile {
        oid: file_oid,
        id: file_id.clone(),
        tenant_oid: tenant.oid,
        filename: filename.clone(),
        purpose_oid: purpose.oid,
        bytes: file_data.len() as i64,
        storage_key,
    };

    let file: File = diesel::insert_into(files::table)
        .values(&new_file)
        .get_result(&mut conn)
        .map_err(|_| AppError::DatabaseError)?;

    diesel::update(tenants::table.find(tenant.oid))
        .set((
            tenants::total_files_bytes.eq(tenants::total_files_bytes + file.bytes),
            tenants::file_count.eq(tenants::file_count + 1),
            tenants::updated_at.eq(Utc::now().naive_utc()),
        ))
        .execute(&mut conn)
        .map_err(|_| AppError::DatabaseError)?;

    Ok(Json(FileResponse {
        id: file.id,
        object: "file".to_string(),
        bytes: file.bytes,
        created_at: file.created_at.and_utc().timestamp(),
        updated_at: file.updated_at.and_utc().timestamp(),
        filename: file.filename,
        purpose: purpose.slug,
        tenant_id: None,
    }))
}

pub async fn get_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(file_id): Path<String>,
) -> Result<Json<FileResponse>, AppError> {
    let tenant_id = headers
        .get("X-Tenant-ID")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::BadRequest("Missing X-Tenant-ID header".to_string()))?;

    let mut conn = state.db_pool.get().map_err(|_| AppError::DatabaseError)?;

    let tenant = get_or_create_tenant(&mut conn, tenant_id, &state)?;

    let file: File = files::table
        .filter(files::id.eq(&file_id))
        .filter(files::tenant_oid.eq(tenant.oid))
        .first(&mut conn)
        .map_err(|_| AppError::NotFound)?;

    let purpose: Purpose = purposes::table
        .find(file.purpose_oid)
        .first(&mut conn)
        .map_err(|_| AppError::DatabaseError)?;

    Ok(Json(FileResponse {
        id: file.id,
        object: "file".to_string(),
        bytes: file.bytes,
        created_at: file.created_at.and_utc().timestamp(),
        updated_at: file.updated_at.and_utc().timestamp(),
        filename: file.filename,
        purpose: purpose.slug,
        tenant_id: None,
    }))
}

pub async fn get_file_content(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(file_id): Path<String>,
) -> Result<Response, AppError> {
    let tenant_id = headers
        .get("X-Tenant-ID")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::BadRequest("Missing X-Tenant-ID header".to_string()))?;

    let mut conn = state.db_pool.get().map_err(|_| AppError::DatabaseError)?;

    let tenant = get_or_create_tenant(&mut conn, tenant_id, &state)?;

    let file: File = files::table
        .filter(files::id.eq(&file_id))
        .filter(files::tenant_oid.eq(tenant.oid))
        .first(&mut conn)
        .map_err(|_| AppError::NotFound)?;

    let content = state
        .storage_client
        .download(&file.storage_key)
        .await
        .map_err(|e| AppError::StorageError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        [("Content-Type", "application/octet-stream")],
        content,
    )
        .into_response())
}

fn get_or_create_tenant(
    conn: &mut PgConnection,
    tenant_id_str: &str,
    state: &AppState,
) -> Result<Tenant, AppError> {
    let existing: Option<Tenant> = tenants::table
        .filter(tenants::id.eq(tenant_id_str))
        .first(conn)
        .optional()
        .map_err(|_| AppError::DatabaseError)?;

    if let Some(tenant) = existing {
        return Ok(tenant);
    }

    let tenant_oid = state
        .snowflake_gen
        .generate()
        .map_err(|_| AppError::InternalError)?;
    let tenant_id = crate::snowflake::generate_prefixed_id("tenant", tenant_oid);

    let new_tenant = NewTenant {
        oid: tenant_oid,
        id: tenant_id,
        name: tenant_id_str.to_string(),
    };

    diesel::insert_into(tenants::table)
        .values(&new_tenant)
        .get_result(conn)
        .map_err(|_| AppError::DatabaseError)
}

#[derive(Debug)]
pub enum AppError {
    BadRequest(String),
    NotFound,
    DatabaseError,
    StorageError(String),
    InternalError,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::NotFound => (StatusCode::NOT_FOUND, "Not found".to_string()),
            AppError::DatabaseError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            ),
            AppError::StorageError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::InternalError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal error".to_string(),
            ),
        };

        (status, message).into_response()
    }
}
