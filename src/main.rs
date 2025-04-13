mod utils;
use std::sync::{Arc, RwLock};

use clap::Parser;
use env_logger::Env;
use solana_client::nonblocking::rpc_client::RpcClient;

#[tokio::main]
async fn main() {
    env_logger::init_from_env(Env::default().default_filter_or("info"));
    let _args = Args::parse();
}

#[derive(Clone)]
struct Miner {
    pub keypair_filepath: Option<String>,
    pub priority_fee: Option<u64>,
    pub dynamic_fee_url: Option<String>,
    pub dynamic_fee: bool,
    pub rpc_client: Arc<RpcClient>,
    pub fee_payer_filepath: Option<String>,
    // pub solo_collecting_data: Arc<RwLock<Vec<SoloCollectingData>>>,
    // pub pool_collecting_data: Arc<std::sync::RwLock<Vec<PoolCollectingData>>>,
}

#[derive(Parser, Debug)]
#[command(about, version)]
struct Args {
    #[arg(
        long,
        value_name = "NETWORK_URL",
        help = "Network address of your RPC provider",
        default_value = "https://mainnetbeta-rpc.eclipse.xyz/",
        global = true
    )]
    rpc: Option<String>,
    #[clap(
        global = true,
        short = 'C',
        long = "config",
        id = "PATH",
        help = "Filepath to config file."
    )]
    config_file: Option<String>,
}
