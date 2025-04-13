use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;
use solana_sdk::program_pack::Pack;
use spl_token::state::Mint;
pub enum ComputeBudget {
    #[allow(dead_code)]
    Dynamic,
    Fixed(u32),
}

pub async fn get_mint(client: &RpcClient, address: Pubkey) -> Result<Mint> {
    let mint_data = client.get_account_data(&address).await?;
    let mint = Mint::unpack(&mint_data)?;
    Ok(mint)
}
