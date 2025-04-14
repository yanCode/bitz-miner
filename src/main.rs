mod utils;
use std::sync::{Arc, RwLock};

use clap::{Parser, Subcommand};
use env_logger::Env;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;

#[tokio::main]
async fn main() {
    env_logger::init_from_env(Env::default().default_filter_or("info"));
    let args = Args::parse();
    let cli_config = match &args.config_file {
        Some(config_file) => solana_cli_config::Config::load(config_file).unwrap_or_else(|_| {
            eprintln!("Failed to load config file: {}", config_file);
            std::process::exit(1);
        }),
        None => {
            println!(
                "No config file provided, using default config: {:?}",
                solana_cli_config::CONFIG_FILE.as_ref().unwrap()
            );
            solana_cli_config::Config::default()
        }
    };
    let cluster_url = args.rpc.unwrap_or(cli_config.json_rpc_url);
    let default_keypair = args.keypair.unwrap_or(cli_config.keypair_path.clone());
    let fee_payer_filepath = args.fee_payer.unwrap_or(default_keypair.clone());
    let rpc_client = RpcClient::new_with_commitment(cluster_url, CommitmentConfig::confirmed());
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
    #[arg(
        long,
        value_name = "KEYPAIR_FILEPATH",
        help = "Filepath to signer keypair. Base58 or Raw JSON.",
        default_value = "key.txt",
        global = true
    )]
    keypair: Option<String>,
    #[arg(
        long,
        value_name = "FEE_PAYER_FILEPATH",
        help = "Filepath to transaction fee payer keypair.",
        global = true
    )]
    fee_payer: Option<String>,
    #[arg(
        long,
        value_name = "MICROLAMPORTS",
        help = "Price to pay for compute units. If dynamic fees are enabled, this value will be used as the cap.",
        default_value = "1000",
        global = true
    )]
    priority_fee: Option<u64>,
    #[arg(
        long,
        value_name = "DYNAMIC_FEE_URL",
        help = "RPC URL to use for dynamic fee estimation.",
        global = true
    )]
    dynamic_fee_url: Option<String>,
    #[arg(long, help = "Enable dynamic priority fees", global = true)]
    dynamic_fee: bool,

    #[command(subcommand)]
    command: Commands,
}
#[derive(Subcommand, Debug)]
enum Commands {
    #[command(about = "Fetch your account details")]
    Account(AccountArgs),
}
#[derive(Parser, Debug)]
pub struct AccountArgs {
    #[arg(value_name = "ADDRESS", help = "The address to the account to fetch.")]
    pub address: Option<String>,

    #[arg(
        short,
        long,
        value_name = "PROOF_ADDRESS",
        help = "The address of the proof to fetch."
    )]
    pub proof: Option<String>,

    #[command(subcommand)]
    pub command: Option<AccountCommand>,
}

#[derive(Subcommand, Clone, Debug)]
pub enum AccountCommand {
    #[command(about = "Close an account and reclaim rent.")]
    Close(AccountCloseArgs),
}

#[derive(Parser, Clone, Debug)]
pub struct AccountCloseArgs {}
