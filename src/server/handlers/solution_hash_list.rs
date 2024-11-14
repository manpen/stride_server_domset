use axum::extract::Path;
use serde_json::json;
use tracing::debug;
use uuid::Uuid;

use super::common::*;

pub async fn solution_hash_list_handler(
    Path(solver_uuid): Path<Uuid>,
    State(app_data): State<Arc<AppState>>,
) -> HandlerResult<impl IntoResponse> {
    let hashes = sqlx::query_scalar!(
        r#"SELECT 
             HEX(s.solution_hash) as "hash!: String"
        FROM Solution s
        JOIN SolverRun sr ON s.sr_uuid = sr.run_uuid
        WHERE s.solution_hash IS NOT NULL AND sr.solver_uuid = UNHEX(?)"#,
        solver_uuid.simple().to_string()
    )
    .fetch_all(app_data.db())
    .await
    .map_err(sql_to_err_response)?;

    debug!(
        "Return {} hashes for solver {uuid:?}",
        hashes.len(),
        uuid = solver_uuid.to_string()
    );

    Ok(Json(json!({
        "status": "ok",
        "hashes": hashes,
    })))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::server::app_state::DbPool;

    #[sqlx::test(fixtures("instances", "solutions"))]

    async fn solution_hash_list(pool: DbPool) -> sqlx::Result<()> {
        let app_state = Arc::new(AppState::new(pool));

        let _response = super::solution_hash_list_handler(
            Path(Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap()),
            State(app_state),
        )
        .await
        .unwrap();

        Ok(())
    }
}
