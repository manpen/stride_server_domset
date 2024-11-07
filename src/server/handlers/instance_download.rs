use axum::extract::Path;

use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::http::HeaderValue;
use axum::response::IntoResponse;

use super::common::*;
use crate::server::app_state::DbPool;

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

#[derive(Deserialize, Serialize)]
struct InstanceResponseHeader {
    iid: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    submitted_by: Option<String>,
}

async fn fetch_instance(id: u32, db_pool: &DbPool) -> HandlerResult<(InstanceModel, String)> {
    // attempt to fetch instance from database
    let mut instance = sqlx::query_as!(
        InstanceModel,
        r#"SELECT i.*, d.data FROM `Instance` i JOIN `InstanceData` d ON i.data_hash = d.hash WHERE i.iid = ? LIMIT 1"#,
        id as i32,
    )
    .fetch_one(db_pool)
    .await
    .map_err(sql_to_err_response)?;

    let data = instance.data.take();

    if data.is_none() {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "status": "error",
                "message": "Instance data is missing"
            })),
        ));
    }

    // decode it into utf-8
    let data_string = match String::from_utf8(data.unwrap()) {
        Ok(data) => data,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "status": "error",
                    "message": "Instance data is not valid UTF-8",
                    "error": e.to_string(),
                })),
            ));
        }
    };

    Ok((instance, data_string))
}

fn document_from_instance_and_data(
    instance: &InstanceModel,
    data: String,
) -> HandlerResult<String> {
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
    document.push_str(&data);

    Ok(document)
}

pub async fn instance_download_handler(
    Path(id): Path<u32>,
    State(data): State<Arc<AppState>>,
) -> HandlerResult<impl IntoResponse> {
    let (instance, data) = fetch_instance(id, data.db()).await?;
    let document = document_from_instance_and_data(&instance, data);

    let header_line = format!("attachment; filename=\"{id}.gr\"");
    let content_disposition = HeaderValue::from_str(&header_line).map_err(debug_to_err_response)?;

    Ok((
        [
            (CONTENT_DISPOSITION, content_disposition),
            (CONTENT_TYPE, HeaderValue::from_static("text/plain")),
        ],
        document,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(fixtures("instances"))]
    async fn fetch_instance(pool: DbPool) -> sqlx::Result<()> {
        let (instance, data) = super::fetch_instance(1, &pool).await.unwrap();
        assert_eq!(instance.iid, 1);
        assert_eq!(instance.nodes, 2);
        assert_eq!(instance.edges, 1);
        assert_eq!(instance.submitted_by.unwrap(), "tester");
        assert!(data.starts_with("p ds"));

        let _ = super::fetch_instance(2, &pool).await.unwrap();

        assert!(super::fetch_instance(3, &pool).await.is_err());

        Ok(())
    }

    #[test]
    fn document_from_instance_and_data() {
        let instance = InstanceModel {
            iid: 1,
            nodes: 1,
            edges: 2,
            name: Some(String::from("name")),
            description: None,
            submitted_by: None,
            data_hash: Some(String::from("hash")),
            data: None,
            created_at: Default::default(),
        };
        let data = "HelloWorld";

        let document = super::document_from_instance_and_data(&instance, data.to_string()).unwrap();
        let lines = document.lines().collect::<Vec<&str>>();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("c "));

        let meta: InstanceResponseHeader = serde_json::from_str(&lines[0][2..]).unwrap();

        assert_eq!(meta.iid, instance.iid);
    }

    #[sqlx::test(fixtures("instances"))]
    async fn download_handler(pool: DbPool) -> sqlx::Result<()> {
        let state = State(Arc::new(AppState::new(pool)));
        let response = instance_download_handler(Path(1), state).await.unwrap();

        let (headers, _body) = response.into_response().into_parts();
        assert_eq!(headers.status, StatusCode::OK);
        assert_eq!(headers.headers.get(CONTENT_TYPE).unwrap(), "text/plain");
        assert_eq!(
            headers.headers.get(CONTENT_DISPOSITION).unwrap(),
            "attachment; filename=\"1.gr\""
        );

        Ok(())
    }
}
