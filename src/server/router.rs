use super::{app_state::AppState, handlers::*};
use axum::{
    extract::DefaultBodyLimit,
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

#[rustfmt::skip]
pub fn create_router(app_state: Arc<AppState>) -> Router {
    // needs to be mutable to allow adding routes based on feature flags
    #[allow(unused_mut)]
    let mut router = Router::new();
    #[cfg(feature = "admin-api")]
    {
        router = router
            .route("/api/instances/new", post(instance_upload_handler))
            .route("/api/instances/update", post(instance_update_meta_handler))
            .route("/api/instances/delete/:id", get(instance_delete_handler))
            .route("/api/tags/new", post(tag_create_handler))
            .route("/api/debug_restart", get(debug_restart_handler));
    }

    router = router
        .route("/api/status", get(status_handler))
        .route("/api/instances", get(instance_list_handler))
        .route("/api/instance_list", get(instance_list_download_handler))
        .route("/api/instances/download/:id", get(instance_download_handler))
        .route("/api/tags", get(tag_list_handler))
        .route("/api/solutions/new", post(solution_upload_handler))
        .route("/api/solution_hashes/:solver_uuid", get(solution_hash_list_handler))
        .route("/api/solver_run/list", get(solver_run_list_handler))
        .route("/api/solver_run/annotate", get(solver_run_annotate_handler));

    let service_404 = handle_404.into_service();
    router
        .layer(DefaultBodyLimit::max(100usize << 20))
        .fallback_service(
            ServeDir::new("assets")
                .precompressed_gzip()
                .precompressed_zstd()
                .not_found_service(service_404))
        .with_state(app_state)
}
