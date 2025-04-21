use std::str::FromStr;

use crate::{
    Miner,
    args::ClaimArgs,
    utils::{ComputeBudget, amount_f64_to_u64, ask_confirm, get_proof_with_authority},
};
use anyhow::{Result, bail};
use colored::Colorize;
use eore_api::consts::MINT_ADDRESS;
use solana_sdk::signer::Signer;
use spl_token::amount_to_ui_amount;
use steel::Pubkey;

impl Miner {
    pub async fn claim(&self, args: ClaimArgs) -> Result<()> {
        if args.pool_url.is_some() {
            bail!("Pool claiming not supported yet.");
        }
        self.claim_from_proof(args).await?;
        Ok(())
    }

    pub async fn claim_from_proof(&self, args: ClaimArgs) -> Result<()> {
        let signer = self.signer();
        let pubkey = signer.pubkey();
        let proof = get_proof_with_authority(&self.rpc_client, pubkey)
            .await
            .expect("Failed to fetch proof account");
        let to_wallet = args.to.map_or(pubkey, |ref to| {
            Pubkey::from_str(to).expect("Failed to parse wallet address")
        });
        let beneficiary = self.get_or_initialize_ata(to_wallet).await;

        // Parse amount to claim

        let amount = args.amount.map_or(proof.balance, amount_f64_to_u64);

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
            return Ok(());
        }

        // Send and confirm
        let ixs = vec![eore_api::sdk::claim(pubkey, beneficiary, amount)];
        self.send_and_confirm(&ixs, ComputeBudget::Fixed(32_000), false)
            .await?;
        Ok(())
    }

    pub async fn get_or_initialize_ata(&self, wallet: Pubkey) -> Pubkey {
        // Initialize client.
        let signer = self.signer();
        let client = self.rpc_client.clone();

        // Build instructions.
        let token_account_pubkey =
            spl_associated_token_account::get_associated_token_address(&wallet, &MINT_ADDRESS);

        // Check if ata already exists
        if client
            .get_token_account(&token_account_pubkey)
            .await
            .ok()
            .flatten()
            .is_some()
        {
            return token_account_pubkey;
        }
        // Sign and send transaction.
        let ix = spl_associated_token_account::instruction::create_associated_token_account(
            &signer.pubkey(),
            &wallet,
            &MINT_ADDRESS,
            &spl_token::ID,
        );
        self.send_and_confirm(&[ix], ComputeBudget::Fixed(400_000), false)
            .await
            .ok();

        // Return token account address
        token_account_pubkey
    }
}
