use crate::{
    Miner,
    args::CollectArgs,
    constants::MAX_TRANSACTION_POLL_ATTEMPTS,
    utils::{
        ComputeBudget, SoloCollectingData, amount_u64_to_f64, find_hash_parallel, format_timestamp,
        get_clock, get_config, get_updated_proof_with_authority,
    },
};
use anyhow::{Result, bail};
use std::{io::stdout, time::Duration};

use b64::FromBase64;
use colored::Colorize;
use crossterm::{
    cursor::MoveTo,
    execute,
    terminal::{Clear, ClearType},
};
use eore_api::{
    consts::{BUS_ADDRESSES, EPOCH_DURATION},
    event::MineEvent,
    state::{Bus, Config, proof_pda},
};
use log::error;
use solana_program::pubkey::Pubkey;
use solana_sdk::{signature::Signature, signer::Signer};
use solana_transaction_status::{
    EncodedConfirmedTransactionWithStatusMeta, UiTransactionEncoding,
    option_serializer::OptionSerializer,
};
use steel::AccountDeserialize;
use tabled::{
    Table,
    settings::{
        Alignment, Border, Color, Highlight, Remove, Style,
        object::{Columns, Rows},
        style::BorderColor,
    },
};
use tokio::time::sleep;

impl Miner {
    pub fn parse_cores(&self, cores: String) -> u64 {
        if cores == "ALL" {
            num_cpus::get() as u64
        } else {
            cores.parse::<u64>().unwrap()
        }
    }
    pub async fn collect(&self, args: CollectArgs) -> Result<()> {
        match args.pool_url {
            Some(pool_url) => {
                error!("Collecting : {}", pool_url);
                todo!()
            }
            None => self.collect_solo(args).await,
        }
    }
    async fn collect_solo(&self, args: CollectArgs) -> Result<()> {
        self.open().await?;
        let core_num_str = args.cores;
        let cores = self.parse_cores(core_num_str);
        self.check_num_cores(cores)?;
        let verbose = args.verbose;
        let signer = self.signer();
        let boost_config_address = eore_boost_api::state::config_pda().0;
        // Start collecting loop
        let mut last_hash_at = 0;

        loop {
            let config = get_config(&self.rpc_client).await?;
            let min_difficulty = args.min_difficulty.max(config.min_difficulty as u32);
            let proof =
                get_updated_proof_with_authority(&self.rpc_client, signer.pubkey(), last_hash_at)
                    .await?;
            // Log collecting table
            self.update_solo_collecting_table(verbose)?;
            last_hash_at = proof.last_hash_at;
            // Calculate cutoff time
            let cutoff_time = self.get_cutoff(proof.last_hash_at, args.buffer_time).await;

            // Build nonce indices
            let mut nonce_indices = Vec::with_capacity(cores as usize);
            for n in 0..(cores) {
                let nonce = u64::MAX.saturating_div(cores).saturating_mul(n);
                nonce_indices.push(nonce);
            }
            let solution = find_hash_parallel(
                proof.challenge,
                cutoff_time,
                cores,
                min_difficulty as u32,
                nonce_indices.as_slice(),
                None,
            )
            .await?;

            // Build instruction set
            let mut ixs = vec![eore_api::sdk::auth(proof_pda(signer.pubkey()).0)];
            let mut compute_budget = 750_000;
            // Check for reset
            if self.should_reset(config).await
            // && rand::thread_rng().gen_range(0..100).eq(&0)
            {
                compute_budget += 100_000;
                ixs.push(eore_api::sdk::reset(signer.pubkey()));
            }
            // Build collect ix
            let collect_ix = eore_api::sdk::mine(
                signer.pubkey(),
                signer.pubkey(),
                self.find_bus().await,
                solution,
                boost_config_address,
            );
            ixs.push(collect_ix);
            match self
                .send_and_confirm(&ixs, ComputeBudget::Fixed(compute_budget), false)
                .await
            {
                Ok(sig) => self.fetch_solo_collect_event(sig, verbose).await?,
                Err(err) => {
                    let collecting_data = SoloCollectingData::failed();
                    let mut data = self.solo_collecting_data.write().map_err(|e| {
                        anyhow::anyhow!(
                            "failed to write to solo_collecting_data: lock poisoned: {}",
                            e
                        )
                    })?;
                    if !data.is_empty() {
                        data.remove(0);
                    }
                    data.insert(0, collecting_data);
                    drop(data);

                    // Log collecting table
                    self.update_solo_collecting_table(verbose)?;
                    println!("{}: {}", "ERROR".bold().red(), err);

                    bail!(err);
                }
            }
        }
    }
    async fn open(&self) -> Result<()> {
        let signer = self.signer();
        let fee_payer = self.fee_payer();
        let proof_address = proof_pda(signer.pubkey()).0;
        if let Err(_) = self.rpc_client.get_account(&proof_address).await {
            let mut ixs = Vec::new();
            let ix = eore_api::sdk::open(signer.pubkey(), signer.pubkey(), fee_payer.pubkey());
            ixs.push(ix);
            self.send_and_confirm(&ixs, ComputeBudget::Fixed(400_000), false)
                .await?;
        }

        Ok(())
    }

    async fn get_cutoff(&self, last_hash_at: i64, buffer_time: u64) -> u64 {
        let clock = get_clock(&self.rpc_client)
            .await
            .expect("Failed to fetch clock account");
        last_hash_at
            .saturating_add(60)
            .saturating_sub(buffer_time as i64)
            .saturating_sub(clock.unix_timestamp)
            .max(0) as u64
    }

    async fn should_reset(&self, config: Config) -> bool {
        let clock = get_clock(&self.rpc_client)
            .await
            .expect("Failed to fetch clock account");
        config
            .last_reset_at
            .saturating_add(EPOCH_DURATION)
            .saturating_sub(5) // Buffer
            .le(&clock.unix_timestamp)
    }

    pub fn check_num_cores(&self, core: u64) -> Result<()> {
        let actual_cores = num_cpus::get() as u64;
        if core > actual_cores {
            bail!(
                "Requested cores {} is greater than actual cores {}",
                core,
                actual_cores
            );
        }
        Ok(())
    }

    async fn find_bus(&self) -> Pubkey {
        self.rpc_client
            .get_multiple_accounts(&BUS_ADDRESSES)
            .await
            .map(|accounts| {
                accounts
                    .iter()
                    .enumerate()
                    .fold(
                        (BUS_ADDRESSES[0], 0u64),
                        |(max_bus, max_rewards), (idx, account)| {
                            if let Some(account) = account {
                                if let Ok(bus) = Bus::try_from_bytes(&account.data) {
                                    if bus.rewards > max_rewards {
                                        return (BUS_ADDRESSES[idx], bus.rewards);
                                    }
                                }
                            }
                            (max_bus, max_rewards)
                        },
                    )
                    .0
            })
            .unwrap_or(BUS_ADDRESSES[0])
    }

    fn update_solo_collecting_table(&self, verbose: bool) -> Result<()> {
        execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0))?;
        let mut rows: Vec<SoloCollectingData> = vec![];
        let data = self.solo_collecting_data.read().map_err(|e| {
            anyhow::anyhow!("failed to read solo_collecting_data: lock poisoned: {}", e)
        })?;
        rows.extend(data.iter().cloned());
        let mut table = Table::new(&rows);
        table.with(Style::blank());
        table.modify(Columns::new(1..), Alignment::right());
        table.modify(Rows::first(), Color::BOLD);
        table.with(
            Highlight::new(Rows::single(1)).color(BorderColor::default().top(Color::FG_WHITE)),
        );
        table.with(Highlight::new(Rows::single(1)).border(Border::new().top('━')));
        if !verbose {
            table.with(Remove::column(Columns::new(1..3)));
        }
        println!("\n{}\n", table);
        Ok(())
    }

    async fn fetch_solo_collect_event(&self, sig: Signature, verbose: bool) -> Result<()> {
        let collecting_data = SoloCollectingData::fetching(sig);
        let mut data = self.solo_collecting_data.write().map_err(|e| {
            anyhow::anyhow!(
                "failed to write to solo_collecting_data: lock poisoned: {}",
                e
            )
        })?;
        data.insert(0, collecting_data);
        if !data.is_empty() {
            data.remove(0);
        }
        drop(data);
        self.update_solo_collecting_table(verbose)?;
        let tx = self.poll_transaction(sig).await;
        if let Ok(tx) = tx {
            let return_data = self.parse_transaction_meta(&tx).await;
            if let Some(return_data) = return_data {
                let mut data = self.solo_collecting_data.write().map_err(|e| {
                    anyhow::anyhow!(
                        "failed to write to solo_collecting_data: lock poisoned: {}",
                        e
                    )
                })?;
                let event = MineEvent::from_bytes(&return_data);
                let collecting_data = SoloCollectingData {
                    signature: format_signature(&sig, verbose),
                    block: tx.slot.to_string(),
                    timestamp: format_timestamp(tx.block_time.unwrap_or_default()),
                    difficulty: event.difficulty.to_string(),
                    base_reward: format_reward(event.net_base_reward),
                    boost_reward: format_reward(event.net_miner_boost_reward),
                    total_reward: format_reward(event.net_reward),
                    timing: format!("{}s", event.timing),
                    status: "Confirmed".bold().green().to_string(),
                };

                data.insert(0, collecting_data);
            }
        }
        Ok(())
    }
    async fn poll_transaction(
        &self,
        sig: Signature,
    ) -> Result<EncodedConfirmedTransactionWithStatusMeta> {
        for _ in 0..MAX_TRANSACTION_POLL_ATTEMPTS {
            match self
                .rpc_client
                .get_transaction(&sig, UiTransactionEncoding::Json)
                .await
            {
                Ok(tx) => return Ok(tx),
                Err(_) => {
                    sleep(Duration::from_secs(1)).await;
                }
            }
        }
        bail!("Failed to fetch transaction after 30 attempts")
    }
    async fn parse_transaction_meta(
        &self,
        tx: &EncodedConfirmedTransactionWithStatusMeta,
    ) -> Option<Vec<u8>> {
        let meta = tx.transaction.meta.as_ref()?;
        if let OptionSerializer::Some(ref log_messages) = meta.log_messages {
            let return_log = log_messages
                .iter()
                .find(|log| log.starts_with("Program return: "))?;
            let return_data =
                return_log.strip_prefix(&format!("Program return: {} ", eore_api::ID))?;

            return_data.from_base64().ok()
        } else {
            None
        }
    }
}

fn format_signature(sig: &Signature, verbose: bool) -> String {
    if verbose {
        sig.to_string()
    } else {
        format!("{}...", &sig.to_string()[..8])
    }
}
fn format_reward(reward: u64) -> String {
    if reward > 0 {
        format!("{:#.11}", amount_u64_to_f64(reward))
    } else {
        "0".to_string()
    }
}
