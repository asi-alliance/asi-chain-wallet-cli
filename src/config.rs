use anyhow::{Context, Result, ensure};
use node_cli::{Endpoint, SigningKey, utils::CryptoUtils};
use std::{env, fs, path::Path, time::Duration};

use crate::models::WalletInfo;

#[derive(Debug, Clone)]
pub struct Config {
    pub validator_grpc: Endpoint,
    pub validator_http: Endpoint,
    pub observer_http: Endpoint,
    pub output_dir: String,
    pub verbose_logs: bool,
    pub graphql_url: String,
    pub shard_id: String,
    pub expected_token_symbol: String,
    pub max_wait: u32,
    pub check_interval: u64,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Self::from_lookup(|key| env::var(key).ok())
    }

    fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Self> {
        let required = |key: &str| get(key).with_context(|| format!("{key} is not set"));
        let positive = |key: &str, default: u64| -> Result<u64> {
            let value = get(key)
                .map(|s| s.parse::<u64>())
                .transpose()
                .with_context(|| format!("Invalid {key}"))?
                .unwrap_or(default);
            ensure!(value > 0, "{key} must be greater than zero");
            Ok(value)
        };
        let connect_timeout = Duration::from_secs(positive("NODE_CONNECT_TIMEOUT_SECONDS", 10)?);
        let timeout = Duration::from_secs(positive("NODE_TIMEOUT_SECONDS", 30)?);
        let ca = get("NODE_CA_CERT")
            .map(fs::read)
            .transpose()
            .context("Cannot read NODE_CA_CERT")?;
        let endpoint = |url_key: &str, host_key: &str, port_key: &str| -> Result<Endpoint> {
            let url = match get(url_key) {
                Some(url) => url,
                None => {
                    let host = required(host_key)?;
                    let port = required(port_key)?
                        .parse::<u16>()
                        .with_context(|| format!("Invalid {port_key}"))?;
                    ensure!(port > 0, "{port_key} must be greater than zero");
                    let host = if host.contains(':') && !host.starts_with('[') {
                        format!("[{host}]")
                    } else {
                        host
                    };
                    format!("http://{host}:{port}")
                }
            };
            let parsed = Endpoint::parse(&url)?
                .with_connect_timeout(connect_timeout)
                .with_request_timeout(timeout);
            Ok(match &ca {
                Some(pem) => parsed.with_ca_certificates(pem.clone())?,
                None => parsed,
            })
        };
        let max_wait = u32::try_from(positive("DEPLOY_MAX_WAIT_SECONDS", 480)?)
            .context("DEPLOY_MAX_WAIT_SECONDS exceeds u32")?;
        let check_interval = positive("DEPLOY_CHECK_INTERVAL_SECONDS", 5)?;
        ensure!(
            check_interval <= u64::from(max_wait),
            "Poll interval must not exceed the wait budget"
        );
        Ok(Self {
            validator_grpc: endpoint(
                "VALIDATOR_GRPC_URL",
                "VALIDATOR_HOST",
                "VALIDATOR_GRPC_PORT",
            )?,
            validator_http: endpoint(
                "VALIDATOR_HTTP_URL",
                "VALIDATOR_HOST",
                "VALIDATOR_HTTP_PORT",
            )?,
            observer_http: endpoint("OBSERVER_HTTP_URL", "OBSERVER_HOST", "OBSERVER_HTTP_PORT")?,
            output_dir: required("OUTPUT_DIR")?,
            verbose_logs: get("VERBOSE_LOGS")
                .unwrap_or_else(|| "false".into())
                .parse()
                .context("VERBOSE_LOGS must be true or false")?,
            graphql_url: get("GRAPHQL_URL").unwrap_or_default(),
            shard_id: get("SHARD_ID").unwrap_or_else(|| "root".into()),
            expected_token_symbol: get("EXPECTED_TOKEN_SYMBOL").unwrap_or_else(|| "ASI".into()),
            max_wait,
            check_interval,
        })
    }

    pub fn get_wallet(&self) -> Result<WalletInfo> {
        let wallet_file = Path::new(&self.output_dir).join("wallet_info.json");
        let json = fs::read_to_string(&wallet_file)
            .with_context(|| format!("Failed to read {}", wallet_file.display()))?;
        let wallet: WalletInfo = serde_json::from_str(&json).context("Invalid wallet_info.json")?;
        validate_wallet(&wallet)?;
        Ok(wallet)
    }
}

pub fn validate_wallet(wallet: &WalletInfo) -> Result<()> {
    let key = SigningKey::from_hex(&wallet.private_key)?;
    let public = CryptoUtils::serialize_public_key(&key.public_key(), false);
    let derived = CryptoUtils::generate_vault_address(&public)?;
    ensure!(
        derived == wallet.address,
        "Wallet private key does not match its address"
    );
    ensure!(
        CryptoUtils::generate_vault_address(&wallet.public_key)? == derived,
        "Wallet public key does not match its address"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    fn values() -> HashMap<&'static str, String> {
        [
            ("VALIDATOR_GRPC_URL", "http://localhost:40401"),
            ("VALIDATOR_HTTP_URL", "https://example.test/node"),
            ("OBSERVER_HTTP_URL", "http://localhost:40453"),
            ("OUTPUT_DIR", "credentials"),
        ]
        .into_iter()
        .map(|(k, v)| (k, v.into()))
        .collect()
    }
    #[test]
    fn explicit_urls_keep_base_paths_and_do_not_need_old_grpc_observer_port() {
        let env = values();
        let c = Config::from_lookup(|k| env.get(k).cloned()).unwrap();
        assert_eq!(c.validator_http.to_string(), "https://example.test/node");
        assert_eq!(c.observer_http.to_string(), "http://localhost:40453");
    }
    #[test]
    fn legacy_hosts_need_an_explicit_observer_http_port() {
        let mut env = values();
        env.remove("OBSERVER_HTTP_URL");
        env.insert("OBSERVER_HOST", "localhost".into());
        env.insert("OBSERVER_GRPC_PORT", "40452".into());
        assert!(Config::from_lookup(|k| env.get(k).cloned()).is_err());
        env.insert("OBSERVER_HTTP_PORT", "40453".into());
        assert!(Config::from_lookup(|k| env.get(k).cloned()).is_ok());
    }
    #[test]
    fn invalid_polling_budgets_fail_before_a_deploy() {
        for (key, value) in [
            ("DEPLOY_MAX_WAIT_SECONDS", "0"),
            ("DEPLOY_CHECK_INTERVAL_SECONDS", "0"),
            ("DEPLOY_CHECK_INTERVAL_SECONDS", "481"),
            ("NODE_TIMEOUT_SECONDS", "0"),
        ] {
            let mut env = values();
            env.insert(key, value.into());
            assert!(Config::from_lookup(|k| env.get(k).cloned()).is_err());
        }
    }
}
