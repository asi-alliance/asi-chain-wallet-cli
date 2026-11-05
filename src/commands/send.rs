use crate::balance::{get_balance, metadata};
use crate::config::Config;
use anyhow::{Result, anyhow, ensure};
use node_cli::{ConnectionConfig, DeployOptions, F1r3flyConnectionManager, SigningKey};
use num_bigint::BigUint;

fn transfer_amount(value: &str) -> Result<BigUint> {
    let amount = node_cli::vault::parse_amount(value)?;
    ensure!(
        amount != BigUint::from(0u8),
        "Amount must be greater than zero"
    );
    Ok(amount)
}

fn send_error(id: &str, error: node_cli::Error) -> anyhow::Error {
    match error {
        node_cli::Error::DeployAdmissionRefused { .. } => anyhow!(
            "Transfer not accepted by the node for deploy {id}: {error}. Please try again later."
        ),
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
    let wallet = config.get_wallet()?;
    let token = metadata(&config).await?;
    let balance = get_balance(&config, &wallet.address).await?;
    ensure!(
        balance.dust >= amount,
        "Insufficient balance to complete the transfer"
    );
    let key = SigningKey::from_hex(&wallet.private_key)?;
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
    let mut options =
        DeployOptions::default().with_phlo_limit(node_cli::vault::TRANSFER_PHLO_LIMIT);
    options.expiration = Some(chrono::Utc::now().timestamp_millis() + 30 * 60 * 1000);
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
    }
    #[test]
    fn zero_negative_fractional_and_overflow_amounts_are_rejected() {
        for value in [
            "0",
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
    fn admission_refusals_are_distinct_from_unknown_outcomes() {
        for reason in [
            node_cli::DeployAdmissionReason::PoolBusy,
            node_cli::DeployAdmissionReason::AdmissionBusy,
        ] {
            let err = send_error(
                "original-id",
                node_cli::Error::DeployAdmissionRefused {
                    deploy_id: "original-id".into(),
                    reason,
                    message: "busy".into(),
                },
            );
            let message = err.to_string();
            assert!(message.contains("not accepted"));
            assert!(message.contains("original-id"));
            assert!(message.contains(reason.code()));
            assert!(!message.contains("outcome unknown"));
        }
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
