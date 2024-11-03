use super::{app_state::AppState, handlers::*};
use axum::{routing::get, routing::post, Router};
use std::sync::Arc;

pub fn create_router(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/status", get(status_handler))
        .route("/api/instances/new", post(instance_upload_handler))
        .route("/api/instances", get(instance_list_handler))
        .route(
            "/api/instances/download/:id",
            get(instance_download_handler),
        )
        .route("/api/tags/new", post(tag_create_handler))
        .route("/api/tags", get(tag_list_handler))
        .route("/api/solutions/new", post(solution_upload_handler))
        //.route(
        //    "/api/notes/:id",
        //    get(get_note_handler)
        //        .patch(edit_note_handler)
        //        .delete(delete_note_handler),
        //)
        .with_state(app_state)
}
