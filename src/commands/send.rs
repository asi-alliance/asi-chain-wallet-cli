use crate::balance::{get_balance, metadata, metadata_at};
use crate::config::Config;
use anyhow::{Result, anyhow, ensure};
use node_cli::{ConnectionConfig, DeployOptions, F1r3flyConnectionManager};
use num_bigint::BigUint;

fn transfer_amount(value: &str) -> Result<BigUint> {
    let significant = value.trim_start_matches('0');
    ensure!(
        value.is_empty() || !significant.is_empty(),
        "Amount must be greater than zero"
    );
    Ok(node_cli::vault::parse_amount(significant)?)
}

fn transfer_deploy_options(min_phlo_price: BigUint, expiration: i64) -> DeployOptions {
    DeployOptions {
        phlo_limit: node_cli::vault::TRANSFER_PHLO_LIMIT,
        phlo_price: min_phlo_price.max(1u8.into()),
        expiration: Some(expiration),
        ..DeployOptions::default()
    }
}

fn send_error(id: &str, error: node_cli::Error) -> anyhow::Error {
    match error {
        node_cli::Error::DeployFailed { .. }
        | node_cli::Error::DeployExpired { .. }
        | node_cli::Error::NodeRejected { .. }
        | node_cli::Error::ContractRejected { .. } => {
            anyhow!("Transfer failed for deploy {id}: {error}")
        }
        _ => anyhow!(
            "Transfer outcome unknown for deploy {id}: {error}. Check this deploy ID before creating another payment."
        ),
    }
}

pub async fn handle_send(to: &str, value: &str) -> Result<()> {
    let amount = transfer_amount(value)?;
    node_cli::vault::validate_address(to)?;
    let config = Config::from_env()?;
    let (wallet, key) = config.signing_wallet()?;
    let token = metadata(&config).await?;
    let validator = metadata_at(&config, &config.validator_http).await?;
    let balance = get_balance(&config, &wallet.address).await?;
    ensure!(
        balance.dust >= amount,
        "Insufficient balance to complete the transfer"
    );
    let mut connection = ConnectionConfig::new(
        config.validator_grpc.clone(),
        config.validator_http.clone(),
        key.clone(),
    )
    .with_observer(config.observer_http.clone());
    connection.shard_id = config.shard_id.clone();
    connection.finalization_timeout_secs = config.max_wait;
    connection.poll_interval_secs = config.check_interval;
    let manager = F1r3flyConnectionManager::new(connection)?;
    let term = node_cli::vault::build_transfer_rholang(&wallet.address, to, &amount)?;
    let options = transfer_deploy_options(
        validator.min_phlo_price,
        chrono::Utc::now().timestamp_millis() + 30 * 60 * 1000,
    );
    let signed = manager
        .api()
        .prepare(&term, &options)
        .await?
        .sign(key.secret_key());
    let id = signed.id();
    // The ID is known before submission, including when the response is lost.
    println!("Deploy ID: {id}");
    println!(
        "Sending {amount} dust of {} to {to}",
        token.native_token_symbol
    );
    if config.verbose_logs {
        println!("Waiting for canonical finalization and the vault verdict");
    }
    let result = manager
        .submit_and_wait(&signed)
        .await
        .map_err(|e| send_error(&id, e))?;
    let block = result.block_hash.clone();
    node_cli::vault::check_transfer_result(result).map_err(|e| send_error(&id, e))?;
    println!("Block hash: {block}");
    println!("Transfer complete.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn amounts_are_exact_through_uint256_without_floats() {
        for value in [
            "9007199254740993",
            "18446744073709551617",
            "115792089237316195423570985008687907853269984665640564039457584007913129639935",
        ] {
            assert_eq!(transfer_amount(value).unwrap().to_string(), value);
        }
        assert_eq!(transfer_amount("0001").unwrap(), BigUint::from(1u8));
    }
    #[test]
    fn zero_negative_fractional_and_overflow_amounts_are_rejected() {
        for value in [
            "0",
            "000",
            "-1",
            "1.5",
            "1e8",
            "",
            "115792089237316195423570985008687907853269984665640564039457584007913129639936",
        ] {
            assert!(transfer_amount(value).is_err());
        }
    }
    #[test]
    fn accepts_a_validator_phlo_price_of_two_to_the_63() {
        let price = BigUint::from(1u64 << 63);
        let options = transfer_deploy_options(price.clone(), 1);
        assert_eq!(options.phlo_price, price);
        node_cli::UnsignedDeploy::new("Nil", &options, 1, "root").unwrap();
    }
    #[test]
    fn a_timeout_is_an_error_with_the_original_deploy_id() {
        let err = send_error(
            "original-id",
            node_cli::Error::FinalizationTimeout {
                deploy_id: "original-id".into(),
                waited: std::time::Duration::from_secs(1),
                execution: None,
            },
        );
        assert!(err.to_string().contains("outcome unknown"));
        assert!(err.to_string().contains("original-id"));
    }
    #[test]
    fn a_contract_rejection_is_never_a_success() {
        let err = send_error(
            "original-id",
            node_cli::Error::ContractRejected {
                contract: node_cli::Contract::Vault,
                reason: "Insufficient funds".into(),
            },
        );
        assert!(err.to_string().contains("Transfer failed"));
    }
}
