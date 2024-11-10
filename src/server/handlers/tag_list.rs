use super::common::*;

#[derive(Debug, Deserialize, Serialize, sqlx::FromRow)]
#[allow(non_snake_case)]
struct TagModel {
    tid: i32,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    style: u32,
    num_instances: i64,
}

pub async fn tag_list_handler(
    State(data): State<Arc<AppState>>,
) -> HandlerResult<impl IntoResponse> {
    let tags = sqlx::query_as!(
        TagModel,
        r#"SELECT 
            t.tid, t.name, t.description, t.style, 
            COUNT(it.instance_iid) as num_instances 
        FROM Tag t
        JOIN InstanceTag it ON it.tag_tid=t.tid
        GROUP BY t.tid
        ORDER BY num_instances DESC"#
    )
    .fetch_all(data.db())
    .await
    .map_err(sql_to_err_response)?;

    Ok(Json(tags))
}
