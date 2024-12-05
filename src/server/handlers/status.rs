use super::common::*;

#[derive(Serialize)]
struct Response {
    status: &'static str,
    num_instances: u64,
    num_jobs: u64,
    num_unique_solutions: u64,
}

pub async fn status_handler(
    State(app_data): State<Arc<AppState>>,
) -> HandlerResult<impl IntoResponse> {
    let num_instances = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM Instance")
        .fetch_one(app_data.db())
        .await? as u64;
    let num_jobs = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM Solution")
        .fetch_one(app_data.db())
        .await? as u64;
    let num_unique_solutions = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM SolutionData")
        .fetch_one(app_data.db())
        .await? as u64;

    Ok(Json(Response {
        status: "ok",
        num_instances,
        num_jobs,
        num_unique_solutions,
    }))
}
