use std::sync::{Arc, Mutex};
use axum::{Router, routing::get, extract::State};
use twap::TwapStorage;

use twap::twap_process_forever;

pub(crate) mod twap;
pub(crate) mod event_listener;
pub(crate) mod restapi;

pub(crate) trait AppState: Send + Sync {
    fn get_last_value(&self) -> Option<u128>;
}

pub(crate) struct AppStateMock {
    value: Option<u128>
}
impl AppStateMock {
    fn new(value: Option<u128>) -> Self {
        Self { value }
    }
}
impl AppState for AppStateMock {
    fn get_last_value(&self) -> Option<u128> {
        self.value
    }
}

pub(crate) struct AppStateImpl {
    storage: Arc<TwapStorage>
}
impl AppState for AppStateImpl {
    fn get_last_value(&self) -> Option<u128> {
        self.storage.get().ok()
    }
}




async fn get_data(State(twap_storage): State<Arc<Mutex<TwapStorage>>>) -> String {
    let twap = twap_storage.lock().unwrap();
    
    match twap.get() {
        Ok(value) => {
            println!("Send: {}", value);
            value.to_string()
        },
        Err(_) => {
            println!("No data");
            "No data".to_string()
        }
    }
}

async fn check_health() -> &'static str {
    "Not implemented yet"
}

pub(crate) async fn server_run_forever(tcp_addr: &str, port: &str, pair_id: &str, rpc_url: &str, api_key: &str, contract_addr: &str) {
    let twap_storage = Arc::new(Mutex::new(TwapStorage::new()));
    let router = Router::new()
        .route("/data", get(get_data))
        .route("/health", get(check_health))
        .with_state(twap_storage.clone());
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", tcp_addr, port)).await.expect("Failed to bind to address");
    tokio::spawn(async move {
        let router = router.into_make_service();
        axum::serve(listener, router).await.unwrap();
    });
    twap_process_forever(format!("{rpc_url}/{api_key}").as_str(), &twap_storage, contract_addr, pair_id).await;
}
