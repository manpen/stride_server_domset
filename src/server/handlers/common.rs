pub use crate::server::app_state::AppState;

pub use axum::extract::{Query, State};
pub use axum::http::StatusCode;
pub use axum::{response::IntoResponse, Json};
pub use serde::{Deserialize, Serialize};
pub use std::sync::Arc;

pub(super) fn debug_to_err_response<T: std::fmt::Debug>(
    err: T,
) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({"status": "error", "message": format!("{err:?}")})),
    )
}

pub(super) fn sql_to_err_response(err: sqlx::Error) -> (StatusCode, Json<serde_json::Value>) {
    match err {
        sqlx::Error::RowNotFound => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"status": "error", "message": "Entry not found"})),
        ),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"status": "error", "message": err.to_string()})),
        ),
    }
}

pub type HandlerErr = (StatusCode, Json<serde_json::Value>);
pub type HandlerResult<T> = Result<T, HandlerErr>;
