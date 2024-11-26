use crate::server::app_state::DbPool;

use super::common::*;
use sqlx_conditional_queries::conditional_query_as;
use uuid::Uuid;

const TARGET_OUTPUT_SIZE: usize = 1000;

#[derive(Clone, Deserialize, Serialize, Debug, Default)]
pub struct FilterOptions {
    solver: Uuid,
    runs: Vec<Uuid>,
    instances_of : Option<Uuid>,
}


#[derive(sqlx::FromRow, Debug)]
struct RowModel {
    score : f64,
    seconds_computed : f64,
}
async fn load_instances(_solver : Uuid, run : Uuid, instance_of : Option<Uuid>, db : &DbPool) -> HandlerResult<Vec<RowModel>> {
    let run_s = run.simple().to_string();
    let instance_of = instance_of.map(|x| x.simple().to_string());
    
    Ok( conditional_query_as!(
        RowModel,
        r#"
        SELECT 
            CAST(s.score as FLOAT) / CAST(i.best_score AS FLOAT) as 'score!',
            seconds_computed as 'seconds_computed!'
        FROM Solution s
        JOIN Instance i ON s.instance_iid = i.iid
        WHERE s.sr_uuid = UNHEX({run_s}) AND s.score IS NOT NULL AND s.seconds_computed IS NOT NULL AND i.best_score IS NOT NULL {#inst_of}
        "#,
        #inst_of = match instance_of {
            Some(inst_of) => "AND i.iid IN (SELECT instance_iid FROM Solution WHERE sr_uuid=UNHEX({inst_of}))",
            None => "",
        }
    )
    .fetch_all(db)
    .await? )
}


#[derive(Serialize, Debug)]
struct RunResponse {
    run: Uuid,
    score: Vec<f32>,
    seconds_computed: Vec<f32>,
}

async fn response_for_run(solver : Uuid, run : Uuid, instance_of : Option<Uuid>, db : &DbPool) -> HandlerResult<RunResponse> {
    let instances = load_instances(solver, run, instance_of, db).await?;
    
    let mut score : Vec<_> = instances.iter().map(|x| x.score as f32).collect();
    score.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());

    let mut seconds_computed : Vec<_> = instances.iter().map(|x| x.seconds_computed as f32).collect();
    seconds_computed.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());


    if seconds_computed.len() > 2 * TARGET_OUTPUT_SIZE {
        let step = seconds_computed.len() / TARGET_OUTPUT_SIZE;
        seconds_computed = seconds_computed.into_iter().step_by(step).collect();
        score = score.into_iter().step_by(step).collect();
    }
    
    Ok(RunResponse {
        run: run.to_owned(),
        score,
        seconds_computed,
    })
}


#[derive(Serialize, Debug)]
struct Response {
    status: String,
    solver: Uuid,
    runs: Vec<RunResponse>,
}

pub async fn solver_run_performance_handler(
    State(app_data): State<Arc<AppState>>,
    Json(opts): Json<FilterOptions>,
) -> HandlerResult<impl IntoResponse> {
    let mut run_responses = Vec::with_capacity(opts.runs.len());
    for run in &opts.runs {
        // TODO: could be done in parallel
        let response = response_for_run(opts.solver, *run, opts.instances_of, app_data.db()).await?;
        run_responses.push(response);
    }
    
    Ok(Json(Response {
        status: "ok".to_string(),
        solver: opts.solver,
        runs: run_responses
    }))
}
