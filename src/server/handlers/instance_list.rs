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

    #[serde(default)]
    pub iid: Option<u32>,

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

    #[serde(default, skip_serializing_if = "Option::is_none")]
    solver: Option<Uuid>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    run: Option<Uuid>,

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

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search : Option<String>,

    #[serde(default)]
    pub result_status: ResultStatusFilter,
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

const SORT_BY_ONLY_WITH_RUN: [SortBy; 4] = [
    SortBy::Score,
    SortBy::ScoreDiff,
    SortBy::SecondsComputed,
    SortBy::ErrorCode,
];

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

impl FilterOptions {
    fn check_validity(&self) -> HandlerResult<()> {
        if self.run.is_some() != self.solver.is_some() {
            return Err(anyhow::anyhow!("solver and run must be both provided or both not").into());
        }

        if self.run.is_none() {
            // assert that no fields are set that require run-mode
            if self.score_lb.is_some()
                || self.score_ub.is_some()
                || self.score_diff_lb.is_some()
                || self.score_diff_ub.is_some()
                || self.seconds_computed_lb.is_some()
                || self.seconds_computed_ub.is_some()
                || self.result_status != ResultStatusFilter::None
            {
                return Err(anyhow::anyhow!(
                    "solver and run must be provided when filtering by score, score_diff, seconds_computed or status"
                )
                .into());
            }

            if SORT_BY_ONLY_WITH_RUN.iter().contains(&self.sort_by) {
                return Err(anyhow::anyhow!(
                    "sort-by option requires solver and run to be provided"
                )
                .into());
            }
        }

        Ok(())
    }

    fn solver_and_run_strings(&self) -> Option<(String, String)> {
        let run = self.run?.simple().to_string();
        let solver = self.solver?.simple().to_string();
        Some((solver, run))
    }
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

    if let Some(x) = opts.iid {
        builder.push(" AND i.iid = ");
        builder.push_bind(x);
    }

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

    if let Some((_solver, run)) = opts.solver_and_run_strings() {
        builder.push(" AND s.sr_uuid = UNHEX(");
        builder.push_bind(run);
        builder.push(")");

        append_range_filter!(opts, score, "s.score");
        append_range_filter!(opts, score_diff, "s.score - i.best_score");
        append_range_filter!(opts, seconds_computed, "s.seconds_computed");

        match opts.result_status {
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

    if let Some(search) = &opts.search {
        builder.push (" AND (MATCH (i.`name`, i.`description`, i.`submitted_by`) AGAINST (");
        builder.push_bind(search);
        builder.push(")");

        for word in search.split_whitespace() {
            if let Ok(number) = word.parse::<u32>() {
                builder.push(" OR i.iid = ");
                builder.push_bind(number);
            }
        }
        builder.push(") ");
    }

    Ok(builder)
}

async fn count_matches(opts: &FilterOptions, app_data: &Arc<AppState>) -> HandlerResult<u32> {
    let mut builder = sqlx::QueryBuilder::new(r#"SELECT COUNT(*) as cnt FROM `Instance` i "#);

    if opts.run.is_some() {
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

    if opts.run.is_some() {
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
    if opts.run.is_some() {
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
    let solver_and_run = opts.solver_and_run_strings();

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
        #rest = match solver_and_run {
            None => "NULL as 'score: u32', NULL as 'score_diff: i32', NULL as 'seconds_computed: f64' FROM `Instance` i",
            Some((solver_s, run_s)) => r#"
                MAX(s.score) as score, MAX(s.score - i.best_score) as 'score_diff: i32', MAX(s.seconds_computed) as seconds_computed
            FROM `Instance` i
            JOIN Solution s ON i.iid = s.instance_iid
            JOIN SolverRun sr ON s.sr_uuid = sr.run_uuid
            WHERE sr.solver_uuid = UNHEX({solver_s}) AND sr.run_uuid = UNHEX({run_s})
            "#,
        }
    )
    .fetch_one(app_data.db())
    .await?;

    if values.score_diff.is_some_and(|x| x < 0) {
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

            let solution = if opts.run.is_some() {
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

        if opts.run.is_some() {
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

    let document = format!("c {}\n{list_as_string}", serde_json::to_string(&opts)?);

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

    const RUN_MODE: bool = true;

    fn run_uuid() -> Uuid {
        Uuid::parse_str("00000000-0000-0000-0001-000000000000").unwrap()
    }

    fn solver_uuid() -> Uuid {
        Uuid::parse_str("00000000-0000-0000-0002-000000000000").unwrap()
    }

    #[sqlx::test(fixtures("instances", "solutions"))]
    async fn instance_list_handler_order_by(db_pool: DbPool) -> sqlx::Result<()> {
        for run_mode in [false, true] {
            for order in SortBy::iter() {
                if !run_mode && SORT_BY_ONLY_WITH_RUN.iter().contains(&order) {
                    continue;
                }

                let mut req = FilterOptions {
                    sort_by: order,
                    ..Default::default()
                };

                if run_mode {
                    req.run = Some(run_uuid());
                    req.solver = Some(solver_uuid());
                }

                let state = Arc::new(AppState::new(db_pool.clone()));
                let resp = super::instance_list_handler(State(state), Json(req)).await;

                assert!(resp.unwrap().into_response().status().is_success());
            }
        }

        Ok(())
    }

    #[sqlx::test(fixtures("instances"))]
    async fn instance_list_download_handler_order_by(db_pool: DbPool) -> sqlx::Result<()> {
        for run_mode in [false, true] {
            for order in SortBy::iter() {
                if !run_mode && SORT_BY_ONLY_WITH_RUN.iter().contains(&order) {
                    continue;
                }

                let mut req = FilterOptions {
                    sort_by: order,
                    ..Default::default()
                };

                if run_mode {
                    req.run = Some(run_uuid());
                    req.solver = Some(solver_uuid());
                }

                let state = Arc::new(AppState::new(db_pool.clone()));
                let resp = super::instance_list_handler(State(state), Json(req)).await;

                assert!(resp.unwrap().into_response().status().is_success());
            }
        }

        Ok(())
    }

    macro_rules! test_filter_option {
        ($key:ident, $values:expr) => {test_filter_option!($key, $values, false);};
        ($key:ident, $values:expr, $run_mode:expr) => {paste::paste! {
            #[sqlx::test(fixtures("instances"))]
            async fn [<filter_option_list_ $key>](db_pool: DbPool) -> sqlx::Result<()> {
                for v in $values {
                    let mut req = FilterOptions {
                        $key: v,
                        ..Default::default()
                    };

                    if $run_mode {
                        req.run = Some(run_uuid());
                        req.solver = Some(solver_uuid());
                    }

                    let state = Arc::new(AppState::new(db_pool.clone()));
                    let resp = super::instance_list_handler(State(state), Json(req)).await;
                    assert!(resp.unwrap().into_response().status().is_success());
                }
                Ok(())
            }

            #[sqlx::test(fixtures("instances"))]
            async fn [<filter_option_download_ $key>](db_pool: DbPool) -> sqlx::Result<()> {
                for v in $values {
                    let mut req = FilterOptions {
                        $key: v,
                        ..Default::default()
                    };

                    if $run_mode {
                        req.run = Some(run_uuid());
                        req.solver = Some(solver_uuid());
                    }

                    let state = Arc::new(AppState::new(db_pool.clone()));
                    let resp = super::instance_list_download_handler(Query(req), State(state)).await;
                    assert!(resp.unwrap().into_response().status().is_success());
                }
                Ok(())
            }
        }};
    }

    test_filter_option!(score_lb, [Some(0), Some(1)], RUN_MODE);
    test_filter_option!(score_ub, [Some(0), Some(1)], RUN_MODE);

    test_filter_option!(score_diff_lb, [Some(0), Some(1)], RUN_MODE);
    test_filter_option!(score_diff_ub, [Some(0), Some(1)], RUN_MODE);

    test_filter_option!(seconds_computed_lb, [Some(0.1), Some(1.1)], RUN_MODE);
    test_filter_option!(seconds_computed_ub, [Some(1.1), Some(2.1)], RUN_MODE);

    test_filter_option!(
        result_status,
        [
            ResultStatusFilter::None,
            ResultStatusFilter::Valid,
            ResultStatusFilter::Invalid,
            ResultStatusFilter::Optimal,
            ResultStatusFilter::Suboptimal,
            ResultStatusFilter::Incomplete,
            ResultStatusFilter::Infeasible,
            ResultStatusFilter::Timeout,
            ResultStatusFilter::Error,
        ],
        RUN_MODE
    );

    test_filter_option!(tag, [Some(0), Some(1)]);
    test_filter_option!(nodes_lb, [Some(0), Some(1)]);
    test_filter_option!(nodes_ub, [Some(0), Some(1)]);
    test_filter_option!(edges_lb, [Some(0), Some(1)]);
    test_filter_option!(edges_ub, [Some(0), Some(1)]);
    test_filter_option!(best_score_lb, [Some(0), Some(1)]);
    test_filter_option!(best_score_ub, [Some(0), Some(1)]);
    test_filter_option!(min_deg_lb, [Some(0), Some(1)]);
    test_filter_option!(min_deg_ub, [Some(0), Some(1)]);
    test_filter_option!(max_deg_lb, [Some(0), Some(1)]);
    test_filter_option!(max_deg_ub, [Some(0), Some(1)]);
    test_filter_option!(num_ccs_lb, [Some(0), Some(1)]);
    test_filter_option!(num_ccs_ub, [Some(0), Some(1)]);
    test_filter_option!(nodes_largest_cc_lb, [Some(0), Some(1)]);
    test_filter_option!(nodes_largest_cc_ub, [Some(0), Some(1)]);
    test_filter_option!(diameter_lb, [Some(0), Some(1)]);
    test_filter_option!(diameter_ub, [Some(0), Some(1)]);
    test_filter_option!(treewidth_lb, [Some(0), Some(1)]);
    test_filter_option!(treewidth_ub, [Some(0), Some(1)]);
    test_filter_option!(planar, [Some(false), Some(true)]);
    test_filter_option!(bipartite, [Some(false), Some(true)]);
    test_filter_option!(regular, [Some(false), Some(true)]);
}
