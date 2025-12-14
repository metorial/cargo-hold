# Cargo Hold

A multi-tenant file upload service that handles file metadata and storage coordination. Files are uploaded through authenticated APIs, stored in the [Metorial object storage service](https://github.com/metorial/object-storage), and can be shared via public links.

## Features

- Authenticated file uploads with multipart/form-data support
- Shareable links for public access (with expiration)
- File size validation and quota tracking
- Dual API architecture (public authenticated + private admin on separate ports)

## Configuration

Required environment variables:

```bash
# Database
DATABASE_URL=postgres://user:password@localhost/cargo_hold

# Server configuration
PUBLIC_HOST=0.0.0.0
PUBLIC_PORT=8080
PRIVATE_HOST=0.0.0.0
PRIVATE_PORT=8081

# Storage backend
STORAGE_BASE_URL=https://storage.example.com
STORAGE_BUCKET=my-bucket

# File validation
MAX_FILE_SIZE_BYTES=10485760
ALLOWED_PURPOSES=document,image,avatar

# Snowflake ID generation
WORKER_ID=1
DATACENTER_ID=1
```

## Usage with Docker

Pull and run the latest image from GitHub Container Registry:

```bash
docker pull ghcr.io/metorial/cargo-hold:latest

docker run -d \
  -p 8080:8080 \
  -p 8081:8081 \
  -e DATABASE_URL=postgres://user:password@db:5432/cargo_hold \
  -e STORAGE_BASE_URL=https://storage.example.com \
  -e STORAGE_BUCKET=my-bucket \
  -e ALLOWED_PURPOSES=document,image,avatar \
  ghcr.io/metorial/cargo-hold:latest
```

## API Endpoints

### Public API (Port 8080)

**Upload file**
```
POST /files
Headers: X-Tenant-ID: <tenant-id>
Body: multipart/form-data with "file" field and "purpose" field
```

**Get file metadata**
```
GET /files/:file_id
Headers: X-Tenant-ID: <tenant-id>
```

**Get file content**
```
GET /files/:file_id/content
Headers: X-Tenant-ID: <tenant-id>
```

**Access file via link (unauthenticated)**
```
GET /f/:link_key
```

### Private API (Port 8081)

**List files**
```
GET /admin/files?tenant_id=<tenant-id>&limit=10&order=desc
```

**Get file details**
```
GET /admin/files/:file_id
```

**Update file**
```
PUT /admin/files/:file_id
Body: {"filename": "new-name.txt", "purpose": "document"}
```

**Delete file**
```
DELETE /admin/files/:file_id
```

**Create shareable link**
```
POST /admin/links
Body: {"file_id": "file_xxx", "expires_in": 3600}
```

**Get link details**
```
GET /admin/links/:link_id
```

**Delete link**
```
DELETE /admin/links/:link_id
```

## Development

Run migrations and start the service:

```bash
cargo install diesel_cli --no-default-features --features postgres
diesel migration run
cargo run
```

Run tests:

```bash
cargo test
```

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) file for details.
