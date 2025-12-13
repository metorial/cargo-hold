use crate::app_state::AppState;
use crate::config::Config;
use crate::db::{create_pool, run_migrations, DbPool};
use crate::snowflake::SnowflakeGeneratorWrapper;
use crate::storage::ObjectStorageClient;

pub fn create_test_config() -> Config {
    Config {
        database_url: std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://localhost/cargo_hold_test".to_string()),
        public_host: "127.0.0.1".to_string(),
        public_port: 0,
        private_host: "127.0.0.1".to_string(),
        private_port: 0,
        storage_base_url: "http://localhost:9999".to_string(),
        storage_bucket: "test-bucket".to_string(),
        max_file_size_bytes: 1048576,
        allowed_purposes: vec![
            "test-purpose".to_string(),
            "document".to_string(),
            "image".to_string(),
        ],
        worker_id: 1,
        datacenter_id: 1,
    }
}

pub fn setup_test_db() -> DbPool {
    let config = create_test_config();
    let pool = create_pool(&config.database_url).expect("Failed to create test pool");
    let mut conn = pool.get().expect("Failed to get connection");

    run_migrations(&mut conn).expect("Failed to run migrations");

    pool
}

pub fn create_test_app_state() -> AppState {
    let config = create_test_config();
    let db_pool = setup_test_db();
    let storage_client = ObjectStorageClient::new(
        config.storage_base_url.clone(),
        config.storage_bucket.clone(),
    );
    let snowflake_gen =
        SnowflakeGeneratorWrapper::new(config.worker_id, config.datacenter_id).unwrap();

    AppState::new(db_pool, storage_client, snowflake_gen, config)
}

pub fn cleanup_test_db(pool: &DbPool) {
    use diesel::prelude::*;

    let mut conn = pool.get().expect("Failed to get connection");

    diesel::sql_query("TRUNCATE TABLE file_links CASCADE")
        .execute(&mut conn)
        .ok();
    diesel::sql_query("TRUNCATE TABLE files CASCADE")
        .execute(&mut conn)
        .ok();
    diesel::sql_query("TRUNCATE TABLE purposes CASCADE")
        .execute(&mut conn)
        .ok();
    diesel::sql_query("TRUNCATE TABLE tenants CASCADE")
        .execute(&mut conn)
        .ok();
}
