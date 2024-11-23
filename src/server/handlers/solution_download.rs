use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::http::HeaderValue;
use axum::response::{IntoResponse, Response};
use uuid::Uuid;

use super::common::*;
use crate::pace::graph::Node;
use crate::pace::Solution;
use crate::server::app_state::DbPool;

#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Default, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ResponseFormat {
    #[default]
    Dimacs,
    Json,
}

#[derive(Clone, Deserialize, Serialize, Debug, Default)]
pub struct FilterOptions {
    // we require a legal combination of iid, solver, and run to avoid leaking solutions
    iid: u32,
    solver: Uuid,
    run: Uuid,

    #[serde(default)]
    format: ResponseFormat,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(non_snake_case)]
struct SolutionModel {
    score: u32,
    data: Vec<u8>,
}

async fn fetch_solution(opts: &FilterOptions, db_pool: &DbPool) -> HandlerResult<SolutionModel> {
    let run = opts.run.simple().to_string();
    let solver = opts.solver.simple().to_string();

    // attempt to fetch instance from database
    Ok(sqlx::query_as::<_, SolutionModel>(
        r#"SELECT 
            sd.data, s.score
           FROM SolutionData sd
           JOIN `Solution` s ON s.`solution_hash` = sd.`hash`
           JOIN `SolverRun` sr ON sr.`run_uuid` = s.`sr_uuid`
           WHERE s.`instance_iid` = ? AND sr.run_uuid = UNHEX(?) AND sr.solver_uuid = UNHEX(?) AND s.`score` IS NOT NULL
           LIMIT 1"#,
    )
    .bind(opts.iid)
    .bind(run)
    .bind(solver)
    .fetch_one(db_pool)
    .await?)
}

fn dimacs_response(
    opts: &FilterOptions,
    solution: SolutionModel,
) -> HandlerResult<impl IntoResponse> {
    let header_line = format!(
        "attachment; filename=\"sol_inst{}_score{}_run{}.sol\"",
        opts.iid, solution.score, opts.run
    );

    let content_disposition = HeaderValue::from_str(&header_line)?;

    let domset: Vec<Node> = serde_json::from_slice(solution.data.as_slice())?;
    let pace_solution = Solution::from_0indexed_vec(domset);

    let mut out_buffer: Vec<u8> = Vec::with_capacity(solution.data.len() + 100);
    pace_solution.write(&mut out_buffer)?;

    Ok((
        [
            (CONTENT_DISPOSITION, content_disposition),
            (CONTENT_TYPE, HeaderValue::from_static("text/plain")),
        ],
        out_buffer,
    ))
}

#[derive(Serialize, Deserialize, Debug)]
struct JsonResponse {
    status: String,
    solution: Vec<Node>,
}

fn json_response(buffer: Vec<u8>) -> HandlerResult<impl IntoResponse> {
    let mut solution: Vec<Node> = serde_json::from_slice(buffer.as_slice())?;
    for x in solution.iter_mut() {
        *x += 1;
    }

    Ok(Json(JsonResponse {
        status: String::from("ok"),
        solution,
    }))
}

pub async fn solution_download_handler(
    Query(opts): Query<FilterOptions>,
    State(app_data): State<Arc<AppState>>,
) -> HandlerResult<Response> {
    let solution = fetch_solution(&opts, app_data.db()).await?;

    Ok(match opts.format {
        ResponseFormat::Dimacs => dimacs_response(&opts, solution)?.into_response(),
        ResponseFormat::Json => json_response(solution.data)?.into_response(),
    })
}

#[cfg(test)]
mod tests {
    use http_body_util::BodyExt;

    use super::*;

    #[sqlx::test(fixtures("instances", "solutions"))]
    async fn solution_download_handler_json(db_pool: DbPool) -> sqlx::Result<()> {
        let opts = FilterOptions {
            iid: 1,
            solver: Uuid::parse_str("00000000-0000-0000-0002-000000000000").unwrap(),
            run: Uuid::parse_str("00000000-0000-0000-0001-000000000000").unwrap(),
            format: ResponseFormat::Json,
        };

        let state = Arc::new(AppState::new(db_pool.clone()));
        let resp = super::solution_download_handler(Query(opts), State(state))
            .await
            .unwrap();
        assert!(resp.status().is_success());

        let (_parts, body) = resp.into_parts();
        let bytes = body.collect().await.expect("body").to_bytes();
        let json_response: JsonResponse =
            serde_json::from_str(std::str::from_utf8(&bytes).unwrap()).unwrap();

        assert_eq!(json_response.solution, vec![1, 2, 4]);

        Ok(())
    }

    #[sqlx::test(fixtures("instances", "solutions"))]
    async fn solution_download_handler_dimacs(db_pool: DbPool) -> sqlx::Result<()> {
        let opts = FilterOptions {
            iid: 1,
            solver: Uuid::parse_str("00000000-0000-0000-0002-000000000000").unwrap(),
            run: Uuid::parse_str("00000000-0000-0000-0001-000000000000").unwrap(),
            format: ResponseFormat::Dimacs,
        };

        let state = Arc::new(AppState::new(db_pool.clone()));
        let resp = super::solution_download_handler(Query(opts), State(state))
            .await
            .unwrap();
        assert!(resp.status().is_success());

        let (_parts, body) = resp.into_parts();
        let bytes: Vec<u8> = body.collect().await.expect("body").to_bytes().into();
        dbg!(std::str::from_utf8(&bytes).unwrap());

        let solution = Solution::read(bytes.as_slice(), None).unwrap();
        assert_eq!(solution.take_1indexed_solution(), vec![1, 2, 4]);

        Ok(())
    }

    #[sqlx::test(fixtures("instances", "solutions"))]
    async fn solution_download_handler_test_solver_required(db_pool: DbPool) -> sqlx::Result<()> {
        let opts = FilterOptions {
            iid: 1,
            solver: Uuid::parse_str("00000000-0000-0000-0002-000000000001").unwrap(), // invalid solver/run combination!
            run: Uuid::parse_str("00000000-0000-0000-0001-000000000000").unwrap(),
            format: ResponseFormat::Dimacs,
        };

        let state = Arc::new(AppState::new(db_pool.clone()));

        let resp = super::solution_download_handler(Query(opts), State(state)).await;

        match resp {
            Ok(resp) if !resp.status().is_success() => return Ok(()),
            Err(_) => return Ok(()),
            _ => panic!("Expected error response"),
        }
    }
}
