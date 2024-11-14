use super::common::*;

#[derive(Debug, Deserialize, Serialize)]
#[allow(non_snake_case)]
struct InstanceModel {
    iid: i32,

    nodes: u32,
    edges: u32,

    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    submitted_by: Option<String>,

    data_hash: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Vec<u8>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    max_compute_seconds: Option<f64>,
}

pub async fn instance_fetch_unsolved_handler(
    State(data): State<Arc<AppState>>,
) -> HandlerResult<impl IntoResponse> {
    // attempt to fetch instance from database
    let mut instance = sqlx::query_as!(
        InstanceModel,
        r#"SELECT i.iid, i.nodes, i.edges, i.name, i.description, i.submitted_by, d.data, d.hash as data_hash, MAX(s.seconds_computed) as max_compute_seconds FROM `Instance` i 
            JOIN `InstanceData` d ON i.data_did = d.did
            LEFT JOIN `Solution` s ON i.iid = s.instance_iid
            WHERE s.solution_hash IS NULL
            GROUP BY i.iid
            ORDER BY i.nodes ASC
            LIMIT 1"#,
    )
    .fetch_one(data.db())
    .await
    .map_err(sql_to_err_response)?;

    let data = String::from_utf8(instance.data.take().unwrap()).map_err(debug_to_err_response)?;

    let json_response = serde_json::json!({
        "status": "success",
        "instance": instance,
        "data": data,
    });

    Ok(Json(json_response))
}
