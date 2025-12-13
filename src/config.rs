use std::env;

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub public_host: String,
    pub public_port: u16,
    pub private_host: String,
    pub private_port: u16,
    pub storage_base_url: String,
    pub storage_bucket: String,
    pub max_file_size_bytes: i64,
    pub allowed_purposes: Vec<String>,
    pub worker_id: u64,
    pub datacenter_id: u64,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        Ok(Self {
            database_url: env::var("DATABASE_URL")
                .map_err(|_| "DATABASE_URL must be set".to_string())?,
            public_host: env::var("PUBLIC_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            public_port: env::var("PUBLIC_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .map_err(|_| "PUBLIC_PORT must be a valid u16".to_string())?,
            private_host: env::var("PRIVATE_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            private_port: env::var("PRIVATE_PORT")
                .unwrap_or_else(|_| "8081".to_string())
                .parse()
                .map_err(|_| "PRIVATE_PORT must be a valid u16".to_string())?,
            storage_base_url: env::var("STORAGE_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:8082".to_string()),
            storage_bucket: env::var("STORAGE_BUCKET").unwrap_or_else(|_| "cargo-hold".to_string()),
            max_file_size_bytes: env::var("MAX_FILE_SIZE_BYTES")
                .unwrap_or_else(|_| "104857600".to_string())
                .parse()
                .map_err(|_| "MAX_FILE_SIZE_BYTES must be a valid i64".to_string())?,
            allowed_purposes: env::var("ALLOWED_PURPOSES")
                .unwrap_or_else(|_| "user-upload,document,image,avatar".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            worker_id: env::var("WORKER_ID")
                .unwrap_or_else(|_| "1".to_string())
                .parse()
                .map_err(|_| "WORKER_ID must be a valid u64".to_string())?,
            datacenter_id: env::var("DATACENTER_ID")
                .unwrap_or_else(|_| "1".to_string())
                .parse()
                .map_err(|_| "DATACENTER_ID must be a valid u64".to_string())?,
        })
    }
}
