use crate::common::transaction::Transaction;
use starknet::{core::types::{BlockId, EventFilter, Felt}, providers::{jsonrpc::HttpTransport, JsonRpcClient, Provider, Url}};
use tokio::sync::mpsc::{self, Receiver, Sender};
use crate::common::spot_entry::{felt_to_u128, felt_to_utf8_str, SpotEntry};

const EVENT_HASH: &str = "0x280bb2099800026f90c334a3a23888ffe718a2920ffbbf4f44c6d3d5efb613c";

async fn read_and_send_events(
    sender: &Sender<Transaction>,
    provider: &JsonRpcClient<HttpTransport>,
    filter: EventFilter,
    number_of_blocks: u64,
    target_pair_id: String,
    is_verbose: bool
) {
    match provider.get_events(filter, None, number_of_blocks*50).await {
        Ok(events) => {
            for event in events.events {
                let pair_id = felt_to_utf8_str(event.data[4]).unwrap();
                let received_block = event.block_number.unwrap();
                
                if pair_id != target_pair_id {
                    println!("❔ [block:{received_block}] Did not received {target_pair_id} (but {pair_id})");
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

                if is_verbose {
                    println!("❕[block:{received_block}] Receive: {transaction}");
                }
                
                sender.send(transaction).await.unwrap()
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to fetch events {e}");
        }
    }
}

pub(crate) async fn receive_event(
    rpc_url: &str,
    contract_addr: &str,
    pair_id: &str,
    max_iterations: Option<usize>,
    is_verbose: bool,
) -> Option<Receiver<Transaction>> {
    if is_verbose {
        println!("✅ Preparing to receive {pair_id} from {rpc_url} -> {contract_addr}");
    }
    let n_previous_block_to_retrieve: u64 = 20;
    let contract_address = String::from(contract_addr);
    let contract_address_felt = match Felt::from_hex(&contract_address) {
        Ok(value) => value,
        Err(e) => {
            eprintln!("❌ {e}");
            return None;
        }
    };
    let target_pair_id = String::from(pair_id);

    let (sender, receiver) = mpsc::channel::<Transaction>(64);

    let provider = JsonRpcClient::new(HttpTransport::new(
        match Url::parse(rpc_url) {
            Ok(url) => url,
            Err(e) => {
                eprintln!("{e}");
                return None;
            }
        }
    ));
    
    match provider.block_number().await {
        Ok(block_number) => {
            match provider.get_class_hash_at(BlockId::Number(block_number), contract_address_felt).await {
                Ok(class_hash) => {
                    if class_hash == Felt::ZERO {
                        eprintln!("❌ The given contract address does not correspond to a deployed contract");
                        return None;
                    } else {
                        println!("✅ The class contract retrieve with success : {:?}", class_hash);
                    }
                }
                Err(e) => {
                    eprintln!("❌ Failed to get contract class - the contract address might be wrong : {e}");
                    return None;
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to get block number - the contract address might be wrong : {e}");
            return None;
        }
    };

    tokio::spawn(async move {
        let mut iteration_count = 0;
        let mut last_block_number: Option<u64> = None;

        let block_number = provider.block_number().await.expect("Failed to get block number within loop");
        let from_block = block_number - n_previous_block_to_retrieve;
        last_block_number.replace(block_number);
        if is_verbose {
            println!("🧊 Last block number retrieve n°{block_number}");
            println!("🔄 Retrieve previous events from the block n°{from_block}");
        }
        
        let filter = EventFilter {
            from_block: Some(BlockId::Number(from_block)),
            to_block: Some(BlockId::Number(block_number)),
            address: Some(contract_address_felt),
            keys: Some(vec![vec![Felt::from_hex_unchecked(EVENT_HASH)]])
        };

        read_and_send_events(&sender, &provider, filter,block_number - from_block, target_pair_id.clone(), is_verbose).await;

        loop {
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

            let block_number = provider.block_number().await.expect("Failed to get block number within loop");

            if let Some(limit) = max_iterations {
                if iteration_count >= limit {
                    if is_verbose {
                        println!("❌ Reached max_iterations = {limit}");
                    }
                    break;
                }
                iteration_count += 1;
            }

            if last_block_number != Some(block_number) {
                let filter = EventFilter {
                    from_block: Some(BlockId::Number(block_number)),
                    to_block: Some(BlockId::Number(block_number)),
                    address: Some(contract_address_felt),
                    keys: Some(vec![vec![Felt::from_hex_unchecked(EVENT_HASH)]])
                };
                read_and_send_events(&sender, &provider, filter,1, target_pair_id.clone(), is_verbose).await;
            }

            last_block_number.replace(block_number);
        }
    });

    Some(receiver)

}


#[cfg(test)]
mod tests {
    use rstest::{fixture, rstest};

    use super::receive_event;
    use std::env;

    const RPC_BASE_URL: &str = "https://starknet-sepolia.infura.io/v3";
    const CONTRACT_ADDR: &str = "0x036031daa264c24520b11d93af622c848b2499b66b41d611bac95e13cfca131a";
    const PAIR_ID: &str = "BTC/USD";
    
    #[fixture]
    fn rpc_url() -> String {
        let base_url = RPC_BASE_URL;
        let infura_api_key = env::var("INFURA_API_KEY").expect("INFURA_API_KEY env var must be set");
        format!("{}/{}", base_url, infura_api_key)
    }

    #[rstest]
    #[tokio::test]
    async fn receive_event_return_receiver_with_success(rpc_url: String) {
        assert!(receive_event(&rpc_url, CONTRACT_ADDR, PAIR_ID, Some(1), true).await.is_some());
    }


    #[tokio::test]
    #[rstest]
    #[case(true, "sdszaf")]
    #[case(false, CONTRACT_ADDR)]
    async fn receive_event_return_receiver_failed(
        #[case] is_rpc_url_ok: bool,
        #[case] contract_addr: &str,
        rpc_url: String
    ) {
        assert!(receive_event(if is_rpc_url_ok {&rpc_url} else {"skjd"}, contract_addr, PAIR_ID, Some(1), true).await.is_none());
    }
}
