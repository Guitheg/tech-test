use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use serde_json::{json, Value};

use crate::server::app::AppState;

pub(crate) async fn create_restapi(state: Arc<dyn AppState>) -> Router {
    Router::new()
        .route("/health", get(handler_health))
        .route("/data", get(handler_data))
        .fallback(handler_404)
        .with_state(state)
}


pub async fn handler_data(State(state): State<Arc<dyn AppState>>) -> Json<Value> {
    let last_value = state.get_last_value();
    println!("📃 Requesting data... Sending {:?}...", last_value);
    Json(json!({ "data": last_value }))
}

async fn handler_404() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "The requested resource was not found")
}

async fn handler_health() -> StatusCode {
    StatusCode::OK
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use alloy::transports::http::reqwest::Body;
    use rstest::rstest;

    use axum::http::Request;
    use serde_json::Value;
    use crate::server::{restapi::create_restapi, app::AppStateMock};
    use tower::util::ServiceExt;
    use axum::http::StatusCode;

    #[tokio::test]
    #[rstest]
    #[case(Some(42))]
    #[case(None)]
    async fn response_success(#[case] value: Option<u128>) {
        use axum::body::to_bytes;
        use serde_json::from_str;

        let app_state = Arc::new(AppStateMock::new(value));
        let restapi = create_restapi(app_state).await;
        
        let response = restapi
            .oneshot(Request::builder().uri("/data").body(Body::default()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body_bytes = to_bytes(response.into_body(), 1024).await.unwrap();
        let body_as_str = String::from_utf8(body_bytes.to_vec()).unwrap();
        let body: Value = from_str(&body_as_str).unwrap();
        match value {
            None => assert!(body["data"].is_null()),
            Some(v) => {
                let number = body["data"].as_number();
                assert!(number.is_some());
                if let Some(n) = number {
                    assert_eq!(v, n.as_u64().unwrap() as u128);
                }
            } 
        }
    }
}
