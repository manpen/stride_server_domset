use sqlx_conditional_queries::conditional_query_as;

use crate::{
    pace::graph::NumNodes,
    server::handlers::tag_list::{get_tag_list, TagModel},
};

use super::common::*;

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct FilterOptions {
    #[serde(default = "default_value_1")]
    pub page: usize,

    #[serde(default = "default_value_100")]
    pub limit: usize,

    #[serde(default)]
    pub tag: Option<u32>,

    #[serde(default)]
    pub sort_by: SortBy,

    #[serde(default)]
    pub sort_direction: SortDirection,

    #[serde(default)]
    pub include_tag_list: bool,
}

fn default_value_1() -> usize {
    1
}
fn default_value_100() -> usize {
    100
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum SortBy {
    #[default]
    Id,
    Name,
    Nodes,
    Edges,
    CreatedAt,
    Score,
    Difficulty,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SortDirection {
    #[default]
    Asc,
    Desc,
}

#[derive(Serialize)]
struct Response {
    status: &'static str,
    options: FilterOptions,
    total_matches: Option<i64>,
    results: Vec<InstanceResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tags: Option<Vec<TagModel>>,
}

#[derive(Debug, Deserialize, Serialize, sqlx::FromRow)]
#[allow(non_snake_case)]
struct InstanceModel {
    iid: i32,
    nodes: u32,
    edges: u32,
    name: Option<String>,
    description: Option<String>,
    best_known_solution: Option<u32>,
    tags: Option<String>,
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
    best_known_solution: Option<NumNodes>,
    tags: Vec<u32>,
}

pub async fn instance_list_handler(
    opts: Option<Query<FilterOptions>>,
    State(data): State<Arc<AppState>>,
) -> HandlerResult<impl IntoResponse> {
    let Query(opts) = opts.unwrap_or_default();

    let limit = opts.limit as u32;
    let offset = (opts.page.saturating_sub(1) * opts.limit) as u32;

    struct CountRecord {
        cnt: Option<i64>,
    }

    let total_matches = conditional_query_as!(CountRecord,
        r#"SELECT COUNT(*) as cnt FROM `Instance` i {#tag}"#,
        #tag = match opts.tag {
            Some(tid) => "JOIN InstanceTag it ON i.iid = it.instance_iid WHERE it.tag_tid = {tid}",
            None => ""
        }
    )
    .fetch_one(data.db())
    .await
    .map_err(sql_to_err_response)?
    .cnt;

    let instances = conditional_query_as!(
        InstanceModel,
        r#"SELECT
            i.iid, i.nodes, i.edges, i.name, i.description,
            (SELECT MIN(score) FROM Solution WHERE instance_iid=i.iid) as best_known_solution, 
            GROUP_CONCAT(tag_tid) as tags
           FROM `Instance` i
           JOIN InstanceTag it ON i.iid = it.instance_iid
           {#tag_filter} 
           GROUP BY i.iid
           ORDER BY {#order_field} {#order_dir}
           LIMIT {limit} OFFSET {offset}"#,
           #tag_filter = match opts.tag {
               Some(x) => " WHERE it.tag_tid = {x}",
               None => ""
           },
           #order_field = match opts.sort_by {
               SortBy::Id => "iid",
               SortBy::Name => "name",
               SortBy::Nodes => "nodes",
               SortBy::Edges => "edges",
               SortBy::CreatedAt => "i.created_at",
               SortBy::Score => "best_known_solution",
               SortBy::Difficulty => "best_known_solution",
           },
           #order_dir = match opts.sort_direction {
               SortDirection::Desc => "DESC",
               SortDirection::Asc => "ASC",
           }
    )
    .fetch_all(data.db())
    .await
    .map_err(sql_to_err_response)?;

    let results: Vec<InstanceResult> = instances
        .iter()
        .map(|model: &InstanceModel| {
            let tags = model.tags.as_ref().map_or(Vec::new(), |t| {
                t.split(',')
                    .filter_map(|s| s.parse::<u32>().ok())
                    .collect::<Vec<_>>()
            });

            InstanceResult {
                iid: model.iid,
                nodes: model.nodes,
                edges: model.edges,
                name: model.name.to_owned(),
                description: model.description.to_owned(),
                best_known_solution: model.best_known_solution,
                tags,
            }
        })
        .collect();

    let tags = if opts.include_tag_list {
        Some(get_tag_list(State(data.clone())).await?)
    } else {
        None
    };

    let json_response = Response {
        status: "ok",
        options: opts,
        total_matches,
        results,
        tags,
    };

    Ok(Json(json_response))
}
