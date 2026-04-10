use crate::balance::get_balance;
use crate::config::Config;
use crate::utils::parse_deploy_status;
use anyhow::{Result, anyhow};
use node_cli::{args::TransferArgs, commands::transfer_command, utils::DeployCompressedInfo};

async fn check_send_conditions(
    config: &Config,
    wallet_address: String,
    amount: u64,
    to: &str,
) -> Result<()> {
    if get_balance(config, wallet_address, true).await? < amount {
        return Err(anyhow!("Insufficient balance to complete the transfer"));
    }
    if to.is_empty() {
        return Err(anyhow!("Recipient address cannot be empty"));
    }
    if amount <= 0 {
        return Err(anyhow!("Amount must be greater than 0"));
    }

    Ok(())
}

pub async fn send(
    config: &Config,
    private_key: String,
    to: &str,
    amount: u64,
    silent: bool,
) -> Result<(DeployCompressedInfo, String)> {
    let args = TransferArgs {
        to_address: to.to_string(),
        amount,
        private_key,
        host: config.validator_host.clone(),
        grpc_port: config.validator_grpc_port,
        http_port: config.validator_http_port,
        bigger_phlo: true,
        propose: false,
        max_wait: 480,
        check_interval: 10,
        observer_host: Some(config.observer_host.clone()),
        observer_grpc_port: Some(config.observer_grpc_port),
        expires_in: Some(30 * 60), // 30 minutes
        expiration: None,
    };

    let _gags = if silent {
        use gag::Gag;
        Some((Gag::stdout().ok(), Gag::stderr().ok()))
    } else {
        None
    };

    let result = transfer_command(&args).await;
    drop(_gags);

    result.map_err(|e| anyhow!("✖ Transfer failed: {}", e))
}

pub async fn handle_send(to: &str, amount: u64) -> Result<()> {
    let config = Config::from_env()?;
    let wallet_info = config.get_wallet()?;

    check_send_conditions(&config, wallet_info.address, amount, to).await?;

    println!("Sending {} tokens to {}...", amount, to);

    let (deploy_info, deploy_id) = send(
        &config,
        wallet_info.private_key,
        to,
        amount,
        !config.verbose_logs,
    )
    .await?;

    println!("Deploy ID: {}", deploy_id);
    parse_deploy_status(&deploy_info)?;

    Ok(())
}
