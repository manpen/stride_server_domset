use super::common::*;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TagCreateRequest {
    pub name: String,
    pub description: Option<String>,
    pub style: u32,
}

pub async fn tag_create_handler(
    State(data): State<Arc<AppState>>,
    Json(body): Json<TagCreateRequest>,
) -> HandlerResult<impl IntoResponse> {
    let name = body.name.trim();
    let description = body.description.as_ref().map(|s| s.trim());

    if name.is_empty() {
        return bad_request_json!("Tag name is required");
    }

    if name.chars().next().unwrap().is_numeric() {
        return bad_request_json!("Tag name cannot start with a number");
    }

    let tag_id = sqlx::query(r#"INSERT INTO Tag (name,description,style) VALUES (?, ?, ?)"#)
        .bind(name.to_owned())
        .bind(description.to_owned())
        .bind(body.style)
        .execute(data.db())
        .await
        .map_err(sql_to_err_response)?
        .last_insert_id();

    Ok(Json(
        serde_json::json!({"status": "success", "tag_id": tag_id}),
    ))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::server::app_state::DbPool;

    #[sqlx::test]
    async fn success(pool: DbPool) -> sqlx::Result<()> {
        let state = Arc::new(AppState::new(pool));

        let req_full = TagCreateRequest {
            name: String::from("Hi"),
            description: Some(String::from("Desc")),
            style: 42,
        };

        let req_partial = TagCreateRequest {
            name: String::from("Low"),
            description: None,
            style: 2,
        };

        {
            let response = super::tag_create_handler(State(state.clone()), Json(req_full.clone()))
                .await
                .unwrap()
                .into_response();

            assert!(response.status().is_success());
        }

        // repeat request --- this give a duplication error
        if false {
            let response = super::tag_create_handler(State(state.clone()), Json(req_full))
                .await
                .unwrap()
                .into_response();

            assert!(!response.status().is_success());
        }

        {
            let response =
                super::tag_create_handler(State(state.clone()), Json(req_partial.clone()))
                    .await
                    .unwrap()
                    .into_response();

            assert!(response.status().is_success());
        }

        Ok(())
    }
}
