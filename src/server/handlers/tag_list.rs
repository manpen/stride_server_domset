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
        LEFT JOIN InstanceTag it ON it.tag_tid=t.tid
        GROUP BY t.tid
        ORDER BY num_instances DESC"#
    )
    .fetch_all(data.db())
    .await
    .map_err(sql_to_err_response)?;

    Ok(Json(tags))
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::server::app_state::DbPool;
    use axum::{body::Body, http::Request};
    use tracing_test::traced_test;

    use super::super::common::test::unwrap_oneshot_request;

    #[sqlx::test(fixtures("instances", "tags"))]
    #[traced_test]
    async fn test_non_empty_tag_list(db_pool: DbPool) -> sqlx::Result<()> {
        let body = unwrap_oneshot_request(
            db_pool,
            Request::builder()
                .uri("/api/tags")
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        let tags: Vec<TagModel> = serde_json::from_slice(body.as_bytes()).unwrap();

        assert_eq!(body.len(), 3, "{:?}", body);
        assert_eq!(
            tags.iter().filter(|m| m.num_instances > 0).count(),
            2,
            "{:?}",
            body
        );

        assert!(tags.iter().all(|m| m.name.starts_with("name")));
        assert!(tags
            .iter()
            .all(|m| m.description.as_ref().unwrap().starts_with("de")));

        Ok(())
    }
}
