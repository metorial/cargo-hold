CREATE TABLE tenants (
    oid BIGINT PRIMARY KEY,
    id VARCHAR(255) NOT NULL UNIQUE,
    name VARCHAR(255) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    total_files_bytes BIGINT NOT NULL DEFAULT 0,
    file_count BIGINT NOT NULL DEFAULT 0
);

CREATE INDEX idx_tenants_id ON tenants(id);

CREATE TABLE purposes (
    oid BIGINT PRIMARY KEY,
    id VARCHAR(255) NOT NULL UNIQUE,
    slug VARCHAR(255) NOT NULL UNIQUE
);

CREATE INDEX idx_purposes_slug ON purposes(slug);

CREATE TABLE files (
    oid BIGINT PRIMARY KEY,
    id VARCHAR(255) NOT NULL UNIQUE,
    tenant_oid BIGINT NOT NULL REFERENCES tenants(oid) ON DELETE CASCADE,
    filename VARCHAR(255) NOT NULL,
    purpose_oid BIGINT NOT NULL REFERENCES purposes(oid),
    bytes BIGINT NOT NULL,
    storage_key VARCHAR(512) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_files_id ON files(id);
CREATE INDEX idx_files_tenant_oid ON files(tenant_oid);
CREATE INDEX idx_files_created_at ON files(created_at);

CREATE TABLE file_links (
    oid BIGINT PRIMARY KEY,
    id VARCHAR(255) NOT NULL UNIQUE,
    file_oid BIGINT NOT NULL REFERENCES files(oid) ON DELETE CASCADE,
    key VARCHAR(255) NOT NULL UNIQUE,
    expires_at TIMESTAMP NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_file_links_key ON file_links(key);
CREATE INDEX idx_file_links_file_oid ON file_links(file_oid);
CREATE INDEX idx_file_links_expires_at ON file_links(expires_at);
