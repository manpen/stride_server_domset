use axum::http::{
    header::{CONTENT_DISPOSITION, CONTENT_TYPE},
    HeaderValue,
};
use itertools::Itertools;
use sqlx::{Database, QueryBuilder};
use sqlx_conditional_queries::conditional_query_as;
use uuid::Uuid;

use crate::{
    pace::graph::NumNodes,
    server::handlers::tag_list::{get_tag_list, TagModel},
};

use super::{common::*, solution_upload::SolverResultType};

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

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nodes_lb: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nodes_ub: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edges_lb: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edges_ub: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub best_score_lb: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub best_score_ub: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_deg_lb: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_deg_ub: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_deg_lb: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_deg_ub: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_ccs_lb: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_ccs_ub: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nodes_largest_cc_lb: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nodes_largest_cc_ub: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diameter_lb: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diameter_ub: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub treewidth_lb: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub treewidth_ub: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub planar: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bipartite: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regular: Option<bool>,

    #[serde(default)]
    pub include_tag_list: bool,

    #[serde(default)]
    pub include_max_values: bool,

    // using an own type for solver_filter guarantees that the user has to provide both
    // solver_uuid and run_uuid before having access to the remaining filters
    #[serde(default, skip_serializing_if = "Option::is_none", flatten)]
    pub run_filters: Option<SolverFilter>,
}

impl FilterOptions {
    fn check_validity(&self) -> HandlerResult<()> {
        if self.run_filters.is_none() {
            match self.sort_by {
                SortBy::Score | SortBy::ScoreDiff | SortBy::SecondsComputed | SortBy::ErrorCode => {
                    return Err(anyhow::anyhow!(
                        "solver_filter is required when sorting by score, score_diff, seconds_computed or error_code"
                    )
                    .into());
                }
                _ => {}
            }
        }

        Ok(())
    }
}

#[derive(Clone, Deserialize, Serialize, Debug, Default)]
pub struct SolverFilter {
    solver: Uuid,
    run: Uuid,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_lb: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_ub: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_diff_lb: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_diff_ub: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seconds_computed_lb: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seconds_computed_ub: Option<f64>,

    #[serde(default)]
    pub status: ResultStatusFilter,
}

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ResultStatusFilter {
    #[default]
    None,

    Valid,
    Invalid,

    Optimal,
    Suboptimal,
    Incomplete,
    Timeout,
    Infeasible,
    Error,
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

    Score, // avaliable only when solver_filter is provided
    #[serde(alias = "score_diff")]
    ScoreDiff, // avaliable only when solver_filter is provided
    #[serde(alias = "seconds_computed")]
    SecondsComputed, // avaliable only when solver_filter is provided
    #[serde(alias = "error_code")]
    ErrorCode, // avaliable only when solver_filter is provided
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

            SortBy::Score => "s.score",
            SortBy::ScoreDiff => "s.score - i.best_score",
            SortBy::SecondsComputed => "s.seconds_computed",
            SortBy::ErrorCode => "s.error_code",
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

    #[serde(skip_serializing_if = "Option::is_none")]
    score: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    score_diff: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    seconds_computed: Option<f64>,
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

#[derive(Default, Debug, Deserialize, Serialize, sqlx::FromRow)]
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

    solution_hash: Option<String>,
    error_code: Option<u8>,
    score: Option<u32>,
    seconds_computed: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
struct SolutionResult {
    solution_hash: Option<String>,
    error_code: SolverResultType,
    score: Option<u32>,
    seconds_computed: f64,
}

#[derive(Clone, Debug, Serialize)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    solution: Option<SolutionResult>,
}

fn append_filters_to_query_builder<'a, DB>(
    mut builder: QueryBuilder<'a, DB>,
    opts: &'a FilterOptions,
) -> HandlerResult<QueryBuilder<'a, DB>>
where
    DB: Database,
    u32: sqlx::Encode<'a, DB> + sqlx::Type<DB>,
    f64: sqlx::Encode<'a, DB> + sqlx::Type<DB>,
    bool: sqlx::Encode<'a, DB> + sqlx::Type<DB>,
    String: sqlx::Encode<'a, DB> + sqlx::Type<DB>,
{
    macro_rules! append_range_filter {
        ($opts:expr, $key:ident) => {
            append_range_filter!($opts, $key, concat!("i.", stringify!($key)));
        };

        ($opts:expr, $key:ident, $sql_expr:expr) => {
            paste::paste! {
                if let Some(x) = $opts.[<$key _lb>] {
                    builder.push(
                        concat!(" AND ", $sql_expr, " >= ")
                    );
                    builder.push_bind(x);
                }

                if let Some(x) = $opts.[<$key _ub>] {
                    builder.push(
                        concat!(" AND ", $sql_expr, " <= ")
                    );
                    builder.push_bind(x);
                }
            }
        };
    }

    append_range_filter!(opts, nodes);
    append_range_filter!(opts, edges);
    append_range_filter!(opts, best_score);

    append_range_filter!(opts, min_deg);
    append_range_filter!(opts, max_deg);
    append_range_filter!(opts, num_ccs);
    append_range_filter!(opts, nodes_largest_cc);
    append_range_filter!(opts, diameter);
    append_range_filter!(opts, treewidth);

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

    if let Some(solver_filter) = &opts.run_filters {
        builder.push(" AND s.sr_uuid = UNHEX(");
        builder.push_bind(solver_filter.run.simple().to_string());
        builder.push(")");

        append_range_filter!(solver_filter, score, "s.score");
        append_range_filter!(solver_filter, score_diff, "s.score - i.best_score");
        append_range_filter!(solver_filter, seconds_computed, "s.seconds_computed");

        match solver_filter.status {
            ResultStatusFilter::None => {}
            ResultStatusFilter::Valid => {
                builder.push(" AND s.score IS NOT NULL ");
            }
            ResultStatusFilter::Invalid => {
                builder.push(" AND s.score IS NULL ");
            }
            ResultStatusFilter::Optimal => {
                builder.push(" AND i.best_score = s.score ");
            }
            ResultStatusFilter::Suboptimal => {
                builder.push(" AND i.best_score < s.score ");
            }
            ResultStatusFilter::Incomplete => {
                builder.push(" AND s.error_code = ");
                builder.push_bind(SolverResultType::IncompleteOutput as u32);
            }
            ResultStatusFilter::Timeout => {
                builder.push(" AND s.error_code = ");
                builder.push_bind(SolverResultType::Timeout as u32);
            }
            ResultStatusFilter::Infeasible => {
                builder.push(" AND s.error_code = ");
                builder.push_bind(SolverResultType::Infeasible as u32);
            }
            ResultStatusFilter::Error => {
                builder.push(" AND s.error_code = ");
                builder.push_bind(SolverResultType::SyntaxError as u32);
            }
        }
    }

    Ok(builder)
}

async fn count_matches(opts: &FilterOptions, app_data: &Arc<AppState>) -> HandlerResult<u32> {
    let mut builder = sqlx::QueryBuilder::new(r#"SELECT COUNT(*) as cnt FROM `Instance` i "#);

    if opts.run_filters.is_some() {
        builder.push(" JOIN Solution s ON i.iid = s.instance_iid ");
    }

    if let Some(tid) = opts.tag {
        builder.push(" JOIN InstanceTag it ON i.iid = it.instance_iid WHERE it.tag_tid = ");
        builder.push_bind(tid);
    } else {
        builder.push(" WHERE 1=1 ");
    }

    builder = append_filters_to_query_builder(builder, opts)?;

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
            GROUP_CONCAT(tag_tid) as tags, "#,
    );

    if opts.run_filters.is_some() {
        builder.push(
            r#" 
            HEX(s.solution_hash) as `solution_hash`, s.error_code, s.score, s.seconds_computed
        FROM `Instance` i
        JOIN Solution s ON i.iid = s.instance_iid "#,
        );
    } else {
        builder.push(r#" 
                    NULL as solution_hash, NULL as error_code,  NULL as score,  NULL as seconds_computed
                    FROM `Instance` i "#);
    }

    builder.push(
        r#"
        JOIN InstanceTag it ON i.iid = it.instance_iid
        WHERE "#,
    );

    if let Some(tid) = opts.tag {
        builder.push("it.tag_tid = ");
        builder.push_bind(tid);
    } else {
        builder.push("1=1 ");
    }

    builder = append_filters_to_query_builder(builder, opts)?;

    builder.push(" GROUP BY i.iid ");
    if opts.run_filters.is_some() {
        builder.push(", s.sr_uuid");
    }

    builder.push(" ORDER BY ");
    builder.push(opts.sort_by.to_sql_fields());

    builder.push(match opts.sort_direction {
        SortDirection::Desc => " DESC ",
        SortDirection::Asc => " ASC ",
    });

    {
        let limit = opts.limit as u32;
        let offset = (opts.page.saturating_sub(1) * opts.limit) as u32;

        builder.push("LIMIT ");
        builder.push_bind(limit);

        builder.push(" OFFSET ");
        builder.push_bind(offset);
    }

    Ok(builder
        .build_query_as::<InstanceModel>()
        .fetch_all(app_data.db())
        .await?)
}

async fn compute_max_values(
    opts: &FilterOptions,
    app_data: &Arc<AppState>,
) -> HandlerResult<MaxValues> {
    let run = opts
        .run_filters
        .as_ref()
        .map_or_else(Default::default, |f| f.run.simple().to_string());
    let solver = opts
        .run_filters
        .as_ref()
        .map_or_else(Default::default, |f| f.solver.simple().to_string());

    let values : MaxValues = conditional_query_as!(
        MaxValues,
        r#"SELECT 
            MAX(nodes)     as nodes,
            MAX(edges)     as edges,
            MAX(min_deg)   as min_deg,
            MAX(max_deg)   as max_deg,
            MAX(num_ccs)   as num_ccs,
            MAX(treewidth) as treewidth,
            MAX(best_score)as best_score,
            MAX(nodes_largest_cc) as nodes_largest_cc,
            {#rest}"#,
        #rest = match &opts.run_filters {
            None => "NULL as 'score: u32', NULL as 'score_diff: i32', NULL as 'seconds_computed: f64' FROM `Instance` i",
            Some(_) => r#"
                MAX(s.score) as score, MAX(s.score - i.best_score) as 'score_diff: i32', MAX(s.seconds_computed) as seconds_computed
            FROM `Instance` i
            JOIN Solution s ON i.iid = s.instance_iid
            JOIN SolverRun sr ON s.sr_uuid = sr.run_uuid
            WHERE sr.solver_uuid = UNHEX({solver}) AND sr.run_uuid = UNHEX({run})
            "#,
        }
    )
    .fetch_one(app_data.db())
    .await?;

    if values.score_diff.map_or(false, |x| x < 0) {
        return Err(anyhow::anyhow!("score_diff is negative").into());
    }

    Ok(values)
}

pub async fn instance_list_handler(
    State(app_data): State<Arc<AppState>>,
    Json(opts): Json<FilterOptions>,
) -> HandlerResult<impl IntoResponse> {
    opts.check_validity()?;

    let max_values = if opts.include_max_values {
        Some(compute_max_values(&opts, &app_data).await?)
    } else {
        None
    };

    let total_matches = count_matches(&opts, &app_data).await?;
    let results: Vec<InstanceResult> = retrieve_instances(&opts, &app_data)
        .await?
        .into_iter()
        .filter_map(|model: InstanceModel| {
            let tags = model.tags.as_ref().map_or(Vec::new(), |t| {
                t.split(',')
                    .filter_map(|s| s.parse::<u32>().ok())
                    .collect::<Vec<_>>()
            });

            let solution = if opts.run_filters.is_some() {
                Some(SolutionResult {
                    solution_hash: model.solution_hash.clone(),
                    error_code: SolverResultType::try_from(model.error_code? as u32).ok()?,
                    score: model.score,
                    seconds_computed: model.seconds_computed.unwrap_or(0.0),
                })
            } else {
                None
            };

            Some(InstanceResult {
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
                solution,
            })
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
    Query(opts): Query<FilterOptions>,
    State(app_data): State<Arc<AppState>>,
) -> HandlerResult<impl IntoResponse> {
    opts.check_validity()?;

    let list_as_string = {
        let mut builder = sqlx::QueryBuilder::new(r#"SELECT i.iid FROM `Instance` i "#);

        if opts.run_filters.is_some() {
            builder.push(" JOIN Solution s ON i.iid = s.instance_iid ");
        }

        if let Some(tid) = opts.tag {
            builder.push(" JOIN InstanceTag it ON i.iid = it.instance_iid WHERE it.tag_tid = ");
            builder.push_bind(tid);
        } else {
            builder.push(" WHERE 1=1 ");
        }

        builder = append_filters_to_query_builder(builder, &opts)?;

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

    macro_rules! sf {
        ($key:ident, $value:expr) => {
            SolverFilter {
                run: Uuid::parse_str("00000000-0000-0000-0001-000000000000").unwrap(),
                solver: Uuid::parse_str("00000000-0000-0000-0002-000000000000").unwrap(),
                $key: $value,
                ..Default::default()
            }
        };
    }

    #[sqlx::test(fixtures("instances", "solutions"))]
    async fn instance_list_handler_order_by(db_pool: DbPool) -> sqlx::Result<()> {
        for order in SortBy::iter() {
            let req = FilterOptions {
                sort_by: order,
                run_filters: Some(sf!(score_lb, Some(0))),
                ..Default::default()
            };

            let state = Arc::new(AppState::new(db_pool.clone()));
            let resp = super::instance_list_handler(State(state), Json(req)).await;

            assert!(resp.unwrap().into_response().status().is_success());
        }

        Ok(())
    }

    #[sqlx::test(fixtures("instances"))]
    async fn instance_list_download_handler(db_pool: DbPool) -> sqlx::Result<()> {
        for order in SortBy::iter() {
            let req = FilterOptions {
                sort_by: order,
                run_filters: Some(sf!(score_lb, Some(0))),
                ..Default::default()
            };

            let state = Arc::new(AppState::new(db_pool.clone()));
            let resp = super::instance_list_handler(State(state), Json(req)).await;

            assert!(resp.unwrap().into_response().status().is_success());
        }

        Ok(())
    }

    macro_rules! test_filter_option {
        ($name:ident, $values:expr) => {test_filter_option!($name, $name, $values);};
        ($name:ident, $key:ident, $values:expr) => {paste::paste! {
            #[sqlx::test(fixtures("instances"))]
            async fn [<filter_option_list_ $name>](db_pool: DbPool) -> sqlx::Result<()> {
                for v in $values {
                    let req = FilterOptions {
                        $key: Some(v),
                        ..Default::default()
                    };

                    let state = Arc::new(AppState::new(db_pool.clone()));
                    let resp = super::instance_list_handler(State(state), Json(req)).await;
                    assert!(resp.unwrap().into_response().status().is_success());
                }
                Ok(())
            }

            #[sqlx::test(fixtures("instances"))]
            async fn [<filter_option_download_ $name>](db_pool: DbPool) -> sqlx::Result<()> {
                for v in $values {
                    let req = FilterOptions {
                        $key: Some(v),
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

    test_filter_option!(run_score_lb, run_filters, [sf!(score_lb, Some(1))]);
    test_filter_option!(run_score_ub, run_filters, [sf!(score_ub, Some(2))]);

    test_filter_option!(
        run_score_diff_lb,
        run_filters,
        [sf!(score_diff_lb, Some(1))]
    );
    test_filter_option!(
        run_score_diff_ub,
        run_filters,
        [sf!(score_diff_ub, Some(2))]
    );

    test_filter_option!(
        run_seconds_computed_lb,
        run_filters,
        [sf!(seconds_computed_lb, Some(0.1))]
    );
    test_filter_option!(
        run_seconds_computed_ub,
        run_filters,
        [sf!(seconds_computed_ub, Some(2.0))]
    );

    test_filter_option!(
        run_status,
        run_filters,
        [
            sf!(status, ResultStatusFilter::None),
            sf!(status, ResultStatusFilter::Valid),
            sf!(status, ResultStatusFilter::Invalid),
            sf!(status, ResultStatusFilter::Optimal),
            sf!(status, ResultStatusFilter::Suboptimal),
            sf!(status, ResultStatusFilter::Incomplete),
            sf!(status, ResultStatusFilter::Infeasible),
            sf!(status, ResultStatusFilter::Timeout),
            sf!(status, ResultStatusFilter::Error),
        ]
    );

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
