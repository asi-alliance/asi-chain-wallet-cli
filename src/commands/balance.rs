use crate::config::Config;
use anyhow::{Result, ensure};
use node_cli::{Balance, Endpoint, NodeHttp, f1r3fly_api::NodeStatus};
use serde_json::Value;

pub async fn metadata(config: &Config) -> Result<NodeStatus> {
    validate_metadata(config, fetch_status(&config.observer_http).await?)
}

pub async fn validator_metadata(config: &Config) -> Result<NodeStatus> {
    let status = fetch_status(&config.validator_http).await?;
    let unreported_cap = status.get("maxPhloLimit").is_some_and(Value::is_null);
    let validated = validate_metadata(config, status)?;
    ensure!(
        !unreported_cap,
        "Validator has not reported its maximum Phlo limit yet"
    );
    Ok(validated)
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, http::StatusCode, routing::get};
    use serde_json::{Value, json};

    fn config(validator: &Endpoint) -> Config {
        Config {
            validator_grpc: validator.clone(),
            validator_http: validator.clone(),
            observer_http: validator.clone(),
            output_dir: String::new(),
            verbose_logs: false,
            graphql_url: String::new(),
            graphql_timeout: std::time::Duration::from_secs(1),
            shard_id: "root".into(),
            expected_token_symbol: "ASI".into(),
            max_wait: 480,
            check_interval: 5,
        }
    }

    fn status() -> Value {
        json!({
            "version": {},
            "address": "fixture",
            "networkId": "test",
            "shardId": "root",
            "peers": 0,
            "nodes": 0,
            "minPhloPrice": "1",
            "nativeTokenSymbol": "ASI",
            "nativeTokenDecimals": 18,
        })
    }

    async fn validator_metadata_at(status: Value) -> Result<NodeStatus> {
        let router = Router::new().route(
            "/api/status",
            get(move || {
                let status = status.clone();
                async move {
                    (
                        StatusCode::OK,
                        [("content-type", "application/json")],
                        status.to_string(),
                    )
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint =
            Endpoint::parse(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let metadata = validator_metadata(&config(&endpoint)).await;
        server.abort();
        metadata
    }

    #[tokio::test]
    async fn an_unreported_validator_cap_is_rejected_after_valid_metadata() {
        let mut status = status();
        status["maxPhloLimit"] = Value::Null;
        let error = validator_metadata_at(status).await.unwrap_err();
        assert!(
            error
                .to_string()
                .contains("Validator has not reported its maximum Phlo limit yet"),
            "{error}"
        );
    }

    #[tokio::test]
    async fn a_wrong_network_is_reported_before_an_unreported_validator_cap() {
        let mut status = status();
        status["nativeTokenSymbol"] = json!("OTHER");
        status["maxPhloLimit"] = Value::Null;
        let error = validator_metadata_at(status).await.unwrap_err();
        let message = error.to_string();
        assert!(message.contains("Wrong native token"), "{message}");
        assert!(!message.contains("has not reported"), "{message}");
    }

    #[tokio::test]
    async fn a_validator_without_a_cap_field_keeps_no_reported_limit() {
        let validated = validator_metadata_at(status()).await.unwrap();
        assert_eq!(validated.max_phlo_limit, None);
        assert_eq!(validated.native_token_symbol, "ASI");
    }

    #[tokio::test]
    async fn a_reported_validator_cap_is_carried_through() {
        let mut status = status();
        status["maxPhloLimit"] = json!(250_000);
        let validated = validator_metadata_at(status).await.unwrap();
        assert_eq!(validated.max_phlo_limit, Some(250_000));
    }
}
