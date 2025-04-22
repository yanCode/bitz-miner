#![allow(dead_code)]
use std::{collections::HashMap, iter, str::FromStr};

use crate::Miner;
use anyhow::{Result, anyhow, bail};
use eore_api::consts::BUS_ADDRESSES;
use reqwest::Client;
use serde_json::{Value, json};
use solana_client::rpc_response::RpcPrioritizationFee;
use steel::Pubkey;
use url::Url;

enum FeeStrategy {
    Helius,
    Triton,
    LOCAL,
    Alchemy,
    Quiknode,
}

impl Miner {
    pub async fn get_priority_fee(&self) -> Result<u64> {
        let sender = self.rpc_client.url();
        let rpc_url = self.dynamic_fee_url.as_ref().unwrap_or(&sender);
        let host = Url::parse(&rpc_url)?
            .host_str()
            .ok_or_else(|| anyhow!("cannot parse host"))?
            .to_string();

        let strategy = match host {
            h if h.contains("helius-rpc.com") => FeeStrategy::Helius,
            h if h.contains("alchemy.com") => FeeStrategy::Alchemy,
            h if h.contains("quiknode.pro") => FeeStrategy::Quiknode,
            h if h.contains("rpcpool.com") => FeeStrategy::Triton,
            _ => FeeStrategy::LOCAL,
        };
        // Build fee estimate request
        let client = Client::new();
        let ore_addresses = iter::once(eore_api::ID.to_string())
            .chain(BUS_ADDRESSES.iter().map(|a| a.to_string()))
            .collect::<Vec<_>>();

        let body = match strategy {
            FeeStrategy::Helius => Some(json!({
                "jsonrpc": "2.0",
                "id": "priority-fee-estimate",
                "method": "getPriorityFeeEstimate",
                "params": [{
                    "accountKeys": ore_addresses,
                    "options": {
                        "recommended": true
                    }
                }]
            })),
            FeeStrategy::Alchemy => Some(json!({
                "jsonrpc": "2.0",
                "id": "priority-fee-estimate",
                "method": "getRecentPrioritizationFees",
                "params": [
                    ore_addresses
                ]
            })),
            FeeStrategy::Quiknode => Some(json!({
                "jsonrpc": "2.0",
                "id": "1",
                "method": "qn_estimatePriorityFees",
                "params": {
                    "account": "EorefDWqzJK31vLxaqkDGsx3CRKqPVpWfuJL7qBQMZYd",
                    "last_n_blocks": 100
                }
            })),
            FeeStrategy::Triton => Some(json!({
                "jsonrpc": "2.0",
                "id": "priority-fee-estimate",
                "method": "getRecentPrioritizationFees",
                "params": [
                    ore_addresses,
                    {
                        "percentile": 5000,
                    }
                ]
            })),
            FeeStrategy::LOCAL => None,
        };
        let response: Value = if let Some(body) = body {
            client
                .post(rpc_url)
                .json(&body)
                .send()
                .await?
                .json()
                .await?
        } else {
            Value::Null
        };
        let calculated_fee = self
            .calculate_priority_fee(strategy, &response)
            .await
            .map(|fee| {
                if let Some(max_fee) = self.priority_fee {
                    fee.min(max_fee)
                } else {
                    fee
                }
            });
        calculated_fee
    }

    pub async fn local_dynamic_fee(&self) -> Result<u64> {
        let client = self.rpc_client.clone();
        let pubkey = [
            "EorefDWqzJK31vLxaqkDGsx3CRKqPVpWfuJL7qBQMZYd",
            "5HngGmYzvSuh3XyU11brHDpMTHXQQRQQT4udGFtQSjgR",
            "2oLNTQKRb4a2117kFi6BYTUDu3RPrMVAHFhCfPKMosxX",
        ];
        let address_strings = pubkey;

        // Convert strings to Pubkey
        let addresses: Vec<Pubkey> = address_strings
            .into_iter()
            .map(|addr_str| Pubkey::from_str(addr_str).expect("Invalid address"))
            .collect();

        // Get recent prioritization fees
        let recent_prioritization_fees = client.get_recent_prioritization_fees(&addresses).await?;
        if recent_prioritization_fees.is_empty() {
            panic!("No recent prioritization fees");
        }
        let mut sorted_fees: Vec<_> = recent_prioritization_fees.into_iter().collect();
        sorted_fees.sort_by(|a, b| b.slot.cmp(&a.slot));
        let chunk_size = 150;
        let chunks: Vec<_> = sorted_fees.chunks(chunk_size).take(3).collect();
        let mut percentiles: HashMap<u8, u64> = HashMap::new();
        for (_, chunk) in chunks.iter().enumerate() {
            let fees: Vec<u64> = chunk.iter().map(|fee| fee.prioritization_fee).collect();
            percentiles = Self::calculate_percentiles(&fees);
            // Default to 75 percentile
        }

        // Default to 75 percentile
        let fee = *percentiles.get(&75).unwrap_or(&0);
        Ok(fee)
    }

    async fn calculate_priority_fee(&self, strategy: FeeStrategy, response: &Value) -> Result<u64> {
        let calculated_fee = match strategy {
        FeeStrategy::Helius => response["result"]["priorityFeeEstimate"]
            .as_f64()
            .map(|fee| fee as u64)
            .ok_or_else(|| anyhow!("Failed to parse priority fee response: {:?}", response)),
        FeeStrategy::Quiknode => response["result"]["per_compute_unit"]["medium"]
            .as_f64()
            .map(|fee| fee as u64)
            .ok_or_else(|| {
                anyhow!(
                    "Please enable the Solana Priority Fee API add-on in your QuickNode account."
                )
            }),
        FeeStrategy::Alchemy => response["result"]
            .as_array()
            .and_then(|arr| {
                Some(
                    arr.into_iter()
                        .map(|v| v["prioritizationFee"].as_u64().unwrap())
                        .collect::<Vec<u64>>(),
                )
            })
            .and_then(|fees| {
                Some(((fees.iter().sum::<u64>() as f32 / fees.len() as f32).ceil() * 1.2) as u64)
            })
            .ok_or_else(|| anyhow!("Failed to parse priority fee response: {:?}", response)),
        FeeStrategy::Triton => {
            serde_json::from_value::<Vec<RpcPrioritizationFee>>(response["result"].clone())
                .map(|prioritization_fees| {
                    estimate_prioritization_fee_microlamports(prioritization_fees)
                })
                .or_else(|error: serde_json::Error| {
                    bail!("Failed to parse priority fee response: {response:?}, error: {error}")
                })
        }
        FeeStrategy::LOCAL => self
            .local_dynamic_fee()
            .await
            .or_else(|err| bail!("Failed to parse priority fee response: {err}")),
    };
        calculated_fee
    }
    fn calculate_percentiles(fees: &[u64]) -> HashMap<u8, u64> {
        let mut sorted_fees = fees.to_vec();
        sorted_fees.sort_unstable();
        let len = sorted_fees.len();
        let percentiles = vec![10, 25, 50, 60, 70, 75, 80, 85, 90, 100];
        percentiles
            .into_iter()
            .map(|p| {
                let index = (p as f64 / 100.0 * len as f64).round() as usize;
                (p, sorted_fees[index.saturating_sub(1)])
            })
            .collect()
    }
}

fn estimate_prioritization_fee_microlamports(
    prioritization_fees: Vec<RpcPrioritizationFee>,
) -> u64 {
    let prioritization_fees = prioritization_fees
        .into_iter()
        .rev()
        .take(20)
        .map(
            |RpcPrioritizationFee {
                 prioritization_fee, ..
             }| prioritization_fee,
        )
        .collect::<Vec<_>>();
    if prioritization_fees.is_empty() {
        panic!("Response does not contain any prioritization fees");
    }

    let prioritization_fee =
        prioritization_fees.iter().sum::<u64>() / prioritization_fees.len() as u64;

    prioritization_fee
}
