use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Clone)]
pub struct WalletInfo {
    pub private_key: String,
    pub public_key: String,
    pub address: String,
}

#[derive(Debug)]
pub struct Config {
    pub validator_host: String,
    pub validator_grpc_port: u16,
    pub validator_http_port: u16,

    pub observer_host: String,
    pub observer_grpc_port: u16,

    pub output_dir: String,
    pub verbose_logs: bool,
    pub graphql_url: String,
}

impl Clone for Config {
    fn clone(&self) -> Self {
        Self {
            validator_host: self.validator_host.clone(),
            validator_grpc_port: self.validator_grpc_port,
            validator_http_port: self.validator_http_port,
            observer_host: self.observer_host.clone(),
            observer_grpc_port: self.observer_grpc_port,
            output_dir: self.output_dir.clone(),
            verbose_logs: self.verbose_logs,
            graphql_url: self.graphql_url.clone(),
        }
    }
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            validator_host: env::var("VALIDATOR_HOST")
                .context("VALIDATOR_HOST is not set in .env")?,
            validator_grpc_port: env::var("VALIDATOR_GRPC_PORT")
                .context("VALIDATOR_GRPC_PORT is not set in .env")?
                .parse()
                .context("Invalid VALIDATOR_GRPC_PORT value")?,
            validator_http_port: env::var("VALIDATOR_HTTP_PORT")
                .context("VALIDATOR_HTTP_PORT is not set in .env")?
                .parse()
                .context("Invalid VALIDATOR_HTTP_PORT value")?,

            observer_host: env::var("OBSERVER_HOST").context("OBSERVER_HOST is not set in .env")?,
            observer_grpc_port: env::var("OBSERVER_GRPC_PORT")
                .context("OBSERVER_GRPC_PORT is not set in .env")?
                .parse()
                .context("Invalid OBSERVER_GRPC_PORT value")?,

            output_dir: env::var("OUTPUT_DIR").context("OUTPUT_DIR is not set in .env")?,
            verbose_logs: env::var("VERBOSE_LOGS")
                .context("VERBOSE_LOGS is not set in .env")?
                .parse()
                .context("Invalid VERBOSE_LOGS value (expected true/false)")?,

            graphql_url: env::var("GRAPHQL_URL").context("GRAPHQL_URL is not set in .env")?,
        })
    }

    pub fn get_wallet(&self) -> Result<WalletInfo> {
        let wallet_file = Path::new(&self.output_dir).join("wallet_info.json");
        let wallet_json = fs::read_to_string(&wallet_file).context(format!(
            "Failed to read wallet info from {}",
            wallet_file.display()
        ))?;

        let wallet_info: WalletInfo =
            serde_json::from_str(&wallet_json).context("Failed to parse wallet_info.json")?;

        Ok(wallet_info)
    }
}
