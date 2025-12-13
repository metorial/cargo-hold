use crate::app_state::AppState;
use crate::handlers_public::AppError;
use crate::models::*;
use crate::schema::*;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use diesel::prelude::*;

pub async fn get_file_by_link(
    State(state): State<AppState>,
    Path(link_key): Path<String>,
) -> Result<Response, AppError> {
    let mut conn = state.db_pool.get().map_err(|_| AppError::DatabaseError)?;

    let file_link: FileLink = file_links::table
        .filter(file_links::key.eq(&link_key))
        .first(&mut conn)
        .map_err(|_| AppError::NotFound)?;

    if file_link.expires_at < Utc::now().naive_utc() {
        return Err(AppError::BadRequest("Link expired".to_string()));
    }

    let file: File = files::table
        .find(file_link.file_oid)
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
