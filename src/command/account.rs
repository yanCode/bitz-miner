use anyhow::{Result, bail};
use std::str::FromStr;

use crate::{
    AccountArgs, Miner,
    args::{AccountCommand, ClaimArgs},
    utils::{
        ComputeBudget, TableData, TableSectionTitle, amount_u64_to_f64, ask_confirm,
        format_timestamp, get_proof, get_proof_with_authority,
    },
};
use colored::Colorize;
use eore_api::state::proof_pda;
use solana_program::pubkey::Pubkey;
use solana_sdk::{native_token::lamports_to_sol, signer::Signer};
use spl_associated_token_account::get_associated_token_address;
use spl_token::amount_to_ui_amount;
use tabled::{
    Table,
    settings::{
        Alignment, Remove, Style,
        object::{Columns, Rows},
    },
};

impl Miner {
    pub async fn account(&self, args: AccountArgs) -> Result<()> {
        if let Some(command) = args.command {
            match command {
                AccountCommand::Close => self.close().await?,
            }
        } else {
            self.get_account(args).await?;
        }
        Ok(())
    }

    async fn get_account(&self, args: AccountArgs) -> Result<()> {
        // Parse account address
        let signer = self.signer();
        if args.proof.is_some() {
            return self.get_proof_account(args).await;
        }
        let address = if let Some(address) = &args.address {
            Pubkey::from_str(&address)?
        } else {
            signer.pubkey()
        };
        // Aggregate data
        let mut data = vec![];
        self.get_account_data(address, &mut data).await;
        self.get_proof_data(address, &mut data).await;

        // Build table
        let mut table = Table::new(data);
        table.with(Remove::row(Rows::first()));
        table.modify(Columns::single(1), Alignment::right());
        table.with(Style::blank());
        table.section_title(0, "Account");
        table.section_title(3, "Proof");

        println!("{table}\n");
        Ok(())
    }
    async fn close(&self) -> Result<()> {
        let signer = self.signer();
        let proof = get_proof_with_authority(&self.rpc_client, signer.pubkey())
            .await
            .expect("Failed to fetch proof account");
        // Confirm the user wants to close.
        if !ask_confirm(
            format!("{} You have {} BITZ staked in this account.\nAre you sure you want to {}close this account? [Y/n]", 
                "WARNING:".bold().yellow(),
                amount_to_ui_amount(proof.balance, eore_api::consts::TOKEN_DECIMALS),
                if proof.balance.gt(&0) { "claim your stake and "} else { "" }
            ).as_str()
        ) {
            return Ok(());
        }
        // Claim stake
        if proof.balance.gt(&0) {
            self.claim_from_proof(ClaimArgs {
                amount: None,
                to: None,
                pool_url: None,
            })
            .await?;
        }
        let ix = eore_api::sdk::close(signer.pubkey());
        self.send_and_confirm(&[ix], ComputeBudget::Fixed(500_000), false)
            .await?;

        Ok(())
    }
    async fn get_account_data(&self, authority: Pubkey, data: &mut Vec<TableData>) {
        let token_account_address =
            get_associated_token_address(&authority, &eore_api::consts::MINT_ADDRESS);
        let token_balance = if let Ok(Some(token_account)) = self
            .rpc_client
            .get_token_account(&token_account_address)
            .await
        {
            token_account.token_amount.ui_amount_string
        } else {
            "0".to_string()
        };

        // Get ETH balance
        let sol_balance = self
            .rpc_client
            .get_balance(&authority)
            .await
            .expect("Failed to fetch ETH balance");
        // Aggregate data
        data.push(TableData {
            key: "Address".to_string(),
            value: authority.to_string(),
        });
        data.push(TableData {
            key: "Balance".to_string(),
            value: format!("{} BITZ", token_balance),
        });
        data.push(TableData {
            key: "ETH".to_string(),
            value: format!("{} ETH", lamports_to_sol(sol_balance)),
        });
    }

    async fn get_proof_account(&self, args: AccountArgs) -> Result<()> {
        // Parse account address
        let proof_address = if let Some(address) = &args.proof {
            if let Ok(address) = Pubkey::from_str(&address) {
                address
            } else {
                bail!(anyhow::anyhow!("Invalid address: {:?}", address));
            }
        } else {
            bail!(anyhow::anyhow!("Invalid address"));
        };

        // Aggregate data
        let proof = get_proof(&self.rpc_client, proof_address)
            .await
            .expect("Failed to fetch proof account");
        let mut data = vec![];
        self.get_account_data(proof.authority, &mut data).await;
        self.get_proof_data(proof.authority, &mut data).await;

        // Build table
        let mut table = Table::new(data);
        table.with(Remove::row(Rows::first()));
        table.modify(Columns::single(1), Alignment::right());
        table.with(Style::blank());
        table.section_title(0, "Account");
        table.section_title(3, "Proof");

        println!("{table}\n");
        Ok(())
    }
    async fn get_proof_data(&self, authority: Pubkey, data: &mut Vec<TableData>) {
        // Parse addresses
        let proof_address = proof_pda(authority).0;
        let proof = get_proof(&self.rpc_client, proof_address).await;

        // Aggregate data
        data.push(TableData {
            key: "Address".to_string(),
            value: proof_address.to_string(),
        });
        if let Ok(proof) = proof {
            data.push(TableData {
                key: "Authority".to_string(),
                value: authority.to_string(),
            });
            data.push(TableData {
                key: "Balance".to_string(),
                value: if proof.balance > 0 {
                    format!("{} BITZ", amount_u64_to_f64(proof.balance))
                        .bold()
                        .yellow()
                        .to_string()
                } else {
                    format!("{} BITZ", amount_u64_to_f64(proof.balance))
                },
            });
            data.push(TableData {
                key: "Last hash".to_string(),
                value: solana_sdk::hash::Hash::new_from_array(proof.last_hash).to_string(),
            });
            data.push(TableData {
                key: "Last hash at".to_string(),
                value: format_timestamp(proof.last_hash_at),
            });
            data.push(TableData {
                key: "Lifetime hashes".to_string(),
                value: proof.total_hashes.to_string(),
            });
            data.push(TableData {
                key: "Lifetime rewards".to_string(),
                value: format!(
                    "{} BITZ",
                    amount_to_ui_amount(proof.total_rewards, eore_api::consts::TOKEN_DECIMALS)
                ),
            });
            data.push(TableData {
                key: "Miner".to_string(),
                value: proof.miner.to_string(),
            });
        } else {
            data.push(TableData {
                key: "Status".to_string(),
                value: "Not found".red().bold().to_string(),
            });
        }
    }
}
