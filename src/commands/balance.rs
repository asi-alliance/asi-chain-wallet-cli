use crate::config::Config;
use anyhow::{Result, ensure};
use node_cli::{Balance, Endpoint, NodeHttp, f1r3fly_api::NodeStatus};
use serde_json::Value;

pub async fn metadata(config: &Config) -> Result<NodeStatus> {
    validate_metadata(config, fetch_status(&config.observer_http).await?)
}

pub async fn validator_metadata(config: &Config) -> Result<NodeStatus> {
    let status = fetch_status(&config.validator_http).await?;
    ensure!(
        !status.get("maxPhloLimit").is_some_and(Value::is_null),
        "Validator has not reported its maximum Phlo limit yet"
    );
    validate_metadata(config, status)
}

async fn fetch_status(endpoint: &Endpoint) -> Result<Value> {
    let node = NodeHttp::new(endpoint.clone())?;
    Ok(node.get_json("/api/status").await?)
}

fn validate_metadata(config: &Config, status: Value) -> Result<NodeStatus> {
    let status: NodeStatus = serde_json::from_value(status)?;
    ensure!(
        status.native_token_symbol == config.expected_token_symbol,
        "Wrong native token: expected {}, node reports {}",
        config.expected_token_symbol,
        status.native_token_symbol
    );
    ensure!(
        status.shard_id == config.shard_id,
        "Node shard does not match SHARD_ID"
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
    let wallet = config.wallet_identity()?;
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
