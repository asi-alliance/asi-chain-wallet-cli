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

pub struct Config {
    pub validator_host: String,
    pub validator_grpc_port: u16,
    pub validator_http_port: u16,

    pub observer_host: String,
    pub observer_grpc_port: u16,

    pub output_dir: String,
    pub token: String,
    pub verbose_logs: bool,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            validator_host: env::var("VALIDATOR_HOST").unwrap_or_else(|_| "localhost".to_string()),
            validator_grpc_port: env::var("VALIDATOR_GRPC_PORT")
                .unwrap_or_else(|_| "40412".to_string())
                .parse()
                .context("Invalid VALIDATOR_GRPC_PORT value")?,
            validator_http_port: env::var("VALIDATOR_HTTP_PORT")
                .unwrap_or_else(|_| "40413".to_string())
                .parse()
                .context("Invalid VALIDATOR_HTTP_PORT value")?,

            observer_host: env::var("OBSERVER_HOST").unwrap_or_else(|_| "localhost".to_string()),
            observer_grpc_port: env::var("OBSERVER_GRPC_PORT")
                .unwrap_or_else(|_| "40452".to_string())
                .parse()
                .context("Invalid OBSERVER_GRPC_PORT value")?,

            output_dir: env::var("OUTPUT_DIR").unwrap_or_else(|_| "./credentials".to_string()),
            token: env::var("TOKEN").unwrap_or_else(|_| "ASI".to_string()),

            verbose_logs: env::var("VERBOSE_LOGS")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
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
