use super::common::*;

pub async fn debug_restart_handler() -> impl IntoResponse {
    std::process::exit(0);
    
    #[allow(unreachable_code)]
    Json(serde_json::json!({"status": "success"}))
}

 
