use std::str::FromStr;

use crate::{
    Miner,
    args::{StakeArgs, StakeCommand, StakeDepositArgs},
    utils::{ComputeBudget, get_boost},
};
use anyhow::Result;
use eore_api::consts::MINT_ADDRESS;
use eore_boost_api::state::{boost_pda, stake_pda};
use log::{debug, error, info};
use solana_sdk::{program_pack::Pack, signer::Signer};
use spl_token::state::Mint;
use steel::Pubkey;

impl Miner {
    pub async fn stake(&self, args: StakeArgs) -> Result<()> {
        if let Some(subcommand) = args.command {
            match subcommand {
                StakeCommand::Claim(_args) => todo!(),
                StakeCommand::Deposit(_args) => todo!(),
                StakeCommand::Withdraw(_args) => todo!(),
                StakeCommand::Accounts(_args) => todo!(),
            }
        } else {
            match args.mint {
                Some(_mint) => todo!(), //get authority
                None => todo!(),        //list
            }
        }
    }

    async fn stake_deposit(&self, args: StakeDepositArgs, stake_args: StakeArgs) -> Result<()> {
        // Parse mint address
        let mint_address = match stake_args.mint {
            Some(mint_str) => {
                info!("Using provided mint address: {}", mint_str);
                Pubkey::from_str(&mint_str).expect("Failed to parse mint address")
            }
            None => {
                info!(
                    "No mint provided, using default MINT_ADDRESS {}",
                    MINT_ADDRESS
                );
                MINT_ADDRESS
            }
        };
        // Get signer
        let signer = self.signer();
        // Get sender token account
        let sender = match &args.token_account {
            Some(address) => {
                debug!("Using provided token account: {}", address);
                Pubkey::from_str(&address).expect("Failed to parse token account address")
            }
            None => {
                let ata = spl_associated_token_account::get_associated_token_address(
                    &signer.pubkey(),
                    &mint_address,
                );
                debug!("Using derived ATA address: {}", ata);
                ata
            }
        };
        let mint_data = match self.rpc_client.get_account_data(&mint_address).await {
            Ok(data) => data,
            Err(err) => {
                error!("ERROR: Failed to fetch mint account data: {}", err);
                panic!("Failed to fetch mint account");
            }
        };
        let mint = match Mint::unpack(&mint_data) {
            Ok(mint) => {
                println!(
                    "Successfully unpacked mint data. Decimals: {}",
                    mint.decimals
                );
                mint
            }
            Err(err) => {
                error!("ERROR: Failed to unpack mint data: {}", err);
                panic!("Failed to parse mint account");
            }
        };
        let token_account = match self.rpc_client.get_token_account(&sender).await {
            Ok(Some(account)) => {
                debug!(
                    "Found token account with balance: {}",
                    account.token_amount.amount
                );
                account
            }
            Ok(None) => {
                error!("ERROR: Token account not found");
                panic!("Token account not found");
            }
            Err(err) => {
                error!("ERROR: Failed to fetch token account: {}", err);
                panic!("Failed to fetch token account");
            }
        };
        let amount: u64 = if let Some(amount) = args.amount {
            let calculated = (amount * 10f64.powf(mint.decimals as f64)) as u64;
            debug!("Using provided amount: {} (raw: {})", amount, calculated);
            calculated
        } else {
            let balance = u64::from_str(token_account.token_amount.amount.as_str())
                .expect("Failed to parse token balance");
            debug!("Using full balance amount: {}", balance);
            balance
        };
        // Get addresses
        let boost_address = boost_pda(mint_address).0;
        debug!("Derived boost PDA: {}", boost_address);

        let stake_address = stake_pda(signer.pubkey(), boost_address).0;
        debug!("Derived stake PDA: {}", stake_address);

        let boost = match get_boost(&self.rpc_client, boost_address).await {
            Ok(boost) => {
                info!("Found boost account with weight: {}", boost.weight);
                boost
            }
            Err(err) => {
                error!("ERROR: No boost account found for mint {}", mint_address);
                error!("Error details: {}", err);
                panic!("Boost account not found");
            }
        };
        if self
            .rpc_client
            .get_account_data(&stake_address)
            .await
            .is_err()
        {
            info!("Stake account not found, initializing...");
            let ix = eore_boost_api::sdk::open(signer.pubkey(), signer.pubkey(), mint_address);
            match self
                .send_and_confirm(&[ix], ComputeBudget::Fixed(50_000), false)
                .await
            {
                Ok(_) => println!("Successfully initialized stake account"),
                Err(err) => {
                    error!("ERROR: Failed to initialize stake account: {}", err);
                    panic!("Failed to initialize stake account");
                }
            }
        } else {
            debug!("Stake account already exists");
        }
        println!("Sending deposit transaction...");
        let ix = eore_boost_api::sdk::deposit(signer.pubkey(), mint_address, amount);
        match self
            .send_and_confirm(&[ix], ComputeBudget::Fixed(50_000), false)
            .await
        {
            Ok(_) => {
                info!("Successfully deposited {} tokens", amount);
                Ok(())
            }
            Err(err) => {
                error!("ERROR: Failed to deposit tokens: {}", err);
                panic!("Failed to deposit tokens");
            }
        }
    }
}
