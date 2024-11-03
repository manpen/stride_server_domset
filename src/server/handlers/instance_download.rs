use axum::extract::Path;
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::http::HeaderValue;
use axum::response::IntoResponse;

use super::common::*;

#[derive(Debug, Deserialize, Serialize, sqlx::FromRow)]
#[allow(non_snake_case)]
struct InstanceModel {
    iid: i32,
    nodes: u32,
    edges: u32,
    name: Option<String>,
    description: Option<String>,
    submitted_by: Option<String>,
    data_hash: Option<String>,
    data: Option<Vec<u8>>,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
struct InstanceResponseHeader {
    iid: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    submitted_by: Option<String>,
}

pub async fn instance_download_handler(
    Path(id): Path<u32>,
    State(data): State<Arc<AppState>>,
) -> HandlerResult<impl IntoResponse> {
    // attempt to fetch instance from database
    let instance = sqlx::query_as!(
        InstanceModel,
        r#"SELECT i.*, d.data FROM `Instance` i JOIN `InstanceData` d ON i.data_hash = d.hash WHERE i.iid = ? LIMIT 1"#,
        id as i32,
    )
    .fetch_one(data.db())
    .await
    .map_err(sql_to_err_response)?;

    if instance.data.is_none() {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "status": "error",
                "message": "Instance data is missing"
            })),
        ));
    }

    // decode it into utf-8
    let data = match std::str::from_utf8(instance.data.as_ref().unwrap()) {
        Ok(data) => data,
        Err(_) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "status": "error",
                    "message": "Instance data is not valid UTF-8"
                })),
            ));
        }
    };

    let mut document: String = String::new();

    // produce header
    {
        let header = InstanceResponseHeader {
            iid: instance.iid,
            name: instance.name.clone(),
            description: instance.description.clone(),
            submitted_by: instance.submitted_by.clone(),
        };

        document.push_str("c ");
        document.push_str(&serde_json::to_string(&header).map_err(debug_to_err_response)?);
        document.push('\n');
    }

    // deliver response
    document.push_str(data);

    let content_disposition = HeaderValue::from_str(&format!("attachment; filename=\"{}.gr\"", id))
        .map_err(debug_to_err_response)?;

    Ok((
        [
            (CONTENT_DISPOSITION, content_disposition),
            (CONTENT_TYPE, HeaderValue::from_static("text/plain")),
        ],
        document,
    ))
}
