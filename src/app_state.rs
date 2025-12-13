use crate::config::Config;
use crate::db::DbPool;
use crate::snowflake::SnowflakeGeneratorWrapper;
use crate::storage::ObjectStorageClient;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: DbPool,
    pub storage_client: ObjectStorageClient,
    pub snowflake_gen: Arc<SnowflakeGeneratorWrapper>,
    pub config: Config,
}

impl AppState {
    pub fn new(
        db_pool: DbPool,
        storage_client: ObjectStorageClient,
        snowflake_gen: SnowflakeGeneratorWrapper,
        config: Config,
    ) -> Self {
        Self {
            db_pool,
            storage_client,
            snowflake_gen: Arc::new(snowflake_gen),
            config,
        }
    }
}
