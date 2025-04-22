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
    Close,
}

#[derive(Parser, Debug, Clone)]
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

#[derive(Parser, Debug)]
pub struct ClaimArgs {
    #[arg(
        value_name = "AMOUNT",
        help = "The amount of rewards to claim. Defaults to max."
    )]
    pub amount: Option<f64>,

    #[arg(
        long,
        value_name = "WALLET_ADDRESS",
        help = "Wallet address to receive claimed tokens."
    )]
    pub to: Option<String>,

    #[arg(
        long,
        short,
        value_name = "POOL_URL",
        help = "The optional pool url to claim rewards from."
    )]
    pub pool_url: Option<String>,
}

#[derive(Clone, Parser, Debug)]
pub struct StakeArgs {
    #[command(subcommand)]
    pub command: Option<StakeCommand>,

    #[arg(
        value_name = "MINT_ADDRESS",
        help = "The mint to stake with. Defaults to BITZ mint when not provided."
    )]
    pub mint: Option<String>,

    #[arg(
        long,
        short,
        value_name = "ACCOUNT_ADDRESS",
        help = "List the stake accounts of another authority."
    )]
    pub authority: Option<String>,
}

#[derive(Subcommand, Clone, Debug)]
pub enum StakeCommand {
    #[command(about = "Claim rewards from a stake account.")]
    Claim(StakeClaimArgs),

    #[command(about = "Deposit tokens into a stake account.")]
    Deposit(StakeDepositArgs),

    #[command(about = "Withdraw tokens from a stake account.")]
    Withdraw(StakeWithdrawArgs),

    #[command(about = "Get the list of stake accounts in a boost.")]
    Accounts(StakeAccountsArgs),
}

#[derive(Parser, Clone, Debug)]
pub struct StakeClaimArgs {
    #[arg(
        value_name = "AMOUNT",
        help = "The amount of rewards to claim. Defaults to max."
    )]
    pub amount: Option<f64>,

    #[arg(
        long,
        value_name = "WALLET_ADDRESS",
        help = "Wallet address to receive claimed tokens."
    )]
    pub to: Option<String>,
}

#[derive(Parser, Clone, Debug)]
pub struct StakeDepositArgs {
    #[arg(
        value_name = "AMOUNT",
        help = "The amount of stake to deposit. Defaults to max."
    )]
    pub amount: Option<f64>,

    #[arg(
        long,
        value_name = "TOKEN_ACCOUNT_ADDRESS",
        help = "Token account to deposit from. Defaults to the associated token account."
    )]
    pub token_account: Option<String>,
}

#[derive(Parser, Clone, Debug)]
pub struct StakeWithdrawArgs {
    #[arg(
        value_name = "AMOUNT",
        help = "The amount of stake to withdraw. Defaults to max."
    )]
    pub amount: Option<f64>,

    #[arg(
        long,
        value_name = "TOKEN_ACCOUNT_ADDRESS",
        help = "Token account to withdraw to. Defaults to the associated token account."
    )]
    pub token_account: Option<String>,
}

#[derive(Parser, Clone, Debug)]
pub struct StakeAccountsArgs {}
