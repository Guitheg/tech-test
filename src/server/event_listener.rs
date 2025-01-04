use crate::common::transaction::Transaction;
use starknet::{core::types::{BlockId, EventFilter, Felt}, providers::{jsonrpc::HttpTransport, JsonRpcClient, Provider, Url}};
use tokio::sync::mpsc::{self, Receiver};
use crate::common::spot_entry::{felt_to_u128, felt_to_utf8_str, SpotEntry};


pub(crate) async fn receive_event(
    rpc_url: &str,
    contract_addr: &str,
    pair_id: &str,
    max_iterations: Option<usize>,
    is_verbose: bool
) -> Option<Receiver<Transaction>> {
    if is_verbose {
        println!("Preparing to receive from {rpc_url} - {contract_addr}. Pair : {pair_id}");
    }
    let contract_address = String::from(contract_addr);
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

    if let Err(e) = provider.block_number().await {
        eprintln!("Failed to get block number - the contract address might be wrong : {e}");
        return None;
    };

    let producer = tokio::spawn(async move {
        if is_verbose {
            println!("Receiving event");
        }
        let mut iteration_count = 0;
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

            if let Some(limit) = max_iterations {
                if iteration_count >= limit {
                    if is_verbose {
                        println!("Reached max_iterations = {limit}");
                    }
                    break;
                }
                iteration_count += 1;
            }

            let block_number = provider.block_number().await.expect("Failed to get block number within loop");
            let filter = EventFilter {
                from_block: Some(BlockId::Number(block_number - 1)),
                to_block: Some(BlockId::Number(block_number)),
                address: Some(Felt::from_hex_unchecked(&contract_address)),
                keys: None
            };
            match provider.get_events(filter, None, 1).await {
                Ok(events) => {
                    for event in events.events {
                        let pair_id = felt_to_utf8_str(event.data[4]).unwrap();
                        
                        if pair_id != target_pair_id { continue; }

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
                            println!("{}", transaction);
                        }
                        
                        sender.send(transaction).await.unwrap()
                    }
                }
                Err(e) => {
                    eprintln!("Failed to fetch events {e}");
                }
            }
        }
    });

    if let Err(e) = producer.await {
        eprintln!("{e}");
        return None;
    }
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
