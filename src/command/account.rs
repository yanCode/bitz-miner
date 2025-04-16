use std::str::FromStr;

use crate::{AccountArgs, Miner, utils::TableData};
use solana_program::pubkey::Pubkey;
use solana_sdk::{native_token::lamports_to_sol, signer::Signer};
use spl_associated_token_account::get_associated_token_address;

impl Miner {
    pub async fn account(&self, args: AccountArgs) {
        if let Some(command) = args.command {
            match command {
                AccountCommand::Close(args) => self.close(args).await,
            }
        } else {
            self.get_account(args).await;
        }
    }

    async fn get_account(&self, args: AccountArgs) {
        // Parse account address
        let signer = self.signer();
        let address = if let Some(address) = &args.address {
            if let Ok(address) = Pubkey::from_str(&address) {
                address
            } else {
                println!("Invalid address: {:?}", address);
                return;
            }
        } else if args.proof.is_some() {
            return self.get_proof_account(args).await;
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

    async fn get_proof_account(&self, args: AccountArgs) {
        // Parse account address
        let proof_address = if let Some(address) = &args.proof {
            if let Ok(address) = Pubkey::from_str(&address) {
                address
            } else {
                println!("Invalid address: {:?}", address);
                return;
            }
        } else {
            return;
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
    }
}
