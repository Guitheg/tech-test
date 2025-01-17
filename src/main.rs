
mod events;
mod metrics;
mod server;

use server::app::server_run_forever;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    id: String,

    #[arg(short, long)]
    tcp_addr: String,

    #[arg(short, long)]
    port: String,

    #[arg(short, long)]
    api_key: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let rpc_url = "https://starknet-sepolia.infura.io/v3";
    let contract_addr = "0x036031daa264c24520b11d93af622c848b2499b66b41d611bac95e13cfca131a";

    server_run_forever(
        args.tcp_addr.to_string(),
        args.port.to_string(),
        args.id.to_string(),
        rpc_url.to_string(),
        args.api_key.to_string(),
        contract_addr.to_string(),
        true
    ).await
}