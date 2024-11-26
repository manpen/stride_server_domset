use std::collections::HashMap;

use super::{common::*, solution_upload::SolverResultType};
use sqlx::types::chrono::{DateTime, Utc};
use sqlx_conditional_queries::conditional_query_as;
use uuid::Uuid;

#[derive(Clone, Deserialize, Serialize, Debug, Default)]
pub struct FilterOptions {
    solver: Uuid,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    run: Option<Uuid>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    instances_of: Option<Uuid>,

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
    num_incomplete: u32,

    seconds_computed_optimal: f64,
    seconds_computed_suboptimal: f64,
    seconds_computed_infeasible: f64,
    seconds_computed_error: f64,
    seconds_computed_timeout: f64,
    seconds_computed_incomplete: f64,
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
            num_incomplete: 0,

            seconds_computed_optimal: 0.0,
            seconds_computed_suboptimal: 0.0,
            seconds_computed_infeasible: 0.0,
            seconds_computed_error: 0.0,
            seconds_computed_timeout: 0.0,
            seconds_computed_incomplete: 0.0,
        })
    }
}

#[derive(Clone, Serialize, Debug, Default)]
struct Response {
    runs: Vec<RunResponse>,
    options: FilterOptions,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum SolutionTypes {
    Optimal,
    Feasible,
    Infeasible,
    Error,
    Timeout,
    IncompleteOutput,
}

#[derive(Clone, Copy, Debug, Default)]
struct CountTime {
    count: u32,
    seconds_computed: f64,
}

async fn solution_count(
    opts: &FilterOptions,
    app_data: &AppState,
) -> HandlerResult<HashMap<(u32, SolutionTypes), CountTime>> {
    struct Row {
        sr_id: i32,
        error_code: Option<u32>,
        count: i64,
        seconds_computed: f64,
    }

    let solver_uuid = opts.solver.simple().to_string();
    let run_uuid = opts.run.map(|r| r.simple().to_string());
    let instances_of_uuid = opts.instances_of.map(|r| r.simple().to_string());

    let run_solution_counts: Vec<_> = conditional_query_as!(
        Row,
        r#"SELECT
            sr.`sr_id`, s.`error_code` as 'error_code!', COUNT(s.sid) as `count!`, SUM(s.seconds_computed) as `seconds_computed!`
         FROM `Solution` s
         JOIN `SolverRun` sr ON s.`sr_uuid` = sr.`run_uuid`
         WHERE sr.`solver_uuid` = UNHEX({solver_uuid}) {#run_cond} {#inst_of_cond}
         GROUP BY sr.`sr_id`, s.`error_code`"#,
         #run_cond = match &run_uuid {
            Some(_) => "AND sr.`run_uuid` = UNHEX({run_uuid})",
            None => "",
         },
         #inst_of_cond = match &instances_of_uuid {
            Some(_) => "AND s.`instance_iid` IN (SELECT instance_iid FROM Solution WHERE sr_uuid = UNHEX({instances_of_uuid}))",
            None => "",
         },
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
            SolverResultType::IncompleteOutput => SolutionTypes::IncompleteOutput,
        };

        let key = (row.sr_id as u32, solution_type);
        let entry: &mut CountTime = hash_map.entry(key).or_default();
        entry.count += row.count as u32;
        entry.seconds_computed += row.seconds_computed;
    }

    struct OptRow {
        sr_id: i32,
        count: i64,
        seconds_computed: f64,
    }

    let valid = SolverResultType::Valid as u32;
    let num_opt_solutions = conditional_query_as!(
        OptRow,
        "SELECT 
            sr.`sr_id`, COUNT(s.sid) as `count!`, SUM(s.seconds_computed) as `seconds_computed!`
         FROM `Solution` s
         JOIN `SolverRun` sr ON s.`sr_uuid` = sr.`run_uuid`
         JOIN `Instance` i ON `s`.`instance_iid` = `i`.`iid`
         WHERE sr.`solver_uuid` = UNHEX({solver_uuid}) {#run_cond} {#inst_of_cond} AND s.`error_code` = {valid} AND i.best_score = s.score
         GROUP BY sr.`sr_id`",
        #run_cond = match &run_uuid {
            Some(_) => "AND sr.`run_uuid` = UNHEX({run_uuid})",
            None => "",
         },
         #inst_of_cond = match &instances_of_uuid {
            Some(_) => "AND s.`instance_iid` IN (SELECT instance_iid FROM Solution WHERE sr_uuid = UNHEX({instances_of_uuid}))",
            None => "",
         },
    )
    .fetch_all(app_data.db())
    .await?;

    for row in num_opt_solutions {
        let key = (row.sr_id as u32, SolutionTypes::Optimal);
        hash_map.insert(
            key,
            CountTime {
                count: row.count as u32,
                seconds_computed: row.seconds_computed,
            },
        );
    }

    Ok(hash_map)
}

pub async fn solver_run_list_handler(
    opts: Option<Query<FilterOptions>>,
    State(app_data): State<Arc<AppState>>,
) -> HandlerResult<impl IntoResponse> {
    let Query(opts) = opts.unwrap_or_default();

    let solver = opts.solver.simple().to_string();
    let run = opts.run.map(|r| r.simple().to_string());
    let run_models : Vec<_> = conditional_query_as!(RunModel,
        "SELECT 
            sr_id, run_uuid, solver_uuid, name, description, user_key, num_scheduled, created_at, hide as 'hide!'
        FROM SolverRun sr
        WHERE solver_uuid = UNHEX({solver}) {#run_cond} {#hidden_cond}
        ORDER BY created_at DESC",
        #run_cond = match &run {
            Some(_) => " AND run_uuid = UNHEX({run}) ",
            None => "",
        },
        #hidden_cond = match opts.include_hidden {
            false => " AND sr.hide = 0 ",
            true => "",
        }
        ).fetch_all(app_data.db()).await?;

    let counts = solution_count(&opts, &app_data).await?;

    let mut run_response = Vec::with_capacity(run_models.len());
    for r in run_models {
        let mut resp = RunResponse::try_from(r)?;

        macro_rules! update_resp {
            ($key:ident, $op:tt, $solution_type:expr) => {
                paste::paste! {
                    let key = (resp.sr_id, $solution_type);
                    let hash_entry = counts.get(&key).copied().unwrap_or_default();
                    resp.[<num_ $key>] $op hash_entry.count;
                    resp.[<seconds_computed_ $key>] $op hash_entry.seconds_computed;
                }
            };
        }

        // we previously count Feasible solutions as Optimal+Suboptimal
        // now subtract optimals from them ...
        update_resp!(suboptimal, +=, SolutionTypes::Feasible);
        update_resp!(suboptimal, -=, SolutionTypes::Optimal);

        update_resp!(optimal, +=, SolutionTypes::Optimal);
        update_resp!(infeasible, +=, SolutionTypes::Infeasible);
        update_resp!(error, +=, SolutionTypes::Error);
        update_resp!(timeout, +=, SolutionTypes::Timeout);

        update_resp!(incomplete, +=, SolutionTypes::IncompleteOutput);

        run_response.push(resp);
    }

    Ok(Json(Response {
        runs: run_response,
        options: opts,
    }))
}
