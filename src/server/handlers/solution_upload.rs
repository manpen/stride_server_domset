use sha2::{Digest, Sha256};

use super::common::*;

use crate::{
    pace::{graph::*, instance_reader::PaceReader, Solution},
    server::app_state::DbPool,
};

#[derive(Debug, Deserialize, Serialize)]
pub struct SolutionUploadRequest {
    pub instance_id: u32,
    pub solver_name: Option<String>,
    pub run_uuid: uuid::Uuid,
    pub exact_candidate: Option<bool>,

    pub seconds_computed: Option<f64>,
    pub solution: String,
}

async fn read_and_verify_solution(
    db: &DbPool,
    instance_id: u32,
    solution: &str,
) -> HandlerResult<Solution> {
    struct Record {
        nodes: u32,
        data: Option<Vec<u8>>,
    }

    let record = sqlx::query_as!(Record, r"SELECT i.nodes, id.data FROM Instance i JOIN InstanceData id ON id.hash = i.data_hash WHERE i.iid = ? LIMIT 1", instance_id)
        .fetch_one(db)
        .await
        .map_err(sql_to_err_response)?;

    let solution = Solution::read(solution.as_bytes(), Some(record.nodes as NumNodes))
        .map_err(debug_to_err_response)?;

    let instance_reader = PaceReader::try_new(record.data.as_ref().unwrap().as_slice())
        .map_err(debug_to_err_response)?;

    if instance_reader.number_of_nodes() != record.nodes {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "status": "error",
                "message": "Instance node count mismatch"
            })),
        ));
    }

    let mut edges = Vec::with_capacity(instance_reader.number_of_edges() as usize);
    for edge in instance_reader {
        match edge {
            Ok(edge) => edges.push(edge.normalized()),
            Err(e) => return Err(debug_to_err_response(e)),
        }
    }

    if !solution
        .valid_domset_for_instance(record.nodes, edges.into_iter())
        .map_err(debug_to_err_response)?
    {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "status": "error",
                "message": "Solution is not a valid dominating set for the instance"
            })),
        ));
    }

    Ok(solution)
}

pub async fn solution_upload_handler(
    State(data): State<Arc<AppState>>,
    Json(body): Json<SolutionUploadRequest>,
) -> HandlerResult<impl IntoResponse> {
    let solution = read_and_verify_solution(data.db(), body.instance_id, &body.solution).await?;

    let normalized_solution = {
        let mut byte_buffer_solution = Vec::with_capacity(body.solution.len());
        solution
            .write(&mut byte_buffer_solution)
            .map_err(debug_to_err_response)?;

        String::from_utf8(byte_buffer_solution).map_err(debug_to_err_response)?
    };

    let hash = {
        let mut hasher = Sha256::new();
        hasher.update(normalized_solution.as_bytes());
        format!("{:x}", hasher.finalize())
    };

    let mut tx = data.db().begin().await.map_err(sql_to_err_response)?;

    sqlx::query(r#"INSERT INTO SolutionData (hash,data) VALUES (?, ?)"#)
        .bind(&hash)
        .bind(&normalized_solution)
        .execute(&mut *tx)
        .await
        .map_err(sql_to_err_response)?;

    let run_uuid = body.run_uuid.to_string();

    // store (if not already present) the solver run
    sqlx::query(
        r#"INSERT IGNORE INTO SolverRun (uuid, solver_name, exact_candidate) VALUES (?, ?, ?)"#,
    )
    .bind(&run_uuid)
    .bind(body.solver_name.to_owned())
    .bind(body.exact_candidate.to_owned())
    .execute(&mut *tx)
    .await
    .map_err(sql_to_err_response)?;

    // store the solution entry
    sqlx::query(
        r#"INSERT INTO Solution (sr_uuid,instance_iid,solution_hash,  score,seconds_computed) VALUES (?, ?, ?,  ?, ?)"#,
    )
    .bind(&run_uuid)
    .bind(body.instance_id)
    .bind(&hash)
    .bind(solution.solution.len() as NumNodes)
    .bind(body.seconds_computed)
    .execute(&mut *tx)
    .await
    .map_err(sql_to_err_response)?;

    tx.commit().await.map_err(sql_to_err_response)?;

    let note_response = serde_json::json!({"status": "success", "solution_hash": hash});
    Ok(Json(note_response))
}
