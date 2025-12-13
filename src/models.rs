use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Identifiable, Debug)]
#[diesel(table_name = crate::schema::tenants)]
#[diesel(primary_key(oid))]
pub struct Tenant {
    pub oid: i64,
    pub id: String,
    pub name: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub total_files_bytes: i64,
    pub file_count: i64,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::tenants)]
pub struct NewTenant {
    pub oid: i64,
    pub id: String,
    pub name: String,
}

#[derive(Queryable, Selectable, Identifiable, Debug)]
#[diesel(table_name = crate::schema::purposes)]
#[diesel(primary_key(oid))]
pub struct Purpose {
    pub oid: i64,
    pub id: String,
    pub slug: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::purposes)]
pub struct NewPurpose {
    pub oid: i64,
    pub id: String,
    pub slug: String,
}

#[derive(Queryable, Selectable, Identifiable, Associations, Debug)]
#[diesel(table_name = crate::schema::files)]
#[diesel(belongs_to(Tenant, foreign_key = tenant_oid))]
#[diesel(belongs_to(Purpose, foreign_key = purpose_oid))]
#[diesel(primary_key(oid))]
pub struct File {
    pub oid: i64,
    pub id: String,
    pub tenant_oid: i64,
    pub filename: String,
    pub purpose_oid: i64,
    pub bytes: i64,
    pub storage_key: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::files)]
pub struct NewFile {
    pub oid: i64,
    pub id: String,
    pub tenant_oid: i64,
    pub filename: String,
    pub purpose_oid: i64,
    pub bytes: i64,
    pub storage_key: String,
}

#[derive(AsChangeset)]
#[diesel(table_name = crate::schema::files)]
pub struct UpdateFile {
    pub filename: Option<String>,
    pub purpose_oid: Option<i64>,
    pub updated_at: NaiveDateTime,
}

#[derive(Queryable, Selectable, Identifiable, Associations, Debug)]
#[diesel(table_name = crate::schema::file_links)]
#[diesel(belongs_to(File, foreign_key = file_oid))]
#[diesel(primary_key(oid))]
pub struct FileLink {
    pub oid: i64,
    pub id: String,
    pub file_oid: i64,
    pub key: String,
    pub expires_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::file_links)]
pub struct NewFileLink {
    pub oid: i64,
    pub id: String,
    pub file_oid: i64,
    pub key: String,
    pub expires_at: NaiveDateTime,
}

#[derive(Serialize, Deserialize)]
pub struct FileResponse {
    pub id: String,
    pub object: String,
    pub bytes: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub filename: String,
    pub purpose: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct FileLinkResponse {
    pub id: String,
    pub object: String,
    pub file_id: String,
    pub key: String,
    pub expires_at: i64,
    pub created_at: i64,
}

#[derive(Serialize, Deserialize)]
pub struct ListFilesResponse {
    pub items: Vec<FileResponse>,
    pub pagination: PaginationResponse,
}

#[derive(Serialize, Deserialize)]
pub struct PaginationResponse {
    pub has_more_before: bool,
    pub has_more_after: bool,
}

#[derive(Deserialize)]
pub struct CreateLinkRequest {
    pub expires_in: i64,
    pub file_id: String,
    pub key: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateFileRequest {
    pub filename: Option<String>,
    pub purpose: Option<String>,
}

#[derive(Deserialize)]
pub struct ListFilesQuery {
    pub tenant_id: Option<String>,
    pub limit: Option<i64>,
    pub order: Option<String>,
    pub before: Option<String>,
    pub after: Option<String>,
}
