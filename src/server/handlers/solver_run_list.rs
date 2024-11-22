use std::collections::HashMap;

use super::{common::*, solution_upload::SolverResultType};
use sqlx::types::chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Clone, Deserialize, Serialize, Debug, Default)]
pub struct FilterOptions {
    solver: uuid::Uuid,
    #[serde(default)]
    include_hidden: bool,
}

#[derive(Clone, Debug, Default)]
struct RunModel {
    sr_id: Option<i32>,
    run_uuid: Option<Vec<u8>>,
    solver_uuid: Option<Vec<u8>>,
    hide: i8,

    created_at: Option<DateTime<Utc>>,

    name: Option<String>,
    description: Option<String>,
    user_key: Option<String>,

    num_scheduled: Option<u32>,
}

#[derive(Clone, Serialize, Debug, Default)]
struct RunResponse {
    sr_id: u32,
    run_uuid: Uuid,
    solver_uuid: Uuid,
    hide: bool,

    created_at: String,

    name: Option<String>,
    description: Option<String>,
    user_key: Option<String>,

    num_scheduled: Option<u32>,
    num_optimal: u32,
    num_suboptimal: u32,
    num_infeasible: u32,
    num_error: u32,
    num_timeout: u32,
}

impl TryFrom<RunModel> for RunResponse {
    type Error = anyhow::Error;
    fn try_from(r: RunModel) -> std::result::Result<Self, Self::Error> {
        let run_uuid = match r.run_uuid {
            Some(uuid) => Uuid::from_slice(uuid.as_slice())?,
            None => return Err(anyhow::anyhow!("run_uuid is required")),
        };

        let solver_uuid = match r.solver_uuid {
            Some(uuid) => Uuid::from_slice(uuid.as_slice())?,
            None => return Err(anyhow::anyhow!("solver_uuid is required")),
        };

        let sr_id = match r.sr_id {
            Some(id) => id as u32,
            None => return Err(anyhow::anyhow!("sr_id is required")),
        };

        let created_at = match r.created_at {
            Some(date) => date.to_rfc3339(),
            None => return Err(anyhow::anyhow!("created_at is required")),
        };

        Ok(RunResponse {
            sr_id,
            run_uuid,
            solver_uuid,
            created_at,
            hide: r.hide != 0,
            name: r.name,
            description: r.description,
            user_key: r.user_key,
            num_scheduled: r.num_scheduled,
            num_optimal: 0,
            num_suboptimal: 0,
            num_infeasible: 0,
            num_error: 0,
            num_timeout: 0,
        })
    }
}

#[derive(Clone, Serialize, Debug, Default)]
struct Response {
    runs: Vec<RunResponse>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum SolutionTypes {
    Optimal,
    Feasible,
    Infeasible,
    Error,
    Timeout,
}

async fn solution_count(
    opts: &FilterOptions,
    app_data: &AppState,
) -> HandlerResult<HashMap<(u32, SolutionTypes), u32>> {
    struct Row {
        sr_id: i32,
        error_code: Option<u32>,
        count: i64,
    }

    let run_solution_counts: Vec<_> = sqlx::query_as!(
        Row,
        "SELECT 
            sr.`sr_id`, s.`error_code` as 'error_code!', COUNT(s.sid) as `count!`
         FROM `Solution` s
         JOIN `SolverRun` sr ON s.`sr_uuid` = sr.`run_uuid`
         WHERE sr.`solver_uuid` = UNHEX(?)
         GROUP BY sr.`sr_id`, s.`error_code`",
        opts.solver.simple().to_string()
    )
    .fetch_all(app_data.db())
    .await?;

    let mut hash_map = HashMap::with_capacity(run_solution_counts.len());
    for row in run_solution_counts {
        let solver_result_type = SolverResultType::try_from(match row.error_code {
            Some(code) => code,
            None => continue,
        })?;

        let solution_type = match solver_result_type {
            SolverResultType::Valid => SolutionTypes::Feasible,
            SolverResultType::Infeasible => SolutionTypes::Infeasible,
            SolverResultType::SyntaxError => SolutionTypes::Error,
            SolverResultType::Timeout => SolutionTypes::Timeout,
            SolverResultType::NonCompetitive => SolutionTypes::Feasible,
        };

        let key = (row.sr_id as u32, solution_type);
        *hash_map.entry(key).or_insert(0) += row.count as u32;
    }

    struct OptRow {
        sr_id: i32,
        count: i64,
    }

    let num_opt_solutions = sqlx::query_as!(
        OptRow,
        "SELECT 
            sr.`sr_id`, COUNT(s.sid) as `count!`
         FROM `Solution` s
         JOIN `SolverRun` sr ON s.`sr_uuid` = sr.`run_uuid`
         JOIN `Instance` i ON `s`.`instance_iid` = `i`.`iid`
         WHERE sr.`solver_uuid` = UNHEX(?) AND s.`error_code` = ? AND i.best_score = s.score
         GROUP BY sr.`sr_id`",
        opts.solver.simple().to_string(),
        SolverResultType::Valid as u32
    )
    .fetch_all(app_data.db())
    .await?;

    for row in num_opt_solutions {
        let key = (row.sr_id as u32, SolutionTypes::Optimal);
        hash_map.insert(key, row.count as u32);
    }

    Ok(hash_map)
}

pub async fn solver_run_list_handler(
    opts: Option<Query<FilterOptions>>,
    State(app_data): State<Arc<AppState>>,
) -> HandlerResult<impl IntoResponse> {
    let opts = opts.unwrap_or_default();

    let run_models : Vec<_> = sqlx::query_as!(RunModel,
        "SELECT 
            sr_id, run_uuid, solver_uuid, name, description, user_key, num_scheduled, created_at, hide as 'hide!'
        FROM SolverRun sr
        WHERE solver_uuid = UNHEX(?) and (sr.hide = 0 or ?)
        ORDER BY created_at DESC",
          opts.solver.simple().to_string(),
            opts.include_hidden
        ).fetch_all(app_data.db()).await?;

    let counts = solution_count(&opts, &app_data).await?;

    let mut run_response = Vec::with_capacity(run_models.len());
    for r in run_models {
        let mut resp = RunResponse::try_from(r)?;

        resp.num_optimal = *counts
            .get(&(resp.sr_id, SolutionTypes::Optimal))
            .unwrap_or(&0);
        let num_feasible = *counts
            .get(&(resp.sr_id, SolutionTypes::Feasible))
            .unwrap_or(&0);

        if resp.num_optimal > num_feasible {
            return Err(anyhow::anyhow!(
                "Optimal solutions count is greater than feasible solutions count"
            )
            .into());
        }

        resp.num_suboptimal = num_feasible - resp.num_optimal;
        resp.num_infeasible = *counts
            .get(&(resp.sr_id, SolutionTypes::Infeasible))
            .unwrap_or(&0);
        resp.num_timeout = *counts
            .get(&(resp.sr_id, SolutionTypes::Timeout))
            .unwrap_or(&0);
        resp.num_error = *counts
            .get(&(resp.sr_id, SolutionTypes::Error))
            .unwrap_or(&0);

        run_response.push(resp);
    }

    Ok(Json(Response { runs: run_response }))
}
