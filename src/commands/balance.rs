use crate::config::Config;
use anyhow::Result;
use node_cli::{args::WalletBalanceArgs, commands::wallet_balance_command};

pub async fn handle_get_balance() -> Result<()> {
    let config = Config::from_env()?;
    let wallet_info = config.get_wallet()?;

    let args = &WalletBalanceArgs {
        host: config.observer_host.clone(),
        grpc_port: config.observer_grpc_port,
        address: wallet_info.address.clone(),
        token: config.token.clone(),
    };

    let _gags = if !config.verbose_logs {
        use gag::Gag;
        Some((Gag::stdout().ok(), Gag::stderr().ok()))
    } else {
        None
    };

    let result = wallet_balance_command(args).await;
    drop(_gags);

    match result {
        Ok((balance, _)) => {
            println!("✔ Wallet balance: {}", balance);
        }
        Err(e) => {
            eprintln!("✖ Request error: {}", e);
        }
    }

    Ok(())
}
