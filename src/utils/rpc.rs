use std::time::Duration;

use anyhow::Result;
use eore_api::{
    consts::CONFIG_ADDRESS,
    state::{Config, Proof, proof_pda},
};
use eore_boost_api::state::Boost;
use serde::Deserialize;
use solana_client::{
    client_error::{ClientError, ClientErrorKind},
    nonblocking::rpc_client::RpcClient,
};
use solana_program::{pubkey::Pubkey, sysvar};
use solana_sdk::{clock::Clock, hash::Hash, program_pack::Pack};
use spl_token::state::Mint;
use steel::AccountDeserialize;

pub const BLOCKHASH_QUERY_RETRIES: usize = 5;
pub const BLOCKHASH_QUERY_DELAY: u64 = 500;

pub enum ComputeBudget {
    #[allow(dead_code)]
    Dynamic,
    Fixed(u32),
}

pub async fn get_config(client: &RpcClient) -> Result<Config> {
    let data = client.get_account_data(&CONFIG_ADDRESS).await?;
    let config = Config::try_from_bytes(&data)?;
    Ok(*config)
}

pub async fn get_mint(client: &RpcClient, address: Pubkey) -> Result<Mint> {
    let mint_data = client.get_account_data(&address).await?;
    let mint = Mint::unpack(&mint_data)?;
    Ok(mint)
}

pub async fn get_proof(client: &RpcClient, address: Pubkey) -> Result<Proof> {
    let data = client.get_account_data(&address).await?;
    let proof = Proof::try_from_bytes(&data)?;
    Ok(*proof)
}

pub async fn get_proof_with_authority(client: &RpcClient, authority: Pubkey) -> Result<Proof> {
    let address = proof_pda(authority).0;
    let data = client.get_account_data(&address).await?;
    let proof = Proof::try_from_bytes(&data)?;
    Ok(*proof)
}
//todo: reactor
pub async fn get_updated_proof_with_authority(
    client: &RpcClient,
    authority: Pubkey,
    lash_hash_at: i64,
) -> Result<Proof, anyhow::Error> {
    loop {
        if let Ok(proof) = get_proof_with_authority(client, authority).await {
            if proof.last_hash_at.gt(&lash_hash_at) {
                return Ok(proof);
            }
        }
        tokio::time::sleep(Duration::from_millis(1_000)).await;
    }
}

pub async fn get_clock(client: &RpcClient) -> Result<Clock, anyhow::Error> {
    retry(|| async {
        let data = client.get_account_data(&sysvar::clock::ID).await?;
        Ok(bincode::deserialize::<Clock>(&data)?)
    })
    .await
}

pub async fn get_boost_config(client: &RpcClient) -> eore_boost_api::state::Config {
    let data = client
        .get_account_data(&eore_boost_api::state::config_pda().0)
        .await
        .expect("Failed to get config account");
    *eore_boost_api::state::Config::try_from_bytes(&data).expect("Failed to parse config account")
}

pub async fn get_boost(client: &RpcClient, address: Pubkey) -> Result<Boost, anyhow::Error> {
    let data = client.get_account_data(&address).await?;
    Ok(*Boost::try_from_bytes(&data).expect("Failed to parse boost account"))
}

pub async fn get_latest_blockhash_with_retries(
    client: &RpcClient,
) -> Result<(Hash, u64), ClientError> {
    let mut attempts = 0;

    loop {
        if let Ok((hash, slot)) = client
            .get_latest_blockhash_with_commitment(client.commitment())
            .await
        {
            return Ok((hash, slot));
        }

        // Retry
        tokio::time::sleep(Duration::from_millis(BLOCKHASH_QUERY_DELAY)).await;
        attempts += 1;
        if attempts >= BLOCKHASH_QUERY_RETRIES {
            return Err(ClientError {
                request: None,
                kind: ClientErrorKind::Custom(
                    "Max retries reached for latest blockhash query".into(),
                ),
            });
        }
    }
}

pub async fn retry<F, Fut, T>(f: F) -> Result<T, anyhow::Error>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, anyhow::Error>>,
{
    const MAX_RETRIES: u32 = 8;
    const INITIAL_BACKOFF: Duration = Duration::from_millis(200);
    const TIMEOUT: Duration = Duration::from_secs(8);
    let mut backoff = INITIAL_BACKOFF;
    for attempt in 0..MAX_RETRIES {
        match tokio::time::timeout(TIMEOUT, f()).await {
            Ok(Ok(result)) => return Ok(result),
            Ok(Err(_)) if attempt < MAX_RETRIES - 1 => {
                tokio::time::sleep(backoff).await;
                backoff *= 2; // Exponential backoff
            }
            Ok(Err(e)) => return Err(e),
            Err(_) if attempt < MAX_RETRIES - 1 => {
                tokio::time::sleep(backoff).await;
                backoff *= 2; // Exponential backoff
            }
            Err(_) => return Err(anyhow::anyhow!("Retry failed")),
        }
    }

    Err(anyhow::anyhow!("Retry failed"))
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct Tip {
    pub time: String,
    pub _landed_tips_25th_percentile: f64,
    pub landed_tips_50th_percentile: f64,
    pub landed_tips_75th_percentile: f64,
    pub landed_tips_95th_percentile: f64,
    pub landed_tips_99th_percentile: f64,
    pub ema_landed_tips_50th_percentile: f64,
}
