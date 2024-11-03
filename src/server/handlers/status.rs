use super::common::*;
use crate::pace::PROBLEM_ID;

pub async fn status_handler() -> impl IntoResponse {
    let json_response = serde_json::json!({
        "status": "success",
        "server": "Pace Instance Server",
        "problem": PROBLEM_ID,
    });

    Json(json_response)
}
