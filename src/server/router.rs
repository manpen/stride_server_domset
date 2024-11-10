use super::{app_state::AppState, handlers::*};
use axum::{
    handler::HandlerWithoutStateExt,
    http::StatusCode,
    routing::{get, post},
    Router,
};
use std::sync::Arc;

use tower_http::services::ServeDir;

async fn handle_404() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "Not found")
}

pub fn create_router(app_state: Arc<AppState>) -> Router {
    // you can convert handler function to service
    let service_404 = handle_404.into_service();

    Router::new()
        .route("/api/status", get(status_handler))
        .route("/api/instances/new", post(instance_upload_handler))
        .route("/api/instances", get(instance_list_handler))
        .route(
            "/api/instances/fetch_unsolved",
            get(instance_fetch_unsolved_handler),
        )
        .route(
            "/api/instances/download/:id",
            get(instance_download_handler),
        )
        .route("/api/tags/new", post(tag_create_handler))
        .route("/api/tags", get(tag_list_handler))
        .route("/api/solutions/new", post(solution_upload_handler))
        .route(
            "/api/solution_hashes/:solver_uuid",
            get(solution_hash_list_handler),
        )
        // serve static files
        .fallback_service(ServeDir::new("assets").not_found_service(service_404))
        .with_state(app_state)
}
