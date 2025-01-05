use std::{collections::HashMap, sync::Mutex};
use crate::metrics::{twap::TwapInput, Metric};

use super::twap::{TwapMetric, TwapValue};

pub(crate) trait MetricStorage<KeyType, StorageType> {
    fn get(&self, key: KeyType) -> Option<StorageType>;
    fn last(&self) -> Option<StorageType>;
    fn insert(&self, key: KeyType, value: StorageType);
}

pub(crate) struct HashMapStorage {
    twap_storage: Mutex<HashMap<u64, u128>>,
    twap: Mutex<TwapMetric>,
    current: Mutex<Option<TwapValue>>
}


impl HashMapStorage {
    pub(crate) fn new() -> Self {
        Self {
            twap_storage: Mutex::new(HashMap::new()),
            twap: Mutex::new(TwapMetric::new(3600)),
            current: Mutex::new(None)
        }
    }
}

impl MetricStorage<u64, u128> for HashMapStorage {
    fn get(&self, key: u64) -> Option<u128> {
        match self.twap_storage.lock() {
            Ok(value) => {
                value.get(&key).copied()
            },
            Err(e) => {
                eprintln!("HashMapStorage Error while locking for 'get': {}", e);
                None
            }
        }
    }

    fn last(&self) -> Option<u128> {
        self.current.lock().unwrap().as_ref().map(|value| value.value)
    }

    fn insert(&self, key: u64, value: u128) {
        match self.twap_storage.lock() {
            Ok(mut guard) => {
                let mut twap = self.twap.lock().unwrap();
                let mut current = self.current.lock().unwrap();
                current.replace(TwapValue { timestamp: key, value });

                if let Some(new_metric) = twap.update(TwapInput{timestamp: key, price: value}).unwrap() {
                    // A period has been complete, so we add the twap value to the storage.
                    guard.insert(new_metric.timestamp, new_metric.value);
                    println!("📥 [{}] One hour complete, adding to the storage : {}", new_metric.timestamp, new_metric.value);
                }
            }
            Err(e) => {
                eprintln!("HashMapStorage Error while locking for 'inser': {}", e);
            }
        }
    }
}



#[cfg(test)]
mod tests {
    use rstest::rstest;
    use std::sync::{Arc, Mutex};

    use crate::metrics::storage::{HashMapStorage, MetricStorage};

    #[rstest]
    #[allow(non_snake_case)]
    fn HashMapStorage_get_insert_concurency() {
        let storage = Arc::new(HashMapStorage::new());
        let mut threads = vec![];

        let stack = Arc::new(Mutex::new((1..4000).rev().collect::<Vec<u64>>()));

        for _ in 0u128..4u128 {
            let s = Arc::clone(&storage);
            let stack_clone = Arc::clone(&stack);
            threads.push(std::thread::spawn(move || {
                for i in 0u128..1000u128 {
                    if let Some(val) = stack_clone.lock().unwrap().pop() {
                        s.insert(val, i);
                    }
                }
            }));
        }

        for thread in threads {
            thread.join().unwrap();
        }
        
    }

    #[rstest]
    #[allow(non_snake_case)]
    fn HashMapStorage_last() {
        let storage = HashMapStorage::new();
        storage.insert(1, 10);
        assert_eq!(Some(10), storage.last());
        storage.insert(2, 20);
        assert_eq!(Some(20), storage.last());
        storage.insert(3, 30);
        assert_eq!(Some(30), storage.last());
        storage.insert(4, 40);
        assert_eq!(Some(40), storage.last());
        storage.insert(5, 50);
        assert_eq!(Some(50), storage.last());
        storage.insert(6, 60);
        assert_eq!(Some(60), storage.last());
        storage.insert(7, 70);
        assert_eq!(Some(70), storage.last());
    }
}

