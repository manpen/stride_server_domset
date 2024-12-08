use serde::Deserialize;
use sqlx::types::chrono::{DateTime, Utc};
use uuid::Uuid;

use super::{common::*, solution_upload::SolverResultType};

pub async fn instance_solutions_handler(
    Query(opts): Query<FilterOptions>,
    State(app_data): State<Arc<AppState>>,
) -> HandlerResult<impl IntoResponse> {
    let global_hist = compute_global_histogram(&app_data, opts.iid).await?;

    let solver_solutions = if let Some(solver) = opts.solver {
        Some(fetch_solutions_of_solver(&app_data, opts.iid, solver).await?)
    } else {
        None
    };

    Ok(Json(Response {
        status: "ok",
        filters: opts,
        global_score_histogram: global_hist,
        solver_solutions,
    }))
}

async fn compute_global_histogram(
    app_data: &AppState,
    iid: u32,
) -> anyhow::Result<Vec<HistogramEntry>> {
    Ok(sqlx::query_as!(HistogramEntry,
        r#"SELECT score, COUNT(*) as "count: u64" FROM Solution WHERE instance_iid = ? GROUP BY score"#,
        iid).fetch_all(app_data.db()).await?)
}

async fn fetch_solutions_of_solver(
    app_data: &AppState,
    iid: u32,
    solver: Uuid,
) -> anyhow::Result<Vec<SolutionRun>> {
    struct Row {
        created_at: DateTime<Utc>,
        run: Vec<u8>,
        run_name: Option<String>,
        run_description: Option<String>,

        seconds_computed: Option<f64>,
        score: Option<u32>,
        error_code: u32,
    }

    let rows = sqlx::query_as!(
        Row,
        r#"SELECT 
            s.created_at as "created_at!",
            s.sr_uuid as run,
            sr.name as run_name,
            sr.description as run_description,
            s.seconds_computed,
            s.score,
            s.error_code as "error_code!"
           FROM Solution s
           JOIN SolverRun sr ON s.sr_uuid = sr.run_uuid
           WHERE s.instance_iid = ? AND sr.solver_uuid = UNHEX(?)"#,
        iid,
        solver.simple().to_string()
    )
    .fetch_all(app_data.db())
    .await?;

    let mut result = Vec::with_capacity(rows.len());
    for row in rows {
        result.push(SolutionRun {
            created_at: row.created_at.to_rfc3339(),
            run: Uuid::from_slice(&row.run)?,
            run_name: row.run_name,
            run_description: row.run_description,
            seconds_computed: row.seconds_computed,
            score: row.score,
            status: SolverResultType::try_from(row.error_code)?,
        });
    }

    Ok(result)
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct FilterOptions {
    iid: u32,
    solver: Option<Uuid>,
}

#[derive(Clone, Serialize, Debug, Default)]
struct HistogramEntry {
    score: Option<u32>,
    count: u64,
}

#[derive(Clone, Serialize, Debug)]
struct SolutionRun {
    created_at: String,
    run: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    run_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    run_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    seconds_computed: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    score: Option<u32>,
    status: SolverResultType,
}

#[derive(Clone, Serialize, Debug, Default)]
pub struct Response {
    status: &'static str,
    filters: FilterOptions,
    global_score_histogram: Vec<HistogramEntry>,

    #[serde(skip_serializing_if = "Option::is_none")]
    solver_solutions: Option<Vec<SolutionRun>>,
}
