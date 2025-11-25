use crate::config::Config;
use anyhow::{Result, anyhow};
use node_cli::{args::WalletBalanceArgs, commands::wallet_balance_command};

pub async fn get_balance(config: &Config, wallet_address: String, silent: bool) -> Result<u64> {
    let args = &WalletBalanceArgs {
        host: config.observer_host.clone(),
        grpc_port: config.observer_grpc_port,
        address: wallet_address.to_string(),
        token: config.token.clone(),
    };

    let _gags = if silent {
        use gag::Gag;
        Some((Gag::stdout().ok(), Gag::stderr().ok()))
    } else {
        None
    };

    let result = wallet_balance_command(args).await;
    drop(_gags);

    match result {
        Ok((balance, _)) => Ok(balance.parse()?),
        Err(e) => {
            eprintln!("✖ Request error: {}", e);
            Err(anyhow!("Balance request failed: {}", e))
        }
    }
}

pub async fn handle_get_balance() -> Result<()> {
    let config = Config::from_env()?;
    let wallet_info = config.get_wallet()?;

    let balance = get_balance(&config, wallet_info.address, !config.verbose_logs).await?;
    println!("✔ Wallet balance: {}", balance);

    Ok(())
}
