use axum::extract::Path;
use serde_json::json;

use super::common::*;

pub async fn instance_delete_handler(
    Path(id): Path<u32>,
    State(data): State<Arc<AppState>>,
) -> HandlerResult<impl IntoResponse> {
    let mut tx = data.db().begin().await?;

    sqlx::query(r#"DELETE FROM InstanceTag WHERE instance_iid=?"#)
        .bind(id)
        .execute(&mut *tx)
        .await?;

    let solution_data_hashes = sqlx::query_as::<_, (String,)>(
        r#"SELECT HEX(solution_hash) FROM Solution WHERE instance_iid=? AND solution_hash IS NOT NULL"#,
    )
    .bind(id)
    .fetch_all(&mut *tx)
    .await?;

    sqlx::query(r#"DELETE FROM Solution WHERE instance_iid=?"#)
        .bind(id)
        .execute(&mut *tx)
        .await?;

    for (hash,) in solution_data_hashes {
        let usages_of_hash = sqlx::query_scalar::<_, i64>(
            r#"SELECT COUNT(*) FROM Solution WHERE solution_hash=UNHEX(?)"#,
        )
        .bind(&hash)
        .fetch_one(&mut *tx)
        .await?;

        if usages_of_hash == 0 {
            sqlx::query(r#"DELETE FROM SolutionData WHERE hash=UNHEX(?)"#)
                .bind(&hash)
                .execute(&mut *tx)
                .await?;
        }
    }

    // delete data, if not used by any other instance
    let data_did = sqlx::query_scalar::<_, i32>(r#"SELECT data_did FROM Instance WHERE iid=?"#)
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;

    sqlx::query(r#"DELETE FROM Instance WHERE iid=?"#)
        .bind(id)
        .execute(&mut *tx)
        .await?;

    let usages_of_did =
        sqlx::query_scalar::<_, i64>(r#"SELECT COUNT(*) FROM Instance WHERE data_did=?"#)
            .bind(data_did)
            .fetch_one(&mut *tx)
            .await?;

    if usages_of_did == 1 {
        sqlx::query(r#"DELETE FROM InstanceData WHERE did=?"#)
            .bind(data_did)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;

    Ok(Json(json!({
        "status": "ok",
        "id": id,
    })))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::server::app_state::DbPool;


    #[sqlx::test(fixtures("instances", "solutions"))]
    async fn instance_delete_handler(pool: DbPool) -> sqlx::Result<()> {
        let state = Arc::new(AppState::new(pool));

        let id = 1;

        let response = super::instance_delete_handler(Path(id), State(state.clone()))
            .await
            .unwrap()
            .into_response();

        assert!(response.status().is_success());

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM Instance WHERE iid=?")
            .bind(id)
            .fetch_one(state.db())
            .await?;

        assert_eq!(0, count);

        Ok(())
    }
}