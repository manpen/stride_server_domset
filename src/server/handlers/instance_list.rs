use axum::http::{
    header::{CONTENT_DISPOSITION, CONTENT_TYPE},
    HeaderValue,
};
use itertools::Itertools;
use sqlx::{Database, QueryBuilder};

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
    pub sort_by: SortBy,

    #[serde(default)]
    pub sort_direction: SortDirection,

    #[serde(default)]
    pub tag: Option<u32>,

    #[serde(default)]
    pub nodes_lb: Option<u32>,
    #[serde(default)]
    pub nodes_ub: Option<u32>,

    #[serde(default)]
    pub edges_lb: Option<u32>,
    #[serde(default)]
    pub edges_ub: Option<u32>,

    #[serde(default)]
    pub score_lb: Option<u32>,
    #[serde(default)]
    pub score_ub: Option<u32>,

    #[serde(default)]
    pub min_deg_lb: Option<u32>,
    #[serde(default)]
    pub min_deg_ub: Option<u32>,

    #[serde(default)]
    pub max_deg_lb: Option<u32>,
    #[serde(default)]
    pub max_deg_ub: Option<u32>,

    #[serde(default)]
    pub num_ccs_lb: Option<u32>,
    #[serde(default)]
    pub num_ccs_ub: Option<u32>,

    #[serde(default)]
    pub nodes_largest_cc_lb: Option<u32>,
    #[serde(default)]
    pub nodes_largest_cc_ub: Option<u32>,

    #[serde(default)]
    pub diameter_lb: Option<u32>,
    #[serde(default)]
    pub diameter_ub: Option<u32>,

    #[serde(default)]
    pub tree_width_lb: Option<u32>,
    #[serde(default)]
    pub tree_width_ub: Option<u32>,

    #[serde(default)]
    pub planar: Option<bool>,

    #[serde(default)]
    pub regular: Option<bool>,

    #[serde(default)]
    pub include_tag_list: bool,

    #[serde(default)]
    pub include_max_values: bool,
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

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Default)]
struct MaxValues {
    max_nodes: Option<u32>,
    max_edges: Option<u32>,
    max_solution_score: Option<u32>,
}

#[derive(Serialize)]
struct Response {
    status: &'static str,
    options: FilterOptions,

    total_matches: u32,
    max_values: Option<MaxValues>,

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

    min_deg: Option<u32>,
    max_deg: Option<u32>,
    num_ccs: Option<u32>,
    nodes_largest_cc: Option<u32>,
    diameter: Option<u32>,
    tree_width: Option<u32>,
    planar: Option<bool>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    min_deg: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_deg: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_ccs: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nodes_largest_cc: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    diameter: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tree_width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    planar: Option<bool>,
}

fn append_filters_to_query_builder<'a, DB>(
    mut builder: QueryBuilder<'a, DB>,
    opts: &'a FilterOptions,
) -> QueryBuilder<'a, DB>
where
    DB: Database,
    u32: sqlx::Encode<'a, DB> + sqlx::Type<DB>,
    bool: sqlx::Encode<'a, DB> + sqlx::Type<DB>,
{
    macro_rules! append_range_filter {
        ($key:ident) => {
            paste::paste! {
                if let Some(x) = opts.[<$key _lb>] {
                    builder.push(" AND i.[<$key>] >= ");
                    builder.push_bind(x);
                }

                if let Some(x) = opts.[<$key _ub>] {
                    builder.push(" AND i.[<$key>] <= ");
                    builder.push_bind(x);
                }
            }
        };
    }

    append_range_filter!(nodes);
    append_range_filter!(edges);

    append_range_filter!(min_deg);
    append_range_filter!(max_deg);
    append_range_filter!(num_ccs);
    append_range_filter!(nodes_largest_cc);
    append_range_filter!(diameter);
    append_range_filter!(tree_width);

    if let Some(x) = opts.planar {
        builder.push(" AND i.planar = ");
        builder.push_bind(x);
    }

    if let Some(x) = opts.regular {
        if x {
            builder.push(" AND i.min_deg = i.max_deg ");
        } else {
            builder.push(" AND i.min_deg != i.max_deg ");
        }
    }

    builder
}

async fn count_matches(opts: &FilterOptions, app_data: &Arc<AppState>) -> HandlerResult<u32> {
    let mut builder = sqlx::QueryBuilder::new(r#"SELECT COUNT(*) as cnt FROM `Instance` i "#);

    if let Some(tid) = opts.tag {
        builder.push(" JOIN InstanceTag it ON i.iid = it.instance_iid WHERE it.tag_tid = ");
        builder.push_bind(tid);
    } else {
        builder.push(" WHERE 1=1 ");
    }

    builder = append_filters_to_query_builder(builder, opts);

    Ok(builder
        .build_query_scalar::<i64>()
        .fetch_one(app_data.db())
        .await? as u32)
}

async fn retrieve_instances(
    opts: &FilterOptions,
    app_data: &Arc<AppState>,
) -> HandlerResult<Vec<InstanceModel>> {
    let mut builder = sqlx::QueryBuilder::new(
        r#"SELECT 
        i.iid, i.nodes, i.edges, i.name, i.description,
        i.min_deg, i.max_deg, i.num_ccs, i.nodes_largest_cc, i.diameter, i.tree_width, i.planar,
    (SELECT MIN(score) FROM Solution WHERE instance_iid=i.iid) as best_known_solution, 
    GROUP_CONCAT(tag_tid) as tags
FROM `Instance` i
JOIN InstanceTag it ON i.iid = it.instance_iid
WHERE "#,
    );

    if let Some(tid) = opts.tag {
        builder.push("it.tag_tid = ");
        builder.push_bind(tid);
    } else {
        builder.push("1=1 ");
    }

    builder = append_filters_to_query_builder(builder, opts);

    builder.push(" GROUP BY i.iid ORDER BY ");
    builder.push(match opts.sort_by {
        SortBy::Id => "iid",
        SortBy::Name => "name",
        SortBy::Nodes => "nodes",
        SortBy::Edges => "edges",
        SortBy::CreatedAt => "i.created_at",
        SortBy::Score => "best_known_solution",
        SortBy::Difficulty => "best_known_solution",
    });

    builder.push(match opts.sort_direction {
        SortDirection::Desc => " DESC ",
        SortDirection::Asc => " ASC ",
    });

    let limit = opts.limit as u32;
    let offset = (opts.page.saturating_sub(1) * opts.limit) as u32;

    builder.push("LIMIT ");
    builder.push_bind(limit);

    builder.push(" OFFSET ");
    builder.push_bind(offset);

    Ok(builder
        .build_query_as::<InstanceModel>()
        .fetch_all(app_data.db())
        .await?)
}

async fn compute_max_values(app_data: &Arc<AppState>) -> HandlerResult<MaxValues> {
    struct MaxGraphSize {
        max_nodes: Option<u32>,
        max_edges: Option<u32>,
    }

    let max_values = sqlx::query_as!(
        MaxGraphSize,
        r#"SELECT MAX(nodes) as max_nodes, MAX(edges) as max_edges FROM `Instance` i"#,
    )
    .fetch_one(app_data.db())
    .await?;

    let max_solution_score = sqlx::query_scalar::<_, u32>(
            r#"SELECT MIN(score) as cnt FROM Solution s GROUP BY s.instance_iid ORDER BY cnt DESC LIMIT 1"#,
        )
        .fetch_one(app_data.db())
        .await;

    Ok(MaxValues {
        max_nodes: max_values.max_nodes,
        max_edges: max_values.max_edges,
        max_solution_score: max_solution_score.ok(),
    })
}

pub async fn instance_list_handler(
    opts: Option<Query<FilterOptions>>,
    State(app_data): State<Arc<AppState>>,
) -> HandlerResult<impl IntoResponse> {
    let Query(opts) = opts.unwrap_or_default();

    let max_values = if opts.include_max_values {
        Some(compute_max_values(&app_data).await?)
    } else {
        None
    };

    let total_matches = count_matches(&opts, &app_data).await?;
    let results: Vec<InstanceResult> = retrieve_instances(&opts, &app_data)
        .await?
        .into_iter()
        .map(|model: InstanceModel| {
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
                min_deg: model.min_deg,
                max_deg: model.max_deg,
                num_ccs: model.num_ccs,
                nodes_largest_cc: model.nodes_largest_cc,
                diameter: model.diameter,
                tree_width: model.tree_width,
                planar: model.planar,
                tags,
            }
        })
        .collect();

    let tags = if opts.include_tag_list {
        Some(get_tag_list(State(app_data.clone())).await?)
    } else {
        None
    };

    let json_response = Response {
        status: "ok",
        options: opts,
        total_matches,
        max_values,
        results,
        tags,
    };

    Ok(Json(json_response))
}

pub async fn instance_list_download_handler(
    opts: Option<Query<FilterOptions>>,
    State(app_data): State<Arc<AppState>>,
) -> HandlerResult<impl IntoResponse> {
    let Query(opts) = opts.unwrap_or_default();

    let list_as_string = {
        let mut builder = sqlx::QueryBuilder::new(r#"SELECT i.iid FROM `Instance` i "#);

        if let Some(tid) = opts.tag {
            builder.push(" JOIN InstanceTag it ON i.iid = it.instance_iid WHERE it.tag_tid = ");
            builder.push_bind(tid);
        } else {
            builder.push(" WHERE 1=1 ");
        }

        builder = append_filters_to_query_builder(builder, &opts);

        builder.push(" ORDER BY ");
        builder.push(match opts.sort_by {
            SortBy::Id => "iid",
            SortBy::Name => "name",
            SortBy::Nodes => "nodes",
            SortBy::Edges => "edges",
            SortBy::CreatedAt => "i.created_at",
            SortBy::Score => "best_known_solution",
            SortBy::Difficulty => "best_known_solution",
        });

        builder.push(match opts.sort_direction {
            SortDirection::Desc => " DESC ",
            SortDirection::Asc => " ASC ",
        });

        builder
            .build_query_scalar::<i32>()
            .fetch_all(app_data.db())
            .await?
            .into_iter()
            .map(|x| x.to_string())
            .join("\n")
    };

    let document = format!("% {}\n{list_as_string}", serde_json::to_string(&opts)?);

    let content_disposition = HeaderValue::from_str("attachment; filename=\"list.txt\"")?;

    Ok((
        [
            (CONTENT_DISPOSITION, content_disposition),
            (CONTENT_TYPE, HeaderValue::from_static("text/plain")),
        ],
        document,
    ))
}
