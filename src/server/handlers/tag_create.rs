use super::common::*;

#[derive(Debug, Deserialize, Serialize)]
pub struct TagCreateRequest {
    pub name: String,
    pub description: Option<String>,
}

pub async fn tag_create_handler(
    State(data): State<Arc<AppState>>,
    Json(body): Json<TagCreateRequest>,
) -> HandlerResult<impl IntoResponse> {
    let name = body.name.trim();
    let description = body.description.as_ref().map(|s| s.trim());

    if name.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"status": "error", "message": "name is required"})),
        ));
    }

    if name.chars().next().unwrap().is_numeric() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(
                serde_json::json!({"status": "error", "message": "name cannot start with a number"}),
            ),
        ));
    }

    let tag_id = sqlx::query(r#"INSERT INTO Tag (name,description) VALUES (?, ?)"#)
        .bind(name.to_owned())
        .bind(description.to_owned())
        .execute(data.db())
        .await
        .map_err(sql_to_err_response)?
        .last_insert_id();

    Ok(Json(
        serde_json::json!({"status": "success", "tag_id": tag_id}),
    ))
}
