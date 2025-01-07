use std::sync::Arc;

use std::time::{SystemTime, UNIX_EPOCH};

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use serde_json::{json, Value};

use crate::server::app::AppState;
use crate::server::signing::get_signature;

pub(crate) async fn create_restapi(state: Arc<dyn AppState>) -> Router {
    Router::new()
        .route("/health", get(handler_health))
        .route("/data", get(handler_data))
        .fallback(handler_404)
        .with_state(state)
}

fn cat_u8_16_n_u8_8_to_u8_24(u_16: [u8; 16], u_8: [u8 ;8]) -> [u8; 24] {
    let mut res = [0u8; 24];
    res[..16].copy_from_slice(&u_16);
    res[16..].copy_from_slice(&u_8);
    res
}

pub async fn handler_data(State(state): State<Arc<dyn AppState>>) -> Json<Value> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let last_value = state.get_last_value();
    let value_as_bytes = if let Some(value) = last_value {
        value.to_ne_bytes()
    } else {
        [0u8; 16]
    };
    let full_message = cat_u8_16_n_u8_8_to_u8_24(value_as_bytes, now.to_ne_bytes());
    let signature = get_signature(&full_message, state.passkey());
    println!("📃 Requesting data... Sending {:?} ({now}) [{signature}]", last_value);

    let json_data = json!({
        "data": last_value,
        "now": now,
        "signature": signature,
        "identifier": state.identifier()
    });
    
    Json(json_data)
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
