use colored::Colorize;
use indicatif::ProgressBar;
use log::{debug, error, info};
use solana_client::client_error::Result as ClientResult;
use solana_rpc_client::spinner;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::instruction::Instruction;
use solana_sdk::{
    native_token::{lamports_to_sol, sol_to_lamports},
    signature::Signature,
    signer::Signer,
};

use crate::{Miner, utils::ComputeBudget};
const MIN_ETH_BALANCE: f64 = 0.0005;
impl Miner {
    pub async fn send_and_confirm(
        &self,
        ixs: &[Instruction],
        compute_budget: ComputeBudget,
        skip_confirm: bool,
    ) -> ClientResult<Signature> {
        debug!("Starting send_and_confirm with {} instructions", ixs.len());
        let progress_bar = spinner::new_progress_bar();
        let signer = self.signer();
        let client = self.rpc_client.clone();
        let fee_payer = self.fee_payer();
        debug!("Using signer: {}", signer.pubkey());
        debug!("Using fee payer: {}", fee_payer.pubkey());
        debug!("RPC client URL: {}", client.url());
        // Return error, if balance is zero
        self.check_balance().await;
        // Set compute budget
        let mut final_ixs = vec![];
        match compute_budget {
            ComputeBudget::Dynamic => {
                debug!("Using dynamic compute budget");
                todo!("simulate tx")
            }
            ComputeBudget::Fixed(cus) => {
                debug!("Using fixed compute budget: {} CUs", cus);
                final_ixs.push(ComputeBudgetInstruction::set_compute_unit_limit(cus))
            }
        }
        // Set compute unit price
        let priority_fee = self.priority_fee.unwrap_or(0);
        debug!("Setting compute unit price: {} microlamports", priority_fee);
        final_ixs.push(ComputeBudgetInstruction::set_compute_unit_price(
            priority_fee,
        ));

        // Add in user instructions
        debug!("Adding {} user instructions", ixs.len());
        // Log program addresses for original instructions
        for (i, ix) in ixs.iter().enumerate() {
            info!(
                "Original Instruction #{}: Program ID = {}",
                i, ix.program_id
            );
            debug!(
                "  - Accounts: {:?}",
                ix.accounts
                    .iter()
                    .map(|a| a.pubkey.to_string())
                    .collect::<Vec<_>>()
            );
        }

        final_ixs.extend_from_slice(ixs);

        unimplemented!()
    }
    #[allow(dead_code)]
    pub async fn check_balance(&self) {
        debug!("Checking balance for signer: {}", self.signer().pubkey());
        let balance = self
            .rpc_client
            .get_balance(&self.signer().pubkey())
            .await
            .unwrap_or(0);

        if balance < sol_to_lamports(MIN_ETH_BALANCE) {
            let error_msg = format!(
                "Insufficient balance: {} ETH < {} ETH",
                lamports_to_sol(balance),
                MIN_ETH_BALANCE
            );
            error!("{error_msg}");
            panic!("{error_msg}");
        }
    }
}

fn log_error(progress_bar: &ProgressBar, err: &str, finish: bool) {
    if finish {
        progress_bar.finish_with_message(format!("{} {}", "ERROR".bold().red(), err));
    } else {
        progress_bar.println(format!("  {} {}", "ERROR".bold().red(), err));
    }
}

fn log_warning(progress_bar: &ProgressBar, msg: &str) {
    progress_bar.println(format!("  {} {}", "WARNING".bold().yellow(), msg));
}
