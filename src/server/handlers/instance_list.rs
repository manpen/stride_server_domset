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

#[derive(Clone, Deserialize, Serialize, Debug, Default)]
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
    pub best_score_lb: Option<u32>,
    #[serde(default)]
    pub best_score_ub: Option<u32>,

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
    pub treewidth_lb: Option<u32>,
    #[serde(default)]
    pub treewidth_ub: Option<u32>,

    #[serde(default)]
    pub planar: Option<bool>,

    #[serde(default)]
    pub bipartite: Option<bool>,

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
#[cfg_attr(test, derive(strum::EnumIter))]
#[serde(rename_all = "lowercase")]
pub enum SortBy {
    #[default]
    Id,
    Name,
    Nodes,
    Edges,
    CreatedAt,
    #[serde(alias = "best_score")]
    BestScore,
    Difficulty,
    #[serde(alias = "min_deg")]
    MinDeg,
    #[serde(alias = "max_deg")]
    MaxDeg,
    #[serde(alias = "avg_deg")]
    AvgDeg,
    #[serde(alias = "num_ccs")]
    NumCCs,
    #[serde(alias = "nodes_largest_cc")]
    NodesLargestCC,
    Diameter,
    Treewidth,
    Bipartite,
    Planar,
    Regular,
}

impl SortBy {
    fn to_sql_fields(self) -> &'static str {
        match self {
            SortBy::Id => "iid",
            SortBy::Name => "name",
            SortBy::Nodes => "nodes",
            SortBy::Edges => "edges",
            SortBy::CreatedAt => "i.created_at",
            SortBy::BestScore => "best_score",
            SortBy::Difficulty => "best_score", // TODO: this is not what we want
            SortBy::MinDeg => "min_deg",
            SortBy::MaxDeg => "max_deg",
            SortBy::AvgDeg => "edges / nodes",
            SortBy::NumCCs => "num_ccs",
            SortBy::NodesLargestCC => "nodes_largest_cc",
            SortBy::Diameter => "diameter",
            SortBy::Bipartite => "bipartite",
            SortBy::Planar => "planar",
            SortBy::Treewidth => "treewidth",
            SortBy::Regular => "min_deg=max_deg",
        }
    }
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
    nodes: Option<u32>,
    edges: Option<u32>,
    best_score: Option<u32>,

    min_deg: Option<u32>,
    max_deg: Option<u32>,

    num_ccs: Option<u32>,
    nodes_largest_cc: Option<u32>,

    treewidth: Option<u32>,
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
    best_score: Option<u32>,
    tags: Option<String>,

    min_deg: Option<u32>,
    max_deg: Option<u32>,
    num_ccs: Option<u32>,
    nodes_largest_cc: Option<u32>,
    diameter: Option<u32>,
    treewidth: Option<u32>,
    planar: Option<bool>,
    bipartite: Option<bool>,
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
    best_score: Option<NumNodes>,
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
    treewidth: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    planar: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bipartite: Option<bool>,
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
                    builder.push(
                        concat!(" AND i.", stringify!($key), " >= ")
                    );
                    builder.push_bind(x);
                }

                if let Some(x) = opts.[<$key _ub>] {
                    builder.push(
                        concat!(" AND i.", stringify!($key), " <= ")
                    );
                    builder.push_bind(x);
                }
            }
        };
    }

    append_range_filter!(nodes);
    append_range_filter!(edges);
    append_range_filter!(best_score);

    append_range_filter!(min_deg);
    append_range_filter!(max_deg);
    append_range_filter!(num_ccs);
    append_range_filter!(nodes_largest_cc);
    append_range_filter!(diameter);
    append_range_filter!(treewidth);

    if let Some(x) = opts.planar {
        builder.push(" AND i.planar = ");
        builder.push_bind(x);
    }

    if let Some(x) = opts.bipartite {
        builder.push(" AND i.bipartite = ");
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
            i.iid, i.nodes, i.edges, i.name, i.description, i.best_score,
            i.min_deg, i.max_deg, i.num_ccs, i.nodes_largest_cc, i.diameter, i.treewidth, 
            i.planar, i.bipartite,
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
    builder.push(opts.sort_by.to_sql_fields());

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
    Ok(sqlx::query_as!(
        MaxValues,
        r#"SELECT 
            MAX(nodes)     as nodes,
            MAX(edges)     as edges,
            MAX(min_deg)   as min_deg,
            MAX(max_deg)   as max_deg,
            MAX(num_ccs)   as num_ccs,
            MAX(treewidth) as treewidth,
            MAX(best_score)as best_score,
            MAX(nodes_largest_cc) as nodes_largest_cc
        FROM `Instance` i"#,
    )
    .fetch_one(app_data.db())
    .await?)
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
                best_score: model.best_score,
                min_deg: model.min_deg,
                max_deg: model.max_deg,
                num_ccs: model.num_ccs,
                nodes_largest_cc: model.nodes_largest_cc,
                diameter: model.diameter,
                treewidth: model.treewidth,
                planar: model.planar,
                bipartite: model.bipartite,
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
        builder.push(opts.sort_by.to_sql_fields());

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::app_state::DbPool;
    use strum::IntoEnumIterator;

    #[sqlx::test(fixtures("instances"))]
    async fn instance_list_handler_order_by(db_pool: DbPool) -> sqlx::Result<()> {
        for order in SortBy::iter() {
            let req = FilterOptions {
                sort_by: order,
                ..Default::default()
            };

            let state = Arc::new(AppState::new(db_pool.clone()));
            let resp = super::instance_list_handler(Some(Query(req)), State(state)).await;

            assert!(resp.unwrap().into_response().status().is_success());
        }

        Ok(())
    }

    #[sqlx::test(fixtures("instances"))]
    async fn instance_list_download_handler(db_pool: DbPool) -> sqlx::Result<()> {
        for order in SortBy::iter() {
            let req = FilterOptions {
                sort_by: order,
                ..Default::default()
            };

            let state = Arc::new(AppState::new(db_pool.clone()));
            let resp = super::instance_list_handler(Some(Query(req)), State(state)).await;

            assert!(resp.unwrap().into_response().status().is_success());
        }

        Ok(())
    }

    macro_rules! test_filter_option {
        ($name:ident, $values:expr) => {paste::paste! {
            #[sqlx::test(fixtures("instances"))]
            async fn [<filter_option_list_ $name>](db_pool: DbPool) -> sqlx::Result<()> {
                for v in $values {
                    let req = FilterOptions {
                        $name: Some(v),
                        ..Default::default()
                    };

                    let state = Arc::new(AppState::new(db_pool.clone()));
                    let resp = super::instance_list_handler(Some(Query(req)), State(state)).await;
                    assert!(resp.unwrap().into_response().status().is_success());
                }
                Ok(())
            }

            #[sqlx::test(fixtures("instances"))]
            async fn [<filter_option_download_ $name>](db_pool: DbPool) -> sqlx::Result<()> {
                for v in $values {
                    let req = FilterOptions {
                        $name: Some(v),
                        ..Default::default()
                    };

                    let state = Arc::new(AppState::new(db_pool.clone()));
                    let resp = super::instance_list_download_handler(Some(Query(req)), State(state)).await;
                    assert!(resp.unwrap().into_response().status().is_success());
                }
                Ok(())
            }
        }};
    }

    test_filter_option!(tag, [0, 1]);
    test_filter_option!(nodes_lb, [0, 1]);
    test_filter_option!(nodes_ub, [0, 1]);
    test_filter_option!(edges_lb, [0, 1]);
    test_filter_option!(edges_ub, [0, 1]);
    test_filter_option!(best_score_lb, [0, 1]);
    test_filter_option!(best_score_ub, [0, 1]);
    test_filter_option!(min_deg_lb, [0, 1]);
    test_filter_option!(min_deg_ub, [0, 1]);
    test_filter_option!(max_deg_lb, [0, 1]);
    test_filter_option!(max_deg_ub, [0, 1]);
    test_filter_option!(num_ccs_lb, [0, 1]);
    test_filter_option!(num_ccs_ub, [0, 1]);
    test_filter_option!(nodes_largest_cc_lb, [0, 1]);
    test_filter_option!(nodes_largest_cc_ub, [0, 1]);
    test_filter_option!(diameter_lb, [0, 1]);
    test_filter_option!(diameter_ub, [0, 1]);
    test_filter_option!(treewidth_lb, [0, 1]);
    test_filter_option!(treewidth_ub, [0, 1]);
    test_filter_option!(planar, [false, true]);
    test_filter_option!(bipartite, [false, true]);
    test_filter_option!(regular, [false, true]);
}
