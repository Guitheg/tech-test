use std::{collections::HashMap, sync::{Arc, Mutex}};
use crate::common::transaction::Transaction;
use starknet::{core::types::{BlockId, EventFilter, Felt}, providers::{jsonrpc::HttpTransport, JsonRpcClient, Provider, Url}};
use tokio::sync::mpsc;
use crate::common::spot_entry::{felt_to_u128, felt_to_utf8_str, SpotEntry};

fn period_hour(timestamp: u64) -> u64 {
    timestamp.div_euclid(3600)
}

pub(crate) struct TwapStorage {
    keys: Arc<Mutex<Vec<u64>>>,
    map: Arc<Mutex<HashMap<u64, TwapPeriod>>>,
    current_hour: u64,
}

impl TwapStorage {
    pub(crate) fn new() -> Self {
        Self {
            keys: Arc::new(Mutex::new(Vec::new())),
            map: Arc::new(Mutex::new(HashMap::new())),
            current_hour: 0,
        }
    }

    pub(crate) fn process_twap(&mut self, transaction: Transaction) {
        let period = period_hour(transaction.spot_entry.timestamp);
        let mut keys = self.keys.lock().unwrap();
        let mut map = self.map.lock().unwrap();

        self.current_hour = period;
            
        if !map.contains_key(&period) {
            keys.push(period);
        }
        map.entry(period).or_insert(TwapPeriod::new());

        if let Some(twap_period) = map.get_mut(&period) {
            twap_period.update(transaction.spot_entry.timestamp, transaction.spot_entry.price);
        }
    }

    pub(crate) fn get(&self) -> Result<u128, String> {
        let keys = self.keys.lock().unwrap();
        let last_key = match keys.last() {
            Some(k) => *k,
            None => return Err("No data found".to_string()),
        };

        let map = self.map.lock().unwrap();
        if let Some(twap_period) = map.get(&last_key) {
            twap_period.twap()
        } else {
            Err("No period found".to_string())
        }
    }
}

pub(crate) async fn twap_process_forever(rpc_url: &str, twap_storage: &Arc<Mutex<TwapStorage>>, contract_addr: &str, pair_id: &str) {
    let contract_address = String::from(contract_addr);
    let target_pair_id = String::from(pair_id);

    let (sender, mut receiver) = mpsc::channel::<Transaction>(64);

    let provider = JsonRpcClient::new(HttpTransport::new(
        Url::parse(rpc_url).unwrap()
    ));

    let producer = tokio::spawn(async move {
        println!("Processing...");
        loop {
            let block_number = provider.block_number().await.expect("Failed to get block number");
            let filter = EventFilter {
                from_block: Some(BlockId::Number(block_number - 1)),
                to_block: Some(BlockId::Number(block_number)),
                address: Some(Felt::from_hex_unchecked(&contract_address)),
                keys: None
            };
            match provider.get_events(filter, None, 64).await {
                Ok(events) => {
                    for event in events.events {
                        let pair_id = felt_to_utf8_str(event.data[4]).unwrap();
                        
                        if pair_id != target_pair_id {
                            print!(".");
                            continue;
                        }

                        let entry = SpotEntry {
                            timestamp: event.data[0].to_biguint().to_u64_digits()[0],
                            source: felt_to_utf8_str(event.data[1]).unwrap(),
                            publisher: felt_to_utf8_str(event.data[2]).unwrap(),
                            price: felt_to_u128(event.data[3]).unwrap(),
                            pair_id,
                            volume: felt_to_u128(event.data[5]).unwrap(),
                        };
            
                        let transaction = Transaction {
                            block_number: event.block_number.unwrap(),
                            transaction_hash: event.transaction_hash.to_fixed_hex_string(),
                            from_address: event.from_address.to_fixed_hex_string(),
                            spot_entry: entry
                        };
                        println!("{}", transaction);
                        
                        sender.send(transaction).await.unwrap()
                    }
                }
                Err(e) => eprintln!("Failed to fetch events {e}"),
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    });


    while let Some(entry) = receiver.recv().await {
        {
            let mut twap_storage = twap_storage.lock().unwrap();
            twap_storage.process_twap(entry);
        }
    }
    let _ = producer.await;
}


struct TwapPeriod {
    total_weighted_price: u128,
    total_time: u64,
    last_price: u128,
    last_timestamp: u64,
    initialized: bool,
}

impl TwapPeriod {
    fn new() -> Self {
        TwapPeriod {
            total_weighted_price: 0,
            total_time: 0,
            last_price: 0,
            last_timestamp: 0,
            initialized: false,
        }
    }

    fn update(&mut self, timestamp: u64, price: u128) {
        if !self.initialized {
            self.last_timestamp = timestamp;
            self.last_price = price;
            self.initialized = true;
        } else {
            let interval = timestamp.saturating_sub(self.last_timestamp);
            if interval > 0 {
                self.total_weighted_price += self.last_price * (interval as u128);
                self.total_time += interval;
            }
            self.last_timestamp = timestamp;
            self.last_price = price;
        }
    }

    fn twap(&self) -> Result<u128, String> {
        if self.total_time == 0 {
            return Err("Nothing computed yet for this hour".to_string());
        }
        Ok(self.total_weighted_price / (self.total_time as u128))
    }
}
