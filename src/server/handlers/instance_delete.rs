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
        r#"SELECT HEX(data_hash) FROM Solution WHERE instance_iid=?"#,
    )
    .bind(id)
    .fetch_all(&mut *tx)
    .await?;

    sqlx::query(r#"DELETE FROM Solution WHERE instance_iid=?"#)
        .bind(id)
        .execute(&mut *tx)
        .await?;

    for (hash,) in solution_data_hashes {
        let usages_of_hash = sqlx::query_scalar::<_, u64>(
            r#"SELECT COUNT(*) FROM Solution WHERE data_hash=UNHEX(?)"#,
        )
        .bind(&hash)
        .fetch_one(&mut *tx)
        .await?;

        if usages_of_hash == 0 {
            sqlx::query(r#"DELETE FROM SolutionData WHERE data_hash=UNHEX(?)"#)
                .bind(&hash)
                .execute(&mut *tx)
                .await?;
        }
    }

    // delete data, if not used by any other instance
    let data_did = sqlx::query_scalar::<_, u32>(r#"SELECT data_did FROM Instance WHERE iid=?"#)
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;

    sqlx::query(r#"DELETE FROM Instance WHERE iid=?"#)
        .bind(id)
        .execute(&mut *tx)
        .await?;

    let usages_of_did =
        sqlx::query_scalar::<_, u64>(r#"SELECT COUNT(*) FROM Instance WHERE data_did=?"#)
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
