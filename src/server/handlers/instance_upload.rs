use sha2::{Digest, Sha256};

use super::common::*;

use crate::pace::{
    graph::*, instance_reader::PaceReader, instance_writer::pace_writer, PROBLEM_ID,
};

#[derive(Debug, Deserialize, Serialize)]
pub struct InstanceUploadRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub submitted_by: Option<String>,
    pub tags: Option<Vec<String>>,
    pub ignore_header: Option<bool>,
    pub data: String,
}

fn normalize_dimacs(
    data: &str,
    check_header: bool,
) -> HandlerResult<(NumNodes, NumEdges, String, String)> {
    let pace_reader = PaceReader::try_new(data.as_bytes()).map_err(debug_to_err_response)?;
    let num_nodes_per_header = pace_reader.number_of_nodes() as usize;
    let num_edges_per_header = pace_reader.number_of_edges() as usize;

    let mut edges = Vec::with_capacity(num_edges_per_header);
    for edge in pace_reader {
        match edge {
            Ok(edge) => {
                let edge = edge.normalized();
                if check_header && edge.max_node() >= num_nodes_per_header as Node {
                    return bad_request_json!("Edge contains node id that is larger than the number of nodes in the header");
                }

                edges.push(edge.normalized())
            }
            Err(e) => return Err(debug_to_err_response(e)),
        }
    }

    edges.sort();
    edges.dedup();

    if check_header && edges.len() != num_edges_per_header {
        return bad_request_json!(
            "Number of edges after deduplication does not match the number of edges in the header"
        );
    }

    // find all nodes that are used in the edges
    let mut used_nodes = vec![0 as Node; num_nodes_per_header];
    for e in &edges {
        used_nodes[e.0 as usize] = 1;
        used_nodes[e.1 as usize] = 1;
    }

    // compute prefix sum to compress nodes ids
    {
        let mut sum = 0;
        for x in used_nodes.iter_mut() {
            let tmp = *x;
            *x = sum;
            sum += tmp;
        }
    }

    // rewrite edges with compressed node ids
    for e in &mut edges {
        e.0 = used_nodes[e.0 as usize];
        e.1 = used_nodes[e.1 as usize];
    }

    let mut normalized_data: Vec<u8> = Vec::with_capacity(data.len());
    let (num_nodes, num_edges) = pace_writer(&mut normalized_data, PROBLEM_ID, edges.into_iter())
        .map_err(debug_to_err_response)?;

    let normalized_data = String::from_utf8(normalized_data).map_err(debug_to_err_response)?;
    let hash = {
        let mut hasher = Sha256::new();
        hasher.update(normalized_data.as_bytes());
        format!("{:x}", hasher.finalize())
    };
    Ok((num_nodes, num_edges, hash, normalized_data))
}

pub async fn instance_upload_handler(
    State(data): State<Arc<AppState>>,
    Json(body): Json<InstanceUploadRequest>,
) -> HandlerResult<impl IntoResponse> {
    let (num_nodes, num_edges, hash, normalized_data) =
        normalize_dimacs(&body.data, !body.ignore_header.unwrap_or(false))?;

    // we need to insert two rows and use a transaction for that
    let mut tx = data.db().begin().await.map_err(sql_to_err_response)?;

    sqlx::query(r#"INSERT INTO InstanceData (hash, data) VALUES (?, ?)"#)
        .bind(&hash)
        .bind(&normalized_data)
        .execute(&mut *tx)
        .await
        .map_err(sql_to_err_response)?;

    // create instance entry
    let instance_id = sqlx::query(r#"INSERT INTO Instance (data_hash,nodes,edges,name,description,submitted_by) VALUES (?, ?, ?, ?, ?, ?)"#)
        .bind(&hash)
        .bind(num_nodes)
        .bind(num_edges)
        .bind(body.name)
        .bind(body.description)
        .bind(body.submitted_by)
        .execute(&mut *tx)
        .await
        .map_err(sql_to_err_response)?.last_insert_id();

    for tag in body.tags.as_ref().unwrap_or(&Vec::new()) {
        sqlx::query(r#"INSERT INTO InstanceTag (instance_iid,tag_tid) VALUES (?, (SELECT tid FROM Tag WHERE name=? LIMIT 1))"#)
            .bind(instance_id)
            .bind(tag)
            .execute(&mut *tx)
            .await
            .map_err(sql_to_err_response)?;
    }

    tx.commit().await.map_err(sql_to_err_response)?;

    let note_response = serde_json::json!({"status": "success", "instance_id": instance_id});
    Ok(Json(note_response))
}
