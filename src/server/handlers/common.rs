use crate::server::app_error::AppError;
pub use crate::server::app_state::AppState;

pub use axum::extract::{Query, State};
pub use axum::{response::IntoResponse, Json};
pub use serde::{Deserialize, Serialize};
pub use std::sync::Arc;

pub type HandlerResult<T> = Result<T, AppError>;

#[macro_export]
macro_rules! error_bad_request {
    ($message:expr) => {
        Err(anyhow::anyhow!($message).into())
    };
}
pub use error_bad_request;

#[cfg(test)]
pub mod test {
    use std::sync::Arc;

    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use crate::server::{app_state::DbPool, router::create_router};

    use super::AppState;

    pub async fn unwrap_oneshot_request(pool: DbPool, request: Request<Body>) -> String {
        let app = create_router(Arc::new(AppState::new(pool)));

        let response = app.oneshot(request).await.expect("Failed to call endpoint");

        assert_eq!(response.status(), StatusCode::OK);
        String::from_utf8(
            response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .to_vec(),
        )
        .unwrap()
    }
}
