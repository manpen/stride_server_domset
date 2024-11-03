use crate::pace::graph::NumNodes;

use super::common::*;

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct FilterOptions {
    pub page: Option<usize>,
    pub limit: Option<usize>,
    pub tag: Option<String>,
}

impl FilterOptions {
    fn defaults_for_missing(self) -> Self {
        Self {
            page: Some(self.page.unwrap_or(1)),
            limit: Some(self.limit.unwrap_or(100)),
            tag: self.tag,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, sqlx::FromRow)]
#[allow(non_snake_case)]
struct InstanceModel {
    iid: i32,
    data_hash : Option<String>,
    nodes: u32,
    edges: u32,
    name: Option<String>,
    description: Option<String>,
    submitted_by: Option<String>,
    best_known_solution: Option<u32>,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize, Serialize)]
struct InstanceResult {
    iid: i32,
    nodes: u32,
    edges: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    submitted_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    best_known_solution: Option<NumNodes>,
}

pub async fn instance_list_handler(
    opts: Option<Query<FilterOptions>>,
    State(data): State<Arc<AppState>>,
) -> HandlerResult<impl IntoResponse> {
    let Query(opts) = opts.unwrap_or_default();
    let opts = opts.defaults_for_missing();

    let limit = opts.limit.unwrap();
    let offset = opts.page.unwrap().saturating_sub(1) * limit;

    // TODO: add join for best_known_solution
    // TODO: add tag filtering
    let instances = if let Some(tag) = opts.tag.as_ref() {
        sqlx::query_as!(
                InstanceModel,
                r#"SELECT i.*, (NULL) as "best_known_solution: u32" FROM `Instance` i JOIN InstanceTag it ON i.iid = it.instance_iid JOIN Tag t ON t.tid = it.tag_tid WHERE t.name = ? ORDER by created_at LIMIT ? OFFSET ?"#,
                tag,
                limit as i32,
                offset as i32
                )    
                .fetch_all(data.db())
                .await
                .map_err(sql_to_err_response)?
    } else {
        sqlx::query_as!(
                InstanceModel,
            r#"SELECT i.*, (NULL) as "best_known_solution: u32" FROM `Instance` i ORDER by created_at LIMIT ? OFFSET ?"#,
            limit as i32,
            offset as i32
            )    
            .fetch_all(data.db())
            .await
            .map_err(sql_to_err_response)?
    };

    let results = instances
        .iter()
        .map(|model: &InstanceModel| InstanceResult {
            iid: model.iid,
            nodes: model.nodes,
            edges: model.edges,
            name: model.name.to_owned(),
            description: model.description.to_owned(),
            submitted_by: model.submitted_by.to_owned(),
            best_known_solution: model.best_known_solution,
        })
        .collect::<Vec<_>>();

    let json_response = serde_json::json!({
        "status": "success",
        "options": opts,
        "results": results
    });

    Ok(Json(json_response))
}
