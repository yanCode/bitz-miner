use std::str::FromStr;

use crate::{
    Miner,
    args::ClaimArgs,
    utils::{ComputeBudget, amount_f64_to_u64, ask_confirm, get_proof_with_authority},
};
use anyhow::{Result, bail};
use clap::Parser;
use colored::Colorize;
use eore_api::consts::MINT_ADDRESS;
use log::info;
use solana_sdk::signer::Signer;
use spl_associated_token_account::{
    get_associated_token_address, instruction::create_associated_token_account,
};
use spl_token::amount_to_ui_amount;
use steel::Pubkey;

impl Miner {
    pub async fn claim(&self, args: ClaimArgs) -> Result<()> {
        if args.pool_url.is_some() {
            bail!("Pool claiming not supported yet.");
        }
        self.claim_from_proof(args).await;
        Ok(())
    }

    pub async fn claim_from_proof(&self, args: ClaimArgs) {
        let signer = self.signer();
        let pubkey = signer.pubkey();
        let proof = get_proof_with_authority(&self.rpc_client, pubkey)
            .await
            .expect("Failed to fetch proof account");
        let mut ixs = vec![];
        let beneficiary = match args.to {
            None => self.initialize_ata(pubkey).await,
            Some(to) => {
                // Create beneficiary token account, if needed
                let wallet = Pubkey::from_str(&to).expect("Failed to parse wallet address");
                let benefiary_tokens = spl_associated_token_account::get_associated_token_address(
                    &wallet,
                    &MINT_ADDRESS,
                );
                if self
                    .rpc_client
                    .get_token_account(&benefiary_tokens)
                    .await
                    .is_err()
                {
                    ixs.push(
                        spl_associated_token_account::instruction::create_associated_token_account(
                            &pubkey,
                            &wallet,
                            &eore_api::consts::MINT_ADDRESS,
                            &spl_token::id(),
                        ),
                    );
                }
                benefiary_tokens
            }
        };

        // Parse amount to claim
        let amount = if let Some(amount) = args.amount {
            amount_f64_to_u64(amount)
        } else {
            proof.balance
        };

        // Confirm user wants to claim
        if !ask_confirm(
            format!(
                "\nYou are about to claim {}.\n\nAre you sure you want to continue? [Y/n]",
                format!(
                    "{} BITZ",
                    amount_to_ui_amount(amount, eore_api::consts::TOKEN_DECIMALS)
                )
                .bold(),
            )
            .as_str(),
        ) {
            return;
        }

        // Send and confirm
        ixs.push(eore_api::sdk::claim(pubkey, beneficiary, amount));
        self.send_and_confirm(&ixs, ComputeBudget::Fixed(32_000), false)
            .await
            .ok();
    }
    pub async fn initialize_ata(&self, wallet: Pubkey) -> Pubkey {
        // Initialize client.
        let signer = self.signer();
        let client = self.rpc_client.clone();

        // Build instructions.
        let token_account_pubkey = spl_associated_token_account::get_associated_token_address(
            &wallet,
            &eore_api::consts::MINT_ADDRESS,
        );

        // Check if ata already exists
        if let Ok(Some(_ata)) = client.get_token_account(&token_account_pubkey).await {
            return token_account_pubkey;
        }
        // Sign and send transaction.
        let ix = spl_associated_token_account::instruction::create_associated_token_account(
            &signer.pubkey(),
            &signer.pubkey(),
            &eore_api::consts::MINT_ADDRESS,
            &spl_token::id(),
        );
        self.send_and_confirm(&[ix], ComputeBudget::Fixed(400_000), false)
            .await
            .ok();

        // Return token account address
        token_account_pubkey
    }
}
