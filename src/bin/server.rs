use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use axum::extract::Host;
use axum::handler::HandlerWithoutStateExt;
use axum::{
    http::{StatusCode, Uri},
    response::Redirect,
};

use dotenv::dotenv;
use sqlx::{mysql::MySqlPoolOptions, MySqlPool};

use stride_server::server::{app_state::AppState, router::create_router};

use structopt::StructOpt;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use tower_http::compression::CompressionLayer;

use axum_server::tls_rustls::RustlsConfig;

const BIND_ADDRESS: [u8; 4] = [0, 0, 0, 0];

async fn connect_to_database(opts: &Opts) -> MySqlPool {
    let pool = MySqlPoolOptions::new()
        .min_connections(opts.mysql_min_connections)
        .max_connections(opts.mysql_max_connections)
        .connect(opts.mysql_url.as_ref().expect("mysql_url must be set"))
        .await;

    if let Err(err) = pool {
        error!("Failed to connect to the database: {:?}", err);
        std::process::exit(1);
    }

    info!("Connection to the database is successful!");
    pool.unwrap()
}

#[allow(dead_code)]
async fn redirect_http_to_https(http_port: u16, https_port: u16) {
    fn make_https(host: String, uri: Uri, http_port: u16, https_port: u16) -> anyhow::Result<Uri> {
        let mut parts = uri.into_parts();

        parts.scheme = Some(axum::http::uri::Scheme::HTTPS);

        if parts.path_and_query.is_none() {
            parts.path_and_query = Some("/".parse().unwrap());
        }

        let https_host = host.replace(&http_port.to_string(), &https_port.to_string());
        parts.authority = Some(https_host.parse()?);

        Ok(Uri::from_parts(parts)?)
    }

    let redirect = move |Host(host): Host, uri: Uri| async move {
        match make_https(host, uri, http_port, https_port) {
            Ok(uri) => Ok(Redirect::permanent(&uri.to_string())),
            Err(error) => {
                tracing::warn!(%error, "failed to convert URI to HTTPS");
                Err(StatusCode::BAD_REQUEST)
            }
        }
    };

    let addr = SocketAddr::from((BIND_ADDRESS, http_port));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, redirect.into_make_service())
        .await
        .unwrap();
}

async fn http_server(app_state: Arc<AppState>, opts: Arc<Opts>) -> Result<(), anyhow::Error> {
    let app = create_router(app_state).layer(CompressionLayer::new());

    let addr = SocketAddr::from((BIND_ADDRESS, opts.http_port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("Start listening on for HTTP on {addr:?}");
    Ok(axum::serve(listener, app).await?)
}

async fn https_server(app_state: Arc<AppState>, opts: Arc<Opts>) -> Result<(), anyhow::Error> {
    let app = create_router(app_state);

    // configure certificate and private key used by https
    let tls_config = RustlsConfig::from_pem_file(
        PathBuf::from("certs").join("cert.pem"),
        PathBuf::from("certs").join("privkey.pem"),
    )
    .await
    .expect("Loading TLS certificates failed (expected files at certs/{cert,privkey}.pem); will only server on HTTP port.");

    let addr = SocketAddr::from((BIND_ADDRESS, opts.https_port));
    info!("Start listening on for HTTPS on {addr:?}");
    Ok(axum_server::bind_rustls(addr, tls_config)
        .serve(app.into_make_service())
        .await?)
}

#[derive(StructOpt)]
struct Opts {
    #[structopt(short = "-h", long, default_value = "8000")]
    http_port: u16,
    #[structopt(short = "-s", long, default_value = "8080")]
    https_port: u16,

    #[structopt(short = "-r", long)]
    no_redirect_to_https: bool,

    #[structopt(short, long)]
    mysql_url: Option<String>,

    #[structopt(long, default_value = "10")]
    mysql_min_connections: u32,

    #[structopt(long, default_value = "50")]
    mysql_max_connections: u32,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    let opts = Arc::new({
        let mut opts = Opts::from_args();
        if opts.mysql_url.is_none() {
            opts.mysql_url = Some(
                std::env::var("DATABASE_URL")
                    .expect("DATABASE_URL must be set or --mysql-url must be provided"),
            );
        }

        assert!(
            opts.mysql_min_connections <= opts.mysql_max_connections,
            "min_connections must be less than or equal to max_connections"
        );
        opts
    });

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "stride_server=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app_state = Arc::new(AppState::new(connect_to_database(&opts).await));

    let https_handle = tokio::spawn(https_server(app_state.clone(), opts.clone()));
    if opts.no_redirect_to_https {
        info!("Not redirecting HTTP to HTTPS -> Start another server instance on HTTP");
        http_server(app_state, opts.clone()).await?;
    } else {
        info!("Redirecting HTTP to HTTPS");
        redirect_http_to_https(opts.http_port, opts.https_port).await;
    }
    https_handle.await?
}
