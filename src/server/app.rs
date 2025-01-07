use std::sync::{Arc, Mutex};
use crate::events::listener::receive_event;
use crate::server::restapi::create_restapi;
use crate::{events::transaction, metrics::storage::MetricStorage};

use self::transaction::Transaction;

use crate::metrics::storage::HashMapStorage;


pub(crate) trait AppState: Send + Sync {
    fn get_last_value(&self) -> Option<u128>;
    fn update(&self, transaction: Transaction);
}

pub(crate) struct AppStateMock {
    value: Mutex<Option<u128>>
}
impl AppStateMock {
    pub(crate) fn new(value: Option<u128>) -> Self {
        Self {
            value: Mutex::new(value)
        }
    }
}
impl AppState for AppStateMock {
    fn get_last_value(&self) -> Option<u128> {
        let value = self.value.lock().unwrap();
        *value
    }
    fn update(&self, transaction: Transaction) {
        let mut value = self.value.lock().unwrap();
        value.replace(transaction.spot_entry.price);
    }
}

pub(crate) struct AppStateImpl {
    storage: Arc<HashMapStorage>
}
impl AppStateImpl {
    fn new() -> Self {
        Self {
            storage: Arc::new(HashMapStorage::new())
        }
    }
}
impl AppState for AppStateImpl {
    fn get_last_value(&self) -> Option<u128> {
        self.storage.last()
    }

    fn update(&self, transaction: Transaction) {
        self.storage.insert(transaction.spot_entry.timestamp, transaction.spot_entry.price);
    }
}

pub(crate) async fn server_run_forever(
    tcp_addr: String,
    port: String,
    pair_id: String,
    rpc_url: String,
    api_key: String,
    contract_addr: String,
    is_verbose: bool
) {
    if is_verbose {
        println!("⌛ Starting server");
    }
    let app_state = Arc::new(AppStateImpl::new());

    let app_state_restapi = Arc::clone(&app_state);
    let restapi_thread = tokio::spawn(async move {
        let app_api = create_restapi(app_state_restapi);
        let service = app_api.await.into_make_service();
        let server_addr = format!("{}:{}", tcp_addr, port);
        let listener = tokio::net::TcpListener::bind(server_addr.clone())
            .await
            .expect("❌ Failed to bind to address");
        if is_verbose {
            println!("✅ REST API has started on {}", server_addr.clone());
        }
        axum::serve(listener, service).await.unwrap();
    });

    let app_state_twap = Arc::clone(&app_state);
    let gather_twap_thread = tokio::spawn(async move {
        let mut receiver = receive_event(&format!("{}/{}", rpc_url, api_key), &contract_addr, &pair_id, None, is_verbose).await.unwrap();
        
        let twap_storage_thread = tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                while let Some(entry) = receiver.recv().await {
                    app_state_twap.update(entry);
                }
            }
        });

        let _ = twap_storage_thread.await;
    });

    let _ = restapi_thread.await;
    let _ = gather_twap_thread.await;
}
