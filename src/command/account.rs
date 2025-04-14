use std::str::FromStr;

use crate::{AccountArgs, Miner, utils::TableData};
use solana_program::pubkey::Pubkey;
use solana_sdk::{native_token::lamports_to_sol, signer::Signer};
use spl_associated_token_account::get_associated_token_address;

impl Miner {
    pub async fn account(&self, args: AccountArgs) {
        // if let Some(command) = args.command {
        //     match command {
        //         AccountCommand::Close(args) => {
        //             self.close(args).await;
        //         }
        //     }
        // } else {
        //     self.get_account(args).await;
        // }
    }
    // async fn get_account(&self, args: AccountArgs) {
    //     let signer = self.signer();
    //     let address = match (&args.address, args.proof.is_some()) {
    //         (Some(add_str), _) => {
    //             Pubkey::from_str(add_str).map_err(|e| {
    //                 println!("Invalid address: {}", e);
    //                 std::process::exit(2);
    //             });
    //         }
    //         (None, true) => self.get_proof_account(args).await,
    //         (None, false) => signer.pubkey(),
    //     };
    // }
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
}
