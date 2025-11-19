use crate::config::Config;
use crate::models::WalletInfo;
use anyhow::Result;
use node_cli::utils::CryptoUtils;
use std::fs;
use std::path::Path;

pub fn handle_create_wallet(save: bool) -> Result<()> {
    let config = Config::from_env()?;

    let (secret_key, public_key) = CryptoUtils::generate_key_pair()?;
    let private_key_hex = CryptoUtils::serialize_private_key(&secret_key);
    let pub_key_hex = CryptoUtils::serialize_public_key(&public_key, false);
    let address = CryptoUtils::generate_address(&pub_key_hex)?;

    println!("✔ Wallet created successfully!");
    println!("Private key: {}", private_key_hex);
    println!("Public key: {}", pub_key_hex);
    println!("Address: {}", address);

    if save {
        let output_dir = Path::new(&config.output_dir);

        if !output_dir.exists() {
            fs::create_dir_all(output_dir)?;
        }

        let wallet_info = WalletInfo {
            private_key: private_key_hex,
            public_key: pub_key_hex,
            address: address.clone(),
        };

        let wallet_file = output_dir.join("wallet_info.json");
        let json_content = serde_json::to_string_pretty(&wallet_info)?;
        fs::write(&wallet_file, json_content)?;

        println!("\nWallet info saved to: {}", wallet_file.display());
    }

    Ok(())
}
