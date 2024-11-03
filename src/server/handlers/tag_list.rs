use super::common::*;

#[derive(Debug, Deserialize, Serialize, sqlx::FromRow)]
#[allow(non_snake_case)]
struct TagModel {
    tid: i32,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

pub async fn tag_list_handler(
    State(data): State<Arc<AppState>>,
) -> HandlerResult<impl IntoResponse> {
    let tags = sqlx::query_as!(TagModel, r#"SELECT tid, name, description FROM Tag"#)
        .fetch_all(data.db())
        .await
        .map_err(sql_to_err_response)?;

    Ok(Json(tags))
}
