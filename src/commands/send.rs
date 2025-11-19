use crate::config::Config;
use anyhow::{Result, anyhow};
use node_cli::{args::TransferArgs, commands::transfer_command, utils::CompressedDeployStatus};

//TO-DO: Balance check before sending
pub async fn handle_send(to: &str, amount: u64) -> Result<()> {
    if to.is_empty() {
        return Err(anyhow!("Recipient address cannot be empty"));
    }

    if amount <= 0 {
        return Err(anyhow!("Amount must be greater than 0"));
    }

    let config = Config::from_env()?;
    let wallet_info = config.get_wallet()?;

    println!("Sending {} {} to {}...", amount, config.token, to);

    let args = TransferArgs {
        to_address: to.to_string(),
        amount,
        private_key: wallet_info.private_key,
        host: config.validator_host,
        grpc_port: config.validator_grpc_port,
        http_port: config.validator_http_port,
        bigger_phlo: true,
        propose: false,
        max_wait: 480,
        check_interval: 10,
        observer_host: Some(config.observer_host),
        observer_grpc_port: Some(config.observer_grpc_port),
        token: config.token.clone(),
    };

    let _gags = if !config.verbose_logs {
        use gag::Gag;
        Some((Gag::stdout().ok(), Gag::stderr().ok()))
    } else {
        None
    };

    let result = transfer_command(&args).await;
    drop(_gags);

    match result {
        Ok((deploy_info, deploy_id)) => {
            println!("Deploy ID: {}", deploy_id);
            if *deploy_info.status() == CompressedDeployStatus::Deploying {
                println!("⚠ Deploy is sent but not yet included in block");
                println!(" This may indicate network delay or insufficient wait time");
                println!(
                    " Consider increasing the wait time or checking the network/deploy_id status"
                );
            } else if *deploy_info.status() == CompressedDeployStatus::Finalizing {
                println!("⚠ Deploy is sent and included in block, but not finalized yet");
                println!(
                    " Consider increasing the wait time or checking the network/deploy_id status"
                );
            } else if *deploy_info.status() == CompressedDeployStatus::Finalized {
                println!("✔ Transfer deploy completed successfully");
            } else {
                let error_msg = deploy_info.msg().unwrap_or("Unknown error");
                return Err(anyhow!("✖ Transfer deploy failed: {}", error_msg));
            }
        }
        Err(e) => {
            return Err(anyhow!("✖ Transfer failed: {}", e));
        }
    }

    Ok(())
}
