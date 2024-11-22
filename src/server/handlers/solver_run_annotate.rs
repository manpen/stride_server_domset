use super::common::*;
use sqlx::QueryBuilder;
use uuid::Uuid;

#[derive(Clone, Deserialize, Serialize, Debug, Default)]
pub struct FilterOptions {
    solver: Uuid,
    run: Uuid,

    #[serde(default)]
    name: Option<String>,

    #[serde(default)]
    description: Option<String>,

    #[serde(default)]
    hide: Option<bool>,
}

pub async fn solver_run_annotate_handler(
    opts: Option<Query<FilterOptions>>,
    State(app_data): State<Arc<AppState>>,
) -> HandlerResult<impl IntoResponse> {
    let opts = opts.unwrap_or_default();

    let mut builder = QueryBuilder::new("UPDATE SolverRun SET ");

    let mut first_entry = true;

    if let Some(name) = &opts.name {
        let name = name.trim();
        if name.is_empty() {
            return error_bad_request!("Name must not be empty");
        }
        if !first_entry {
            builder.push(", ");
        }
        builder.push(" name = ");
        builder.push_bind(name);
        first_entry = false;
    }

    if let Some(description) = &opts.description {
        let description = description.trim();

        if !first_entry {
            builder.push(", ");
        }
        builder.push(" description = ");
        builder.push_bind(description);
        first_entry = false;
    }

    if let Some(hide) = opts.hide {
        if !first_entry {
            builder.push(", ");
        }
        builder.push(" hide = ");
        builder.push_bind(hide);
        first_entry = false;
    }

    if first_entry {
        return error_bad_request!("No fields to update");
    }

    builder.push(" WHERE solver_uuid = UNHEX(");
    builder.push_bind(opts.solver.simple().to_string());
    builder.push(") AND run_uuid = UNHEX(");
    builder.push_bind(opts.run.simple().to_string());
    builder.push(") LIMIT 1");

    builder.build().execute(app_data.db()).await?;

    Ok(Json(serde_json::json!({"status": "success"})))
}
