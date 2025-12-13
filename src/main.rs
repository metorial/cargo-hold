mod app_state;
mod config;
mod db;
mod handlers_private;
mod handlers_public;
mod handlers_unauthenticated;
mod models;
mod schema;
mod snowflake;
mod startup;
mod storage;

#[cfg(test)]
mod test_utils;

use app_state::AppState;
use axum::{
    http::Method,
    routing::{delete, get, post, put},
    Router,
};
use config::Config;
use snowflake::SnowflakeGeneratorWrapper;
use storage::ObjectStorageClient;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "cargo_hold=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env()?;

    let db_pool = db::create_pool(&config.database_url)?;

    let mut conn = db_pool.get()?;
    db::run_migrations(&mut conn)?;
    tracing::info!("Database migrations completed");

    let snowflake_gen = SnowflakeGeneratorWrapper::new(config.worker_id, config.datacenter_id)?;

    startup::upsert_purposes(&mut conn, &snowflake_gen, &config.allowed_purposes)?;
    tracing::info!("Purposes upserted");

    let storage_client = ObjectStorageClient::new(
        config.storage_base_url.clone(),
        config.storage_bucket.clone(),
    );

    let state = AppState::new(db_pool, storage_client, snowflake_gen, config.clone());

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers(Any)
        .allow_origin(Any);

    let public_app = Router::new()
        .route("/files", post(handlers_public::upload_file))
        .route("/files/:file_id", get(handlers_public::get_file))
        .route(
            "/files/:file_id/content",
            get(handlers_public::get_file_content),
        )
        .route(
            "/f/:link_key",
            get(handlers_unauthenticated::get_file_by_link),
        )
        .layer(cors.clone())
        .with_state(state.clone());

    let private_app = Router::new()
        .route("/files/:file_id", delete(handlers_private::delete_file))
        .route("/files/:file_id", put(handlers_private::update_file))
        .route("/files/:file_id", get(handlers_private::get_file_private))
        .route("/files", get(handlers_private::list_files))
        .route("/links", post(handlers_private::create_link))
        .route("/links/:link_id", get(handlers_private::get_link))
        .route("/links/:link_id", delete(handlers_private::delete_link))
        .layer(cors)
        .with_state(state);

    let public_addr = format!("{}:{}", config.public_host, config.public_port);
    let private_addr = format!("{}:{}", config.private_host, config.private_port);

    tracing::info!("Starting public API on {}", public_addr);
    tracing::info!("Starting private API on {}", private_addr);

    let public_listener = tokio::net::TcpListener::bind(&public_addr).await?;
    let private_listener = tokio::net::TcpListener::bind(&private_addr).await?;

    let public_serve = tokio::spawn(async move { axum::serve(public_listener, public_app).await });

    let private_serve =
        tokio::spawn(async move { axum::serve(private_listener, private_app).await });

    tokio::select! {
        result = public_serve => {
            result??;
        }
        result = private_serve => {
            result??;
        }
    }

    Ok(())
}
