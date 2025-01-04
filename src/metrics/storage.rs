use std::{collections::HashMap, sync::Mutex};

pub(crate) trait MetricStorage<KeyType, StorageType> {
    fn get(&self, key: KeyType) -> Option<StorageType>;
    fn insert(&self, key: KeyType, value: StorageType);
}

pub(crate) struct HashMapStorage {
    twap: Mutex<HashMap<u64, u128>>
}


impl HashMapStorage {
    pub(crate) fn new() -> Self {
        Self {
            twap: Mutex::new(HashMap::new())
        }
    }
}

impl MetricStorage<u64, u128> for HashMapStorage {
    fn get(&self, key: u64) -> Option<u128> {
        match self.twap.lock() {
            Ok(value) => {
                value.get(&key).copied()
            },
            Err(e) => {
                eprintln!("HashMapStorage Error while locking for 'get': {}", e);
                None
            }
        }
    }
    fn insert(&self, key: u64, value: u128) {
        match self.twap.lock() {
            Ok(mut guard) => {
                guard.insert(key, value);
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
    use std::sync::Arc;

    use crate::metrics::storage::{HashMapStorage, MetricStorage};

    #[rstest]
    #[allow(non_snake_case)]
    fn HashMapStorage_get_insert_concurency() {
        let storage = Arc::new(HashMapStorage::new());
        let mut threads = vec![];

        for t_id in 0..4 {
            let s = Arc::clone(&storage);
            threads.push(std::thread::spawn(move || {
                for i in 0..1000 {
                    s.insert((t_id * 1000) + i, i as u128);
                }
            }));
        }

        for thread in threads {
            thread.join().unwrap();
        }
        
        for t_id in 0..4 {
            for i in 0..1000 {
                assert_eq!(
                    storage.get((t_id * 1000) + i),
                    Some(i as u128)
                );
            }
        }
    }
}

