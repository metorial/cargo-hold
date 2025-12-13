diesel::table! {
    tenants (oid) {
        oid -> Int8,
        id -> Varchar,
        name -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        total_files_bytes -> Int8,
        file_count -> Int8,
    }
}

diesel::table! {
    purposes (oid) {
        oid -> Int8,
        id -> Varchar,
        slug -> Varchar,
    }
}

diesel::table! {
    files (oid) {
        oid -> Int8,
        id -> Varchar,
        tenant_oid -> Int8,
        filename -> Varchar,
        purpose_oid -> Int8,
        bytes -> Int8,
        storage_key -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    file_links (oid) {
        oid -> Int8,
        id -> Varchar,
        file_oid -> Int8,
        key -> Varchar,
        expires_at -> Timestamp,
        created_at -> Timestamp,
    }
}

diesel::joinable!(files -> tenants (tenant_oid));
diesel::joinable!(files -> purposes (purpose_oid));
diesel::joinable!(file_links -> files (file_oid));

diesel::allow_tables_to_appear_in_same_query!(tenants, purposes, files, file_links,);
