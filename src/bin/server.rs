use std::sync::Arc;

use axum::http::{
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
    HeaderValue, Method,
};
use dotenv::dotenv;
use sqlx::{mysql::MySqlPoolOptions, MySqlPool};
use tower_http::cors::CorsLayer;

use pace_server::server::{app_state::AppState, router::create_router};

use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

async fn connect_to_database() -> MySqlPool {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = MySqlPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await;

    if let Err(err) = pool {
        error!("🔥 Failed to connect to the database: {:?}", err);
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
                .unwrap_or_else(|_| "pace_server=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app_state = AppState::new(connect_to_database().await);

    let app = create_router(Arc::new(app_state));

    let app = {
        let cors = CorsLayer::new()
            .allow_origin("http://localhost:3000".parse::<HeaderValue>().unwrap())
            .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
            .allow_credentials(true)
            .allow_headers([AUTHORIZATION, ACCEPT, CONTENT_TYPE]);
        app.layer(cors)
    };

    info!("Pace Server started successfully");

    let bind_address = "0.0.0.0:8000";
    let listener = tokio::net::TcpListener::bind(bind_address).await.unwrap();
    info!("Start listening on {bind_address}");

    axum::serve(listener, app).await.unwrap()
}
