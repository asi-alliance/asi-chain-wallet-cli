use crate::balance::{get_balance, metadata, metadata_at};
use crate::config::Config;
use anyhow::{Result, anyhow, ensure};
use node_cli::{
    ConnectionConfig, DeployAdmissionReason, DeployOptions, DeployResult, F1r3flyConnectionManager,
    QueueCongestion, SignedDeploy,
};
use num_bigint::BigUint;
use std::time::Duration;

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
        node_cli::Error::DeployAdmissionRefused {
            reason,
            congestion,
            retry_after,
            ..
        } => {
            let guidance = match (reason, congestion) {
                (DeployAdmissionReason::DeployerShareFull, _) => {
                    " Cause: deployer_share. Only retry the original signed deploy. Running send again signs a new transfer."
                }
                (DeployAdmissionReason::QueueFull, Some(QueueCongestion::Validator)) => {
                    " Cause: validator. Wait, then sign with a current block height."
                }
                (DeployAdmissionReason::QueueFull, Some(QueueCongestion::ValidatorIdle)) => {
                    " Cause: validator_idle. Wait or choose another validator, then sign with a current block height."
                }
                (DeployAdmissionReason::QueueFull, Some(QueueCongestion::Shard)) => {
                    " Cause: shard. Wait for shard finalization, then sign with a current block height."
                }
                (DeployAdmissionReason::QueueFull, Some(QueueCongestion::Storage)) => {
                    " Cause: storage. Wait for queue space, then sign with a current block height."
                }
                _ if reason.is_retryable() => " Please try again later.",
                _ => "",
            };
            let wait = retry_after
                .map_or_else(String::new, |d| format!(" Retry after {} s.", d.as_secs()));
            anyhow!("Transfer not accepted for deploy {id}: {error}.{wait}{guidance}")
        }
        node_cli::Error::DeployFailed { .. }
        | node_cli::Error::DeployExpired { .. }
        | node_cli::Error::DeployRejected { .. }
        | node_cli::Error::NodeRejected { .. }
        | node_cli::Error::ContractRejected { .. } => {
            anyhow!("Transfer failed for deploy {id}: {error}")
        }
        node_cli::Error::DeployResultUnreadable { .. } => anyhow!(
            "Transfer result unreadable for deploy {id}: {error}. Do not resubmit this deploy; read its result by deploy ID."
        ),
        _ => anyhow!(
            "Transfer outcome unknown for deploy {id}: {error}. Check this deploy ID before creating another payment."
        ),
    }
}

async fn submit_transfer(
    manager: &F1r3flyConnectionManager,
    signed: &SignedDeploy,
    expiration_ms: i64,
    retry_deployer_share: bool,
) -> node_cli::Result<DeployResult> {
    let first = manager.submit_and_wait(signed).await;
    let wait = match &first {
        Err(node_cli::Error::DeployAdmissionRefused {
            reason: DeployAdmissionReason::DeployerShareFull,
            retry_after: Some(wait),
            ..
        }) if retry_deployer_share => *wait,
        _ => return first,
    };
    let remaining_ms = expiration_ms.saturating_sub(chrono::Utc::now().timestamp_millis());
    if wait >= Duration::from_millis(remaining_ms.max(0) as u64) {
        eprintln!("Skipping retry: the signed transfer expires before the queue wait ends.");
        return first;
    }
    eprintln!(
        "Cause: deployer_share. Retrying signed deploy {} once after {} s.",
        signed.id(),
        wait.as_secs()
    );
    tokio::time::sleep(wait).await;
    if chrono::Utc::now().timestamp_millis() >= expiration_ms {
        eprintln!("Skipping retry: the signed transfer expired during the wait.");
        return first;
    }
    manager.submit_and_wait(signed).await
}

pub async fn handle_send(to: &str, value: &str, retry_deployer_share: bool) -> Result<()> {
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
    let expiration_ms = chrono::Utc::now().timestamp_millis() + 30 * 60 * 1000;
    let options = transfer_deploy_options(validator.min_phlo_price, expiration_ms);
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
    let result = submit_transfer(&manager, &signed, expiration_ms, retry_deployer_share)
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

    #[tokio::test]
    async fn only_an_opted_in_share_refusal_resends_the_original_bytes_once() {
        use axum::{Router, body::Bytes, http::HeaderMap, routing::post};
        use node_cli::{Endpoint, SigningKey, UnsignedDeploy};
        use tokio::{net::TcpListener, sync::mpsc};

        for (cause, wait, opted_in, expected_calls) in [
            ("deployer_share", Some("1"), true, 2),
            ("deployer_share", Some("0"), false, 1),
            ("deployer_share", None, true, 1),
            ("deployer_share", Some("3600"), true, 1),
            ("validator", Some("0"), true, 1),
            ("validator_idle", Some("0"), true, 1),
            ("shard", Some("0"), true, 1),
            ("storage", Some("0"), true, 1),
            ("future", Some("0"), true, 1),
        ] {
            let (tx, mut requests) = mpsc::unbounded_channel();
            let router = Router::new().route(
                "/casper.v1.DeployService/doDeploy",
                post(move |body: Bytes| {
                    tx.send((tokio::time::Instant::now(), body)).unwrap();
                    async move {
                        let mut headers = HeaderMap::new();
                        headers.insert("content-type", "application/grpc".parse().unwrap());
                        headers.insert("grpc-status", "8".parse().unwrap());
                        headers.insert("cause", cause.parse().unwrap());
                        if let Some(wait) = wait {
                            headers.insert("retry-after", wait.parse().unwrap());
                        }
                        headers
                    }
                }),
            );
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let endpoint =
                Endpoint::parse(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
            let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
            let key = SigningKey::from_hex(&"42".repeat(32)).unwrap();
            let expiration = chrono::Utc::now().timestamp_millis() + 60_000;
            let options = transfer_deploy_options(1u8.into(), expiration);
            let signed = UnsignedDeploy::new("Nil", &options, 1, "root")
                .unwrap()
                .sign(key.secret_key());
            let manager = F1r3flyConnectionManager::new(ConnectionConfig::new(
                endpoint.clone(),
                endpoint,
                key,
            ))
            .unwrap();
            let error = submit_transfer(&manager, &signed, expiration, opted_in)
                .await
                .unwrap_err();
            let message = send_error(&signed.id(), error).to_string();
            assert!(message.contains(&signed.id()), "{message}");
            assert!(message.contains("deploy_queue_full"), "{message}");
            if cause == "future" {
                assert!(!message.contains("Cause:"), "{message}");
            } else {
                assert!(message.contains(&format!("Cause: {cause}.")), "{message}");
            }
            if let Some(wait) = wait {
                assert!(
                    message.contains(&format!("Retry after {wait} s.")),
                    "{message}"
                );
            }
            assert_eq!(requests.len(), expected_calls, "{cause}");
            let (first_at, first) = requests.try_recv().unwrap();
            assert_eq!(&first[5..], signed.to_bytes()); // Skip the five-byte gRPC message header.
            if expected_calls == 2 {
                let (second_at, second) = requests.try_recv().unwrap();
                assert_eq!(first, second);
                assert!(second_at - first_at >= Duration::from_secs(1));
            }
            server.abort();
        }
    }
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
    fn admission_refusals_are_distinct_from_unknown_outcomes() {
        for (reason, retryable) in [
            (node_cli::DeployAdmissionReason::PoolBusy, true),
            (node_cli::DeployAdmissionReason::AdmissionBusy, true),
            (node_cli::DeployAdmissionReason::TooLarge, false),
        ] {
            let err = send_error(
                "original-id",
                node_cli::Error::DeployAdmissionRefused {
                    deploy_id: "original-id".into(),
                    reason,
                    message: "busy".into(),
                    retry_after: None,
                    congestion: None,
                },
            );
            let message = err.to_string();
            assert!(message.contains("not accepted"));
            assert!(message.contains("original-id"));
            assert!(message.contains(reason.code()));
            assert_eq!(message.contains("Please try again later"), retryable);
            assert!(!message.contains("outcome unknown"));
        }
    }
    #[test]
    fn a_rejection_after_queue_admission_is_a_terminal_failure() {
        let detail = "term could not be compiled";
        let err = send_error(
            "original-id",
            node_cli::Error::DeployRejected {
                deploy_id: "original-id".into(),
                detail: Some(detail.into()),
            },
        );
        let message = err.to_string();
        assert!(message.starts_with("Transfer failed for deploy original-id:"));
        assert!(message.contains(detail));
    }
    #[test]
    fn a_finalized_deploy_with_an_unreadable_result_is_not_resubmitted() {
        let err = send_error(
            "original-id",
            node_cli::Error::DeployResultUnreadable {
                deploy_id: "original-id".into(),
                block_hash: "block-hash".into(),
                cause: Box::new(node_cli::Error::Transport {
                    endpoint: "http://node:40403".into(),
                    cause: node_cli::TransportError::TimedOut(std::time::Duration::from_secs(1)),
                }),
            },
        );
        let message = err.to_string();
        assert!(message.contains("original-id"));
        assert!(message.contains("block-hash"));
        assert!(message.contains("Do not resubmit"));
        assert!(!message.contains("creating another payment"));
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
