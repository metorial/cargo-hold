use crate::app_state::AppState;
use crate::handlers_public::AppError;
use crate::models::*;
use crate::schema::*;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{Duration, Utc};
use diesel::prelude::*;

pub async fn delete_file(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
) -> Result<Json<FileResponse>, AppError> {
    let mut conn = state.db_pool.get().map_err(|_| AppError::DatabaseError)?;

    let file: File = files::table
        .filter(files::id.eq(&file_id))
        .first(&mut conn)
        .map_err(|_| AppError::NotFound)?;

    let tenant: Tenant = tenants::table
        .find(file.tenant_oid)
        .first(&mut conn)
        .map_err(|_| AppError::DatabaseError)?;

    let purpose: Purpose = purposes::table
        .find(file.purpose_oid)
        .first(&mut conn)
        .map_err(|_| AppError::DatabaseError)?;

    state
        .storage_client
        .delete(&file.storage_key)
        .await
        .map_err(|e| AppError::StorageError(e.to_string()))?;

    diesel::delete(files::table.find(file.oid))
        .execute(&mut conn)
        .map_err(|_| AppError::DatabaseError)?;

    diesel::update(tenants::table.find(tenant.oid))
        .set((
            tenants::total_files_bytes.eq(tenants::total_files_bytes - file.bytes),
            tenants::file_count.eq(tenants::file_count - 1),
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
        tenant_id: Some(tenant.id),
    }))
}

pub async fn update_file(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
    Json(payload): Json<UpdateFileRequest>,
) -> Result<Json<FileResponse>, AppError> {
    let mut conn = state.db_pool.get().map_err(|_| AppError::DatabaseError)?;

    let file: File = files::table
        .filter(files::id.eq(&file_id))
        .first(&mut conn)
        .map_err(|_| AppError::NotFound)?;

    let mut purpose_oid = file.purpose_oid;

    if let Some(ref purpose_slug) = payload.purpose {
        let purpose: Purpose = purposes::table
            .filter(purposes::slug.eq(purpose_slug))
            .first(&mut conn)
            .map_err(|_| AppError::BadRequest(format!("Invalid purpose: {}", purpose_slug)))?;
        purpose_oid = purpose.oid;
    }

    let update = UpdateFile {
        filename: payload.filename,
        purpose_oid: if payload.purpose.is_some() {
            Some(purpose_oid)
        } else {
            None
        },
        updated_at: Utc::now().naive_utc(),
    };

    let updated_file: File = diesel::update(files::table.find(file.oid))
        .set(&update)
        .get_result(&mut conn)
        .map_err(|_| AppError::DatabaseError)?;

    let tenant: Tenant = tenants::table
        .find(updated_file.tenant_oid)
        .first(&mut conn)
        .map_err(|_| AppError::DatabaseError)?;

    let purpose: Purpose = purposes::table
        .find(updated_file.purpose_oid)
        .first(&mut conn)
        .map_err(|_| AppError::DatabaseError)?;

    Ok(Json(FileResponse {
        id: updated_file.id,
        object: "file".to_string(),
        bytes: updated_file.bytes,
        created_at: updated_file.created_at.and_utc().timestamp(),
        updated_at: updated_file.updated_at.and_utc().timestamp(),
        filename: updated_file.filename,
        purpose: purpose.slug,
        tenant_id: Some(tenant.id),
    }))
}

pub async fn get_file_private(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
) -> Result<Json<FileResponse>, AppError> {
    let mut conn = state.db_pool.get().map_err(|_| AppError::DatabaseError)?;

    let file: File = files::table
        .filter(files::id.eq(&file_id))
        .first(&mut conn)
        .map_err(|_| AppError::NotFound)?;

    let tenant: Tenant = tenants::table
        .find(file.tenant_oid)
        .first(&mut conn)
        .map_err(|_| AppError::DatabaseError)?;

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
        tenant_id: Some(tenant.id),
    }))
}

pub async fn list_files(
    State(state): State<AppState>,
    Query(query): Query<ListFilesQuery>,
) -> Result<Json<ListFilesResponse>, AppError> {
    let mut conn = state.db_pool.get().map_err(|_| AppError::DatabaseError)?;

    let limit = query.limit.unwrap_or(10).clamp(1, 100);
    let order = query.order.unwrap_or_else(|| "desc".to_string());

    let mut base_query = files::table.into_boxed();

    if let Some(tenant_id_str) = &query.tenant_id {
        let tenant: Tenant = tenants::table
            .filter(tenants::id.eq(tenant_id_str))
            .first(&mut conn)
            .map_err(|_| AppError::BadRequest("Invalid tenant_id".to_string()))?;
        base_query = base_query.filter(files::tenant_oid.eq(tenant.oid));
    }

    if let Some(after_id) = &query.after {
        let after_file: File = files::table
            .filter(files::id.eq(after_id))
            .first(&mut conn)
            .map_err(|_| AppError::BadRequest("Invalid after id".to_string()))?;
        if order == "asc" {
            base_query = base_query.filter(files::oid.gt(after_file.oid));
        } else {
            base_query = base_query.filter(files::oid.lt(after_file.oid));
        }
    }

    if let Some(before_id) = &query.before {
        let before_file: File = files::table
            .filter(files::id.eq(before_id))
            .first(&mut conn)
            .map_err(|_| AppError::BadRequest("Invalid before id".to_string()))?;
        if order == "asc" {
            base_query = base_query.filter(files::oid.lt(before_file.oid));
        } else {
            base_query = base_query.filter(files::oid.gt(before_file.oid));
        }
    }

    if order == "asc" {
        base_query = base_query.order(files::oid.asc());
    } else {
        base_query = base_query.order(files::oid.desc());
    }

    let files_list: Vec<File> = base_query
        .limit(limit + 1)
        .load(&mut conn)
        .map_err(|_| AppError::DatabaseError)?;

    let has_more = files_list.len() as i64 > limit;
    let files_to_return: Vec<File> = files_list.into_iter().take(limit as usize).collect();

    let mut file_responses = Vec::new();

    for file in &files_to_return {
        let tenant: Tenant = tenants::table
            .find(file.tenant_oid)
            .first(&mut conn)
            .map_err(|_| AppError::DatabaseError)?;

        let purpose: Purpose = purposes::table
            .find(file.purpose_oid)
            .first(&mut conn)
            .map_err(|_| AppError::DatabaseError)?;

        file_responses.push(FileResponse {
            id: file.id.clone(),
            object: "file".to_string(),
            bytes: file.bytes,
            created_at: file.created_at.and_utc().timestamp(),
            updated_at: file.updated_at.and_utc().timestamp(),
            filename: file.filename.clone(),
            purpose: purpose.slug,
            tenant_id: Some(tenant.id),
        });
    }

    let has_more_after = if order == "desc" { has_more } else { false };
    let has_more_before = if order == "asc" { has_more } else { false };

    Ok(Json(ListFilesResponse {
        items: file_responses,
        pagination: PaginationResponse {
            has_more_before,
            has_more_after,
        },
    }))
}

pub async fn create_link(
    State(state): State<AppState>,
    Json(payload): Json<CreateLinkRequest>,
) -> Result<Json<FileLinkResponse>, AppError> {
    let mut conn = state.db_pool.get().map_err(|_| AppError::DatabaseError)?;

    let file: File = files::table
        .filter(files::id.eq(&payload.file_id))
        .first(&mut conn)
        .map_err(|_| AppError::BadRequest("Invalid file_id".to_string()))?;

    let link_oid = state
        .snowflake_gen
        .generate()
        .map_err(|_| AppError::InternalError)?;
    let link_id = crate::snowflake::generate_prefixed_id("link", link_oid);

    let key = if let Some(custom_key) = payload.key {
        custom_key
    } else {
        use rand::Rng;
        rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(16)
            .map(char::from)
            .collect()
    };

    let expires_at = Utc::now().naive_utc() + Duration::seconds(payload.expires_in);

    let new_link = NewFileLink {
        oid: link_oid,
        id: link_id.clone(),
        file_oid: file.oid,
        key: key.clone(),
        expires_at,
    };

    let link: FileLink = diesel::insert_into(file_links::table)
        .values(&new_link)
        .get_result(&mut conn)
        .map_err(|_| AppError::DatabaseError)?;

    Ok(Json(FileLinkResponse {
        id: link.id,
        object: "file_link".to_string(),
        file_id: file.id,
        key: link.key,
        expires_at: link.expires_at.and_utc().timestamp(),
        created_at: link.created_at.and_utc().timestamp(),
    }))
}

pub async fn get_link(
    State(state): State<AppState>,
    Path(link_id): Path<String>,
) -> Result<Json<FileLinkResponse>, AppError> {
    let mut conn = state.db_pool.get().map_err(|_| AppError::DatabaseError)?;

    let link: FileLink = file_links::table
        .filter(file_links::id.eq(&link_id))
        .first(&mut conn)
        .map_err(|_| AppError::NotFound)?;

    let file: File = files::table
        .find(link.file_oid)
        .first(&mut conn)
        .map_err(|_| AppError::DatabaseError)?;

    Ok(Json(FileLinkResponse {
        id: link.id,
        object: "file_link".to_string(),
        file_id: file.id,
        key: link.key,
        expires_at: link.expires_at.and_utc().timestamp(),
        created_at: link.created_at.and_utc().timestamp(),
    }))
}

pub async fn delete_link(
    State(state): State<AppState>,
    Path(link_id): Path<String>,
) -> Result<Json<FileLinkResponse>, AppError> {
    let mut conn = state.db_pool.get().map_err(|_| AppError::DatabaseError)?;

    let link: FileLink = file_links::table
        .filter(file_links::id.eq(&link_id))
        .first(&mut conn)
        .map_err(|_| AppError::NotFound)?;

    let file: File = files::table
        .find(link.file_oid)
        .first(&mut conn)
        .map_err(|_| AppError::DatabaseError)?;

    diesel::delete(file_links::table.find(link.oid))
        .execute(&mut conn)
        .map_err(|_| AppError::DatabaseError)?;

    Ok(Json(FileLinkResponse {
        id: link.id,
        object: "file_link".to_string(),
        file_id: file.id,
        key: link.key,
        expires_at: link.expires_at.and_utc().timestamp(),
        created_at: link.created_at.and_utc().timestamp(),
    }))
}
