use std::sync::Arc;

use dotenv::dotenv;
use sqlx::{mysql::MySqlPoolOptions, MySqlPool};

use stride_server::server::{app_state::AppState, router::create_router};

use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

async fn connect_to_database() -> MySqlPool {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = MySqlPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await;

    if let Err(err) = pool {
        error!("Failed to connect to the database: {:?}", err);
        std::process::exit(1);
    }

    info!("Connection to the database is successful!");
    pool.unwrap()
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "stride_server=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app_state = AppState::new(connect_to_database().await);

    let app = create_router(Arc::new(app_state));

    info!("Pace Server started successfully");

    let bind_address = "0.0.0.0:8000";
    let listener = match tokio::net::TcpListener::bind(bind_address).await {
        Ok(listener) => {
            info!("Start listening on {bind_address}");
            listener
        }
        Err(err) => {
            error!("Failed to listen to {bind_address}: {:?}", err);
            std::process::exit(1);
        }
    };

    if let Err(err) = axum::serve(listener, app).await {
        error!("Failed to serve {err:?}");
    }
}
