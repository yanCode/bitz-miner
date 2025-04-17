mod args;
mod command;
mod constants;
mod send;
mod utils;
use std::sync::{Arc, RwLock};

use anyhow::Result;
use args::{AccountArgs, BenchmarkArgs, CollectArgs};
use clap::{Parser, Subcommand};
use env_logger::Env;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{commitment_config::CommitmentConfig, signature::Keypair};
use utils::{PoolCollectingData, SoloCollectingData};

#[tokio::main]
async fn main() -> Result<()> {
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
    let solo_collecting_data = Arc::new(RwLock::new(Vec::new()));
    let pool_collecting_data = Arc::new(RwLock::new(Vec::new()));
    let miner = Miner::new(
        Arc::new(rpc_client),
        args.priority_fee,
        Some(default_keypair),
        args.dynamic_fee_url,
        args.dynamic_fee,
        Some(fee_payer_filepath),
        solo_collecting_data,
        pool_collecting_data,
    );
    let _signer = miner.signer();
    match args.command {
        Commands::Benchmark(benchmark_args) => {
            miner.benchmark(benchmark_args).await?;
        }
        Commands::Collect(collect_args) => {
            miner.collect(collect_args).await?;
        }
        Commands::Account(account_args) => {
            miner.account(account_args).await?;
        } // _ => {}
    }
    Ok(())
}
//
#[allow(dead_code)]
#[derive(Clone)]
struct Miner {
    pub keypair_filepath: Option<String>,
    pub priority_fee: Option<u64>,
    pub dynamic_fee_url: Option<String>,
    pub dynamic_fee: bool,
    pub rpc_client: Arc<RpcClient>,
    pub fee_payer_filepath: Option<String>,
    pub solo_collecting_data: Arc<RwLock<Vec<SoloCollectingData>>>,
    pub pool_collecting_data: Arc<std::sync::RwLock<Vec<PoolCollectingData>>>,
}
impl Miner {
    pub fn new(
        rpc_client: Arc<RpcClient>,
        priority_fee: Option<u64>,
        keypair_filepath: Option<String>,
        dynamic_fee_url: Option<String>,
        dynamic_fee: bool,
        fee_payer_filepath: Option<String>,
        solo_collecting_data: Arc<RwLock<Vec<SoloCollectingData>>>,
        pool_collecting_data: Arc<RwLock<Vec<PoolCollectingData>>>,
    ) -> Self {
        Self {
            rpc_client,
            keypair_filepath,
            priority_fee,
            dynamic_fee_url,
            dynamic_fee,
            fee_payer_filepath,
            solo_collecting_data,
            pool_collecting_data,
        }
    }

    pub fn signer(&self) -> Keypair {
        match self.keypair_filepath.clone() {
            Some(filepath) => Miner::read_keypair_from_file(filepath.clone()),
            None => panic!("No keypair provided"),
        }
    }
    pub fn read_keypair_from_file(filepath: String) -> Keypair {
        use solana_sdk::signature::{Keypair, read_keypair_file};
        use std::fs::File;
        use std::io::Read;
        use std::path::Path;

        if !Path::new(&filepath).exists() {
            panic!("File not found at {}", filepath);
        }

        match read_keypair_file(&filepath) {
            Ok(keypair) => keypair,
            Err(_) => {
                // Try to read as base58 string
                let mut file = File::open(&filepath).expect("Failed to open file");
                let mut contents = String::new();
                file.read_to_string(&mut contents)
                    .expect("Failed to read file contents");
                let trimmed_contents = contents.trim();
                Keypair::from_base58_string(trimmed_contents)
            }
        }
    }
    pub fn fee_payer(&self) -> Keypair {
        match self.fee_payer_filepath.clone() {
            Some(filepath) => Miner::read_keypair_from_file(filepath.clone()),
            None => panic!("No fee payer keypair provided"),
        }
    }
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
    #[command(about = "Start collecting on your local machine")]
    Collect(CollectArgs),
    #[command(about = "Benchmark your machine's hashpower")]
    Benchmark(BenchmarkArgs),
}
