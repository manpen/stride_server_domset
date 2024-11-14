use tracing::{debug, error};

use super::common::*;

use crate::{
    pace::{graph::*, instance_reader::PaceReader, Solution},
    server::app_state::{DbPool, DbTransaction},
};

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum SolverResult {
    Valid {
        data: Vec<Node>,
    },
    ValidCached {
        hash: String,
    },
    Infeasible,
    SyntaxError,
    Timeout,
    NonCompetitive,

    #[serde(skip_deserializing)]
    Empty, // internal use only, to allow moving solutions out without copying
}

#[derive(Debug, Eq, PartialEq)]
enum SolverResultType {
    Valid = 1,
    Infeasible = 2,
    SyntaxError = 3,
    Timeout = 4,
    NonCompetitive = 5,
}

impl SolverResult {
    fn result_type(&self) -> Option<SolverResultType> {
        match self {
            SolverResult::Valid { .. } => Some(SolverResultType::Valid),
            SolverResult::ValidCached { .. } => Some(SolverResultType::Valid),
            SolverResult::Infeasible => Some(SolverResultType::Infeasible),
            SolverResult::SyntaxError => Some(SolverResultType::SyntaxError),
            SolverResult::Timeout => Some(SolverResultType::Timeout),
            SolverResult::NonCompetitive => Some(SolverResultType::NonCompetitive),
            SolverResult::Empty => None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SolutionUploadRequest {
    pub instance_id: u32,

    pub run_uuid: uuid::Uuid,
    pub solver_uuid: Option<uuid::Uuid>,

    pub seconds_computed: f64,
    pub result: SolverResult,
}

async fn read_instance_data(db: &DbPool, instance_id: u32) -> HandlerResult<(NumNodes, Vec<Edge>)> {
    struct Record {
        nodes: u32,
        data: Option<Vec<u8>>,
    }

    let record = sqlx::query_as!(Record, r"SELECT i.nodes, id.data FROM Instance i JOIN InstanceData id ON id.did = i.data_did WHERE i.iid = ? LIMIT 1", instance_id)
            .fetch_one(db)
            .await
            .map_err(sql_to_err_response)?;

    let instance_reader = PaceReader::try_new(record.data.as_ref().unwrap().as_slice())
        .map_err(debug_to_err_response)?;

    if instance_reader.number_of_nodes() != record.nodes {
        return bad_request_json!("Instance node count mismatch");
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
        return bad_request_json!("Solution is not a valid dominating set for the instance");
    }

    Ok(solution)
}

async fn insert_solution_data(
    tx: &mut DbTransaction<'_>,
    solution: &Solution,
) -> HandlerResult<String> {
    let hash = format!("{:x}", solution.compute_digest());

    let encoded_solution =
        serde_json::to_string(solution.solution()).map_err(debug_to_err_response)?;

    sqlx::query(r#"INSERT IGNORE INTO SolutionData (hash,data) VALUES (UNHEX(?), ?)"#)
        .bind(&hash)
        .bind(encoded_solution)
        .execute(&mut **tx)
        .await
        .map_err(sql_to_err_response)?;

    debug!(" Processed SolutionData entry with hash {hash}");

    Ok(hash)
}

async fn insert_solver_run_entry(
    tx: &mut DbTransaction<'_>,
    body: &SolutionUploadRequest,
) -> HandlerResult<()> {
    // store (if not already present) the solver run
    sqlx::query(
        r#"INSERT IGNORE INTO SolverRun (run_uuid, solver_uuid) VALUES (UNHEX(?), UNHEX(?))"#,
    )
    .bind(body.run_uuid.simple().to_string())
    .bind(body.solver_uuid.as_ref().map(|x| x.simple().to_string()))
    .execute(&mut **tx)
    .await
    .map_err(sql_to_err_response)?;

    debug!(" Processed SolverRun entry");

    Ok(())
}

async fn insert_valid_solution_entry(
    tx: &mut DbTransaction<'_>,
    body: &SolutionUploadRequest,
    solution_hash: &str,
    solution_score: NumNodes,
) -> HandlerResult<()> {
    sqlx::query(
        r#"INSERT INTO Solution (sr_uuid,instance_iid, solution_hash,error_code,  score,seconds_computed) VALUES (UNHEX(?), ?,  UNHEX(?), ?,  ?, ?)"#,
    )
    .bind(body.run_uuid.simple().to_string())
    .bind(body.instance_id)
    //
    .bind(solution_hash)
    .bind(SolverResultType::Valid as u32)
    //
    .bind(solution_score as NumNodes)
    .bind(body.seconds_computed)
    .execute(&mut **tx)
    .await
    .map_err(sql_to_err_response)?;

    debug!(" Successfully inserted record of valid solution");

    Ok(())
}

async fn insert_invalid_solution_entry(
    tx: &mut DbTransaction<'_>,
    body: &SolutionUploadRequest,
    result_type: SolverResultType,
) -> HandlerResult<()> {
    if result_type == SolverResultType::Valid {
        error!("result_type indicates valid solution in invalid branch");
        return bad_request_json!("Invalid solution result");
    };

    sqlx::query(
        r#"INSERT INTO Solution (sr_uuid,instance_iid, solution_hash,error_code,  score,seconds_computed) VALUES (UNHEX(?), ?,  NULL, ?,  NULL, ?)"#,
    )
    .bind(body.run_uuid.simple().to_string())
    .bind(body.instance_id)
    .bind(result_type as u32)
    .bind(body.seconds_computed)
    .execute(&mut **tx)
    .await
    .map_err(sql_to_err_response)?;

    debug!(" Successfully inserted record of invalid solution");

    Ok(())
}

async fn handle_valid_new_solution(
    app_data: Arc<AppState>,
    request: SolutionUploadRequest,
    solution_data: Vec<Node>,
) -> HandlerResult<impl IntoResponse> {
    debug!("Handling upload of new solution data");

    let solution = verify_solution(app_data.db(), request.instance_id, solution_data).await?;
    let solution_score = solution.solution.len() as NumNodes;

    let mut tx = app_data.db().begin().await.map_err(sql_to_err_response)?;

    insert_solver_run_entry(&mut tx, &request).await?;
    let solution_hash = insert_solution_data(&mut tx, &solution).await?;
    insert_valid_solution_entry(&mut tx, &request, &solution_hash, solution_score).await?;

    tx.commit().await.map_err(sql_to_err_response)?;

    let note_response = serde_json::json!({"status": "success", "solution_hash": solution_hash});
    Ok(Json(note_response))
}

async fn handle_valid_cached_solution(
    app_data: Arc<AppState>,
    request: SolutionUploadRequest,
    solution_hash: String,
) -> HandlerResult<impl IntoResponse> {
    debug!("Handling upload of cached solution data");

    let mut tx = app_data.db().begin().await.map_err(sql_to_err_response)?;

    let solution_score = sqlx::query_scalar::<_, NumNodes>(
        r#"SELECT score FROM Solution WHERE solution_hash=UNHEX(?)"#,
    )
    .bind(&solution_hash)
    .fetch_one(&mut *tx)
    .await
    .map_err(sql_to_err_response)?;

    insert_solver_run_entry(&mut tx, &request).await?;
    insert_valid_solution_entry(&mut tx, &request, &solution_hash, solution_score).await?;

    tx.commit().await.map_err(sql_to_err_response)?;

    let note_response = serde_json::json!({"status": "success", "solution_hash": solution_hash});
    Ok(Json(note_response))
}

async fn handle_invalid_solution(
    app_data: Arc<AppState>,
    request: SolutionUploadRequest,
    result_type: SolverResultType,
) -> HandlerResult<impl IntoResponse> {
    debug!("Handling upload of invalid solution");

    let mut tx = app_data.db().begin().await.map_err(sql_to_err_response)?;

    insert_solver_run_entry(&mut tx, &request).await?;
    insert_invalid_solution_entry(&mut tx, &request, result_type).await?;

    tx.commit().await.map_err(sql_to_err_response)?;

    let note_response = serde_json::json!({"status": "success"});
    Ok(Json(note_response))
}

pub async fn solution_upload_handler(
    State(app_state): State<Arc<AppState>>,
    Json(mut request): Json<SolutionUploadRequest>,
) -> HandlerResult<impl IntoResponse> {
    let result = std::mem::replace(&mut request.result, SolverResult::Empty);
    let result_type = result.result_type().unwrap();

    Ok(match result {
        SolverResult::Valid {
            data: solution_data,
        } => handle_valid_new_solution(app_state, request, solution_data)
            .await?
            .into_response(),
        SolverResult::ValidCached { hash } => {
            handle_valid_cached_solution(app_state, request, hash)
                .await?
                .into_response()
        }
        SolverResult::Infeasible
        | SolverResult::SyntaxError
        | SolverResult::Timeout
        | SolverResult::NonCompetitive => handle_invalid_solution(app_state, request, result_type)
            .await?
            .into_response(),
        SolverResult::Empty => return bad_request_json!("Empty solution result"),
    })
}

#[cfg(test)]
mod test {
    use tracing_test::traced_test;

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

    macro_rules! test {
        ($pool:expr, $request:expr, $success:expr) => {{
            let state = Arc::new(AppState::new($pool));

            let response =
                super::solution_upload_handler(State(state.clone()), Json($request)).await;

            if ($success) {
                let response = response.unwrap().into_response();
                assert!(response.status().is_success(), "{:?}", response);
            } else {
                assert!(response.is_err());
            }
        }};
    }

    #[sqlx::test(fixtures("instances"))]
    #[traced_test]
    async fn solution_upload_single_new_data(pool: DbPool) -> sqlx::Result<()> {
        test!(
            pool,
            SolutionUploadRequest {
                instance_id: 2,
                run_uuid: uuid::Uuid::new_v4(),
                solver_uuid: None,
                seconds_computed: 1.0,

                result: SolverResult::Valid {
                    data: vec![1 as Node, 2],
                },
            },
            true
        );
        Ok(())
    }

    #[sqlx::test(fixtures("instances"))]
    #[traced_test]
    async fn solution_upload_single_cached_data(pool: DbPool) -> sqlx::Result<()> {
        // upload WITH data, to ensure it's cached
        test!(
            pool.clone(),
            SolutionUploadRequest {
                instance_id: 2,
                run_uuid: uuid::Uuid::new_v4(),
                solver_uuid: None,
                seconds_computed: 1.0,

                result: SolverResult::Valid {
                    data: vec![1 as Node, 2],
                },
            },
            true
        );

        let hash = sqlx::query_scalar::<_, String>(r"SELECT HEX(hash) FROM SolutionData")
            .fetch_one(&pool)
            .await
            .unwrap();

        let count_before = sqlx::query_scalar::<_, i32>(
            r"SELECT COUNT(*) FROM Solution WHERE solution_hash=UNHEX(?)",
        )
        .bind(&hash)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(count_before, 1);

        test!(
            pool.clone(),
            SolutionUploadRequest {
                instance_id: 2,
                run_uuid: uuid::Uuid::new_v4(),
                solver_uuid: None,
                seconds_computed: 1.0,

                result: SolverResult::ValidCached { hash: hash.clone() }
            },
            true
        );

        let count_after = sqlx::query_scalar::<_, i32>(
            r"SELECT COUNT(*) FROM Solution WHERE solution_hash=UNHEX(?)",
        )
        .bind(&hash)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(count_after, count_before + 1);

        Ok(())
    }

    #[sqlx::test(fixtures("instances"))]
    async fn solution_upload_single_infeasible(pool: DbPool) -> sqlx::Result<()> {
        test!(
            pool,
            SolutionUploadRequest {
                instance_id: 2,
                run_uuid: uuid::Uuid::new_v4(),
                solver_uuid: None,
                seconds_computed: 1.0,

                result: SolverResult::Infeasible,
            },
            true
        );
        Ok(())
    }

    #[sqlx::test(fixtures("instances"))]
    async fn solution_upload_duplicate_upload(pool: DbPool) -> sqlx::Result<()> {
        let run_uuid = uuid::Uuid::new_v4();
        let solver_uuid = uuid::Uuid::new_v4();

        test!(
            pool.clone(),
            SolutionUploadRequest {
                instance_id: 2,
                run_uuid,
                solver_uuid: Some(solver_uuid),
                seconds_computed: 1.0,
                result: SolverResult::Valid {
                    data: vec![1 as Node, 2]
                },
            },
            true
        );

        test!(
            pool.clone(),
            SolutionUploadRequest {
                instance_id: 2,
                run_uuid,
                solver_uuid: Some(solver_uuid),
                seconds_computed: 1.0,
                result: SolverResult::Valid {
                    data: vec![2 as Node],
                }
            },
            false // there's already a solution for this run_uuid and instance_id
        );

        Ok(())
    }
}
