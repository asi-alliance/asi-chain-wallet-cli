use crate::config::Config;
use crate::models::{WalletIdentity, WalletInfo};
use anyhow::{Context, Result};
use node_cli::utils::CryptoUtils;
use std::{fs, path::Path};

pub fn handle_create_wallet(save: bool) -> Result<()> {
    let config = Config::from_env()?;
    let (secret_key, public_key) = CryptoUtils::generate_key_pair();
    let public_key = CryptoUtils::serialize_public_key(&public_key, false);
    let address = CryptoUtils::generate_vault_address(&public_key)?;
    let wallet = WalletInfo {
        private_key: CryptoUtils::serialize_private_key(&secret_key),
        identity: WalletIdentity {
            public_key,
            address,
        },
    };
    let output_dir = Path::new(&config.output_dir);
    let wallet_file = output_dir.join("wallet_info.json");
    if save {
        fs::create_dir_all(output_dir)?;
        let mut temporary = tempfile::NamedTempFile::new_in(output_dir)?;
        serde_json::to_writer_pretty(&mut temporary, &wallet)?;
        temporary.as_file().sync_all()?;
        temporary
            .persist_noclobber(&wallet_file)
            .with_context(|| format!("Could not save a new wallet at {}", wallet_file.display()))?;
    }

    println!("✔ Wallet created successfully!");
    println!("Private key: {}", wallet.private_key);
    println!("Public key: {}", wallet.identity.public_key);
    println!("Address: {}", wallet.identity.address);
    if save {
        println!("\nWallet info saved to: {}", wallet_file.display());
    }
    Ok(())
}
