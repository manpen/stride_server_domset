use super::common::*;

use crate::{
    pace::{graph::*, instance_reader::PaceReader, Solution},
    server::app_state::DbPool,
};

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
pub enum SolutionErrorCode {
    Valid = 0,
    Invalid = 1,
    Timeout = 2,
    SyntaxError = 3,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SolutionUploadRequest {
    pub instance_id: u32,

    pub run_uuid: uuid::Uuid,
    pub solver_uuid: Option<uuid::Uuid>,

    pub seconds_computed: f64,

    pub solution: Option<Vec<Node>>,
    pub error_code: Option<SolutionErrorCode>,
}

async fn read_instance_data(db: &DbPool, instance_id: u32) -> HandlerResult<(NumNodes, Vec<Edge>)> {
    struct Record {
        nodes: u32,
        data: Option<Vec<u8>>,
    }

    let record = sqlx::query_as!(Record, r"SELECT i.nodes, id.data FROM Instance i JOIN InstanceData id ON id.hash = i.data_hash WHERE i.iid = ? LIMIT 1", instance_id)
            .fetch_one(db)
            .await
            .map_err(sql_to_err_response)?;

    let instance_reader = PaceReader::try_new(record.data.as_ref().unwrap().as_slice())
        .map_err(debug_to_err_response)?;

    if instance_reader.number_of_nodes() != record.nodes {
        return_bad_request_json!("Instance node count mismatch");
    }

    let mut edges = Vec::with_capacity(instance_reader.number_of_edges() as usize);
    for edge in instance_reader {
        match edge {
            Ok(edge) => edges.push(edge.normalized()),
            Err(e) => return Err(debug_to_err_response(e)),
        }
    }

    Ok((record.nodes as NumNodes, edges))
}

async fn verify_solution(
    db: &DbPool,
    instance_id: u32,
    solution: Vec<Node>,
) -> HandlerResult<Solution> {
    let (nodes, edges) = read_instance_data(db, instance_id).await?;

    let solution =
        Solution::from_vec(solution, Some(nodes as NumNodes)).map_err(debug_to_err_response)?;

    if !solution
        .valid_domset_for_instance(nodes, edges.into_iter())
        .map_err(debug_to_err_response)?
    {
        return_bad_request_json!("Solution is not a valid dominating set for the instance");
    }

    Ok(solution)
}

pub async fn solution_upload_handler(
    State(data): State<Arc<AppState>>,
    Json(mut body): Json<SolutionUploadRequest>,
) -> HandlerResult<impl IntoResponse> {
    let error_code = body.error_code.unwrap_or(SolutionErrorCode::Valid);

    let solution = if error_code == SolutionErrorCode::Valid {
        if let Some(solution) = &mut body.solution {
            Some(verify_solution(data.db(), body.instance_id, std::mem::take(solution)).await?)
        } else {
            return_bad_request_json!("Error code is 'Valid', but no solution provided");
        }
    } else if body.solution.as_ref().map_or(false, |s| !s.is_empty()) {
        return_bad_request_json!("Error code is marks invalid solution, but solution provided");
    } else {
        None
    };

    let hash = solution
        .as_ref()
        .map(|s| format!("{:x}", s.compute_digest()));

    let encoded_solution = if let Some(solution) = &solution {
        Some(serde_json::to_string(solution.solution()).map_err(debug_to_err_response)?)
    } else {
        None
    };

    let mut tx = data.db().begin().await.map_err(sql_to_err_response)?;

    if let Some(hash) = &hash {
        sqlx::query(r#"INSERT IGNORE INTO SolutionData (hash,data) VALUES (?, ?)"#)
            .bind(hash)
            .bind(&encoded_solution)
            .execute(&mut *tx)
            .await
            .map_err(sql_to_err_response)?;
    }

    let run_uuid = body.run_uuid.to_string();

    // store (if not already present) the solver run
    sqlx::query(r#"INSERT IGNORE INTO SolverRun (run_uuid, solver_uuid) VALUES (?, ?)"#)
        .bind(&run_uuid)
        .bind(body.solver_uuid)
        .execute(&mut *tx)
        .await
        .map_err(sql_to_err_response)?;

    // store the solution entry
    sqlx::query(
        r#"INSERT INTO Solution (sr_uuid,instance_iid, solution_hash,error_code,  score,seconds_computed) VALUES (?, ?,  ?, ?,  ?, ?)"#,
    )
    .bind(&run_uuid)
    .bind(body.instance_id)
    //
    .bind(&hash)
    .bind(error_code as u32)
    //
    .bind(solution.map(|s| s.solution.len() as NumNodes))
    .bind(body.seconds_computed)
    .execute(&mut *tx)
    .await
    .map_err(sql_to_err_response)?;

    tx.commit().await.map_err(sql_to_err_response)?;

    let note_response = serde_json::json!({"status": "success", "solution_hash": hash});
    Ok(Json(note_response))
}

#[cfg(test)]
mod test {
    use super::*;

    #[sqlx::test(fixtures("instances"))]
    async fn read_instance_data(pool: DbPool) -> sqlx::Result<()> {
        let (nodes, edges) = super::read_instance_data(&pool, 2).await.unwrap();

        assert_eq!(nodes, 3);
        assert_eq!(edges, vec![Edge(0, 1), Edge(1, 2)]);

        Ok(())
    }

    #[sqlx::test(fixtures("instances"))]
    async fn verify_solution(pool: DbPool) -> sqlx::Result<()> {
        let solution = vec![1 as Node, 2];

        let _ = super::verify_solution(&pool, 2, solution).await.unwrap();

        let solution = vec![1 as Node];

        assert!(super::verify_solution(&pool, 2, solution).await.is_err());

        Ok(())
    }

    #[sqlx::test(fixtures("instances"))]
    async fn solution_upload_handler(pool: DbPool) -> sqlx::Result<()> {
        let state = Arc::new(AppState::new(pool));
        macro_rules! test {
            ($request:expr, $success:expr) => {{
                let response =
                    super::solution_upload_handler(State(state.clone()), Json($request)).await;

                if ($success) {
                    assert!(response.unwrap().into_response().status().is_success());
                } else {
                    assert!(response.is_err());
                }
            }};
        }

        test!(
            SolutionUploadRequest {
                instance_id: 2,
                run_uuid: uuid::Uuid::new_v4(),
                solver_uuid: None,
                seconds_computed: 1.0,
                solution: Some(vec![1 as Node, 2]),
                error_code: None,
            },
            true
        );

        test!(
            SolutionUploadRequest {
                instance_id: 2,
                run_uuid: uuid::Uuid::new_v4(),
                solver_uuid: None,
                seconds_computed: 1.0,

                solution: Some(vec![1 as Node, 2]),
                error_code: Some(SolutionErrorCode::Invalid), // code is invalid, but solution is given
            },
            false
        );

        test!(
            SolutionUploadRequest {
                instance_id: 2,
                run_uuid: uuid::Uuid::new_v4(),
                solver_uuid: None,
                seconds_computed: 1.0,

                solution: None,
                error_code: Some(SolutionErrorCode::Invalid),
            },
            true
        );

        let run_uuid = uuid::Uuid::new_v4();
        let solver_uuid = uuid::Uuid::new_v4();

        test!(
            SolutionUploadRequest {
                instance_id: 2,
                run_uuid,
                solver_uuid: Some(solver_uuid),
                seconds_computed: 1.0,
                solution: Some(vec![1 as Node, 2]),
                error_code: None,
            },
            true
        );

        test!(
            SolutionUploadRequest {
                instance_id: 2,
                run_uuid,
                solver_uuid: Some(solver_uuid),
                seconds_computed: 1.0,
                solution: Some(vec![2 as Node]),
                error_code: None,
            },
            false // there's already a solution for this run_uuid and instance_id
        );

        assert_eq!(
            sqlx::query_scalar::<_, i32>(r"SELECT COUNT(*) FROM SolverRun WHERE solver_uuid=?")
                .bind(solver_uuid)
                .fetch_one(state.db())
                .await
                .unwrap(),
            1
        );

        test!(
            SolutionUploadRequest {
                instance_id: 2,
                run_uuid: uuid::Uuid::new_v4(),
                solver_uuid: Some(solver_uuid),
                seconds_computed: 1.0,
                solution: Some(vec![2 as Node]),
                error_code: None,
            },
            true
        );

        assert_eq!(
            sqlx::query_scalar::<_, i32>(r"SELECT COUNT(*) FROM SolverRun WHERE solver_uuid=?")
                .bind(solver_uuid)
                .fetch_one(state.db())
                .await
                .unwrap(),
            2
        );

        Ok(())
    }
}
