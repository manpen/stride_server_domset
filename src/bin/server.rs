use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use dotenv::dotenv;
use sqlx::{mysql::MySqlPoolOptions, MySqlPool};

use stride_server::server::{app_state::AppState, router::create_router};

use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use axum_server::tls_rustls::RustlsConfig;

const BIND_ADDRESS: [u8; 4] = [0, 0, 0, 0];
const HTTP_PORT: u16 = 8000;
const HTTPS_PORT: u16 = 8080;

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

async fn http_server(app_state: Arc<AppState>) -> Result<(), anyhow::Error> {
    let app = create_router(app_state);

    let addr = SocketAddr::from((BIND_ADDRESS, HTTP_PORT));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("Start listening on for HTTP on {addr:?}");
    Ok(axum::serve(listener, app).await?)
}

async fn https_server(app_state: Arc<AppState>) -> Result<(), anyhow::Error> {
    let app = create_router(app_state);

    // configure certificate and private key used by https
    let tls_config = RustlsConfig::from_pem_file(
        PathBuf::from("certs").join("cert.pem"),
        PathBuf::from("certs").join("privkey.pem"),
    )
    .await
    .expect("Loading TLS certificates failed (expected files at certs/{cert,privkey}.pem); will only server on HTTP port.");

    let addr = SocketAddr::from((BIND_ADDRESS, HTTPS_PORT));
    info!("Start listening on for HTTPS on {addr:?}");
    Ok(axum_server::bind_rustls(addr, tls_config)
        .serve(app.into_make_service())
        .await?)
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

    let app_state = Arc::new(AppState::new(connect_to_database().await));

    tokio::spawn(https_server(app_state.clone()));
    http_server(app_state).await.unwrap()
}
