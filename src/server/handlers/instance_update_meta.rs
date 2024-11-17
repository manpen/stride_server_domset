use super::common::*;
use paste::paste;
use sqlx::QueryBuilder;

#[derive(Debug, Deserialize, Default)]
#[allow(non_snake_case)]
pub struct UpdateRequest {
    iid: i32,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    min_deg: Option<u32>,
    #[serde(default)]
    max_deg: Option<u32>,
    #[serde(default)]
    num_ccs: Option<u32>,
    #[serde(default)]
    nodes_largest_cc: Option<u32>,
    #[serde(default)]
    diameter: Option<u32>,
    #[serde(default)]
    tree_width: Option<u32>,
    #[serde(default)]
    planar: Option<bool>,
}

async fn check_params(app_data: &Arc<AppState>, body: &UpdateRequest) -> HandlerResult<()> {
    if let Some(name) = body.name.as_ref() {
        if name.is_empty() {
            return error_bad_request!("Name cannot be empty");
        }
    }

    if let Some(description) = body.description.as_ref() {
        if description.is_empty() {
            return error_bad_request!("Description cannot be empty");
        }
    }

    let nodes: u32 = sqlx::query_scalar(r#"SELECT nodes FROM Instance WHERE iid = ?"#)
        .bind(body.iid)
        .fetch_one(app_data.db())
        .await?;

    if body.min_deg.unwrap_or_default() >= nodes {
        return error_bad_request!("Minimum degree cannot be greater than number of nodes");
    }

    if body.max_deg.unwrap_or_default() >= nodes {
        return error_bad_request!("Maximum degree cannot be greater than number of nodes");
    }

    if body.num_ccs.unwrap_or_default() > nodes {
        return error_bad_request!(
            "Number of connected components cannot be greater than number of nodes"
        );
    }

    if body.nodes_largest_cc.unwrap_or_default() > nodes {
        return error_bad_request!(
            "Number of nodes in largest connected component cannot be greater than number of nodes"
        );
    }

    if body.num_ccs.unwrap_or_default() + body.nodes_largest_cc.unwrap_or_default() > nodes + 1 {
        return error_bad_request!(
            "Number of connected components and number of nodes in largest connected component cannot be greater than number of nodes"
        );
    }

    if body.diameter.unwrap_or_default() >= nodes {
        return error_bad_request!("Diameter cannot be greater than number of nodes");
    }

    if body.tree_width.unwrap_or_default() > nodes {
        return error_bad_request!("Tree width cannot be greater than number of nodes");
    }

    Ok(())
}

async fn update_record(app_data: &Arc<AppState>, body: &UpdateRequest) -> HandlerResult<()> {
    let mut builder = QueryBuilder::new("UPDATE Instance SET ");
    let mut any_is_set = false;

    macro_rules! process {
        ($field:ident) => {
            paste! {
                if let Some(val) = &body.$field {
                    if any_is_set {
                        builder.push(", ");
                    }
                    builder.push(stringify!($field =));
                    builder.push_bind(val);
                    any_is_set = true;
                }
            }
        };
    }

    process!(name);
    process!(description);
    process!(min_deg);
    process!(max_deg);
    process!(num_ccs);
    process!(nodes_largest_cc);
    process!(diameter);
    process!(tree_width);
    process!(planar);

    if !any_is_set {
        return error_bad_request!("No fields to update");
    }

    builder.push(" WHERE iid = ");
    builder.push_bind(body.iid);

    let result = builder.build().execute(app_data.db()).await?;

    Ok(())
}

pub async fn instance_update_meta_handler(
    State(data): State<Arc<AppState>>,
    Json(body): Json<UpdateRequest>,
) -> HandlerResult<impl IntoResponse> {
    check_params(&data, &body).await?;
    update_record(&data, &body).await?;

    Ok(Json(serde_json::json!({"status": "success"})))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::server::app_state::DbPool;

    macro_rules! test_field {
        ($name:ident, $value:expr, $t : ty) => {
            paste! {
                #[sqlx::test(fixtures("instances"))]
                async fn [<test_ $name>](pool: DbPool) -> sqlx::Result<()> {
                    let state = Arc::new(AppState::new(pool));

                    let iid = 1;
                    let request = UpdateRequest {iid, [<$name>]: Some($value), ..Default::default() };

                    let response = super::instance_update_meta_handler(State(state.clone()), Json(request))
                        .await
                        .unwrap()
                        .into_response();

                    assert!(response.status().is_success());

                    let updated_value : $t = sqlx::query_scalar::<_, $t>(stringify!(SELECT $name FROM Instance WHERE iid = ?))
                        .bind(iid)
                        .fetch_one(state.db())
                        .await?;

                    assert_eq!($value, updated_value);

                    Ok(())
                }
            }
        };
    }

    test_field!(min_deg, 2, u32);
    test_field!(max_deg, 3, u32);
    test_field!(num_ccs, 4, u32);
    test_field!(nodes_largest_cc, 5, u32);
    test_field!(diameter, 6, u32);
    test_field!(tree_width, 7, u32);
    test_field!(planar, true, bool);

    #[sqlx::test(fixtures("instances"))]
    async fn test_multiple(pool: DbPool) -> sqlx::Result<()> {
        let state = Arc::new(AppState::new(pool));

        let iid = 1;
        let value = String::from("Hi");

        let request = UpdateRequest {
            iid,
            name: Some(value.clone()),
            min_deg: Some(2),
            max_deg: Some(3),
            ..Default::default()
        };

        let response = super::instance_update_meta_handler(State(state.clone()), Json(request))
            .await
            .unwrap()
            .into_response();

        assert!(response.status().is_success());

        Ok(())
    }
}
