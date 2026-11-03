use crate::config::Config;
use anyhow::{Result, ensure};
use node_cli::{Balance, NodeHttp, f1r3fly_api::NodeStatus};

pub async fn metadata(config: &Config) -> Result<NodeStatus> {
    let node = NodeHttp::new(config.observer_http.clone())?;
    let status: NodeStatus = serde_json::from_value(node.get_json("/api/status").await?)?;
    ensure!(
        status.native_token_symbol == config.expected_token_symbol,
        "Wrong native token: expected {}, node reports {}",
        config.expected_token_symbol,
        status.native_token_symbol
    );
    ensure!(
        status.shard_id == config.shard_id,
        "Observer shard does not match SHARD_ID"
    );
    Ok(status)
}

pub async fn get_balance(config: &Config, address: &str) -> Result<Balance> {
    Ok(NodeHttp::new(config.observer_http.clone())?
        .balance(address, None)
        .await?)
}

pub async fn handle_get_balance() -> Result<()> {
    let config = Config::from_env()?;
    let wallet = config.get_wallet()?;
    let token = metadata(&config).await?;
    let balance = get_balance(&config, &wallet.address).await?;
    println!("Wallet balance: {} dust", balance.dust);
    if let Some(decimals) = token.native_token_decimals {
        println!(
            "Native balance: {} {}",
            node_cli::vault::dust_to_tokens(&balance.dust, decimals)?,
            token.native_token_symbol
        );
    }
    Ok(())
}
