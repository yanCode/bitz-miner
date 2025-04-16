use clap::{Parser, Subcommand};

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

#[derive(Parser, Debug)]
pub struct CollectArgs {
    #[arg(
        long,
        short,
        value_name = "CORES_COUNT",
        help = "The number of CPU cores to allocate to collecting.",
        default_value_t = (num_cpus::get() - 1).max(1).to_string()
    )]
    pub cores: String,

    #[arg(
        long,
        short,
        value_name = "SECONDS",
        help = "The number seconds before the deadline to stop collecting and start submitting.",
        default_value = "5"
    )]
    pub buffer_time: u64,

    #[arg(
        long,
        short,
        value_name = "MIN_DIFFICULTY",
        help = "The minimum difficulty to collect at.",
        default_value = "20"
    )]
    pub min_difficulty: u32,

    #[arg(
        long,
        short,
        value_name = "DEVICE_ID",
        help = "An optional device id to use for pool collecting (max 5 devices per keypair)."
    )]
    pub device_id: Option<u64>,

    #[arg(
        long,
        short,
        value_name = "POOL_URL",
        help = "The optional pool url to join and forward solutions to."
    )]
    pub pool_url: Option<String>,

    #[arg(
        long,
        short,
        help = "Flag indicating whether or not to run in verbose mode.",
        default_value = "false"
    )]
    pub verbose: bool,
}

#[derive(Parser, Debug)]
pub struct BenchmarkArgs {
    #[arg(
        long,
        short,
        value_name = "THREAD_COUNT",
        help = "The number of cores to use during the benchmark",
        default_value = "1"
    )]
    pub cores: String,
}
