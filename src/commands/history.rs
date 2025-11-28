use crate::config::Config;
use crate::requests::graphql::{
    build_new_deployments_subscription, build_recent_deployments_query,
};
use crate::utils::deployment::{
    format_deployment_timestamp, get_deployment_field, get_deployment_status,
};
use crate::utils::websocket::connect_graphql_websocket;
use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use std::collections::HashSet;
use tokio_tungstenite::tungstenite::protocol::Message;

pub async fn handle_get_history(config: &Config, limit: Option<u32>) -> Result<()> {
    let wallet = config.get_wallet()?;
    let limit = limit.unwrap_or(10);

    println!("\n>>> Starting real-time deploy monitoring...");
    println!("Wallet: {}", wallet.address);
    println!("Public Key: {}", wallet.public_key);

    let graphql_url = config.graphql_url.clone();
    let address = wallet.address.clone();
    let public_key = wallet.public_key.clone();

    // deployments with nested transfers for complete deploy history
    subscribe_deployments(&graphql_url, &address, &public_key, limit).await?;

    Ok(())
}

async fn subscribe_deployments(
    graphql_url: &str,
    address: &str,
    public_key: &str,
    limit: u32,
) -> Result<()> {
    let (mut write, mut read) = connect_graphql_websocket(graphql_url).await?;
    // Query last deployments by `limit`
    let initial_message =
        build_recent_deployments_query("init", "start", public_key, address, limit);

    write
        .send(Message::Text(initial_message.to_string()))
        .await
        .context("Failed to send initial query")?;

    // Display deployments of recent history
    let mut initial_received = false;
    let mut last_timestamp = chrono::Utc::now().to_rfc3339();

    while let Some(msg) = read.next().await {
        if let Ok(Message::Text(text)) = msg {
            if let Ok(response) = serde_json::from_str::<serde_json::Value>(&text) {
                let msg_type = response.get("type").and_then(|v| v.as_str());
                let msg_id = response.get("id").and_then(|v| v.as_str());

                if msg_id == Some("init") {
                    match msg_type {
                        Some("data") => {
                            if let Some(payload) = response.get("payload") {
                                if let Some(data) = payload.get("data") {
                                    if let Some(deployments_data) = data.get("deployments") {
                                        if let Ok(mut deployments) =
                                            serde_json::from_value::<Vec<serde_json::Value>>(
                                                deployments_data.clone(),
                                            )
                                        {
                                            deployments.reverse();

                                            println!("{}", "=".repeat(80));
                                            println!("\nRecent History (last {}):\n", limit);

                                            for deployment in &deployments {
                                                display_deployment(deployment, address);

                                                if let Some(created) = deployment
                                                    .get("created_at")
                                                    .and_then(|v| v.as_str())
                                                {
                                                    last_timestamp = created.to_string();
                                                }
                                            }

                                            if deployments.is_empty() {
                                                println!("No recent deployments found");
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Some("complete") => {
                            initial_received = true;
                            println!("\n>>> Monitoring for new deployments...");
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    if !initial_received {
        return Err(anyhow::anyhow!("Failed to receive initial deployments"));
    }

    // Now subscribe to new deployments only
    let subscribe_message =
        build_new_deployments_subscription("1", "start", public_key, address, &last_timestamp);

    write
        .send(Message::Text(subscribe_message.to_string()))
        .await
        .context("Failed to send subscription")?;

    let mut displayed_deploys: HashSet<String> = HashSet::new();

    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(response) = serde_json::from_str::<serde_json::Value>(&text) {
                    let msg_type = response.get("type").and_then(|v| v.as_str());
                    let msg_id = response.get("id").and_then(|v| v.as_str());

                    // process messages for the subscription
                    if msg_id == Some("1") {
                        match msg_type {
                            Some("data") => {
                                if let Some(payload) = response.get("payload") {
                                    if let Some(data) = payload.get("data") {
                                        if let Some(deployments_data) = data.get("deployments") {
                                            if let Ok(deployments) =
                                                serde_json::from_value::<Vec<serde_json::Value>>(
                                                    deployments_data.clone(),
                                                )
                                            {
                                                for deployment in deployments {
                                                    let deploy_id = deployment
                                                        .get("deploy_id")
                                                        .and_then(|v| v.as_str())
                                                        .unwrap_or("");

                                                    if !displayed_deploys.contains(deploy_id) {
                                                        display_deployment(&deployment, address);
                                                        displayed_deploys
                                                            .insert(deploy_id.to_string());
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Some("error") => {
                                if let Some(payload) = response.get("payload") {
                                    println!("✖ Subscription error: {:?}", payload);
                                }
                            }
                            Some("complete") => {
                                println!("✔ Subscription completed");
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }
            Ok(Message::Close(_)) => {
                println!("WebSocket connection closed");
                break;
            }
            Err(e) => {
                return Err(anyhow::anyhow!("WebSocket error: {}", e));
            }
            _ => {}
        }
    }

    Ok(())
}

fn display_deployment(deployment: &serde_json::Value, wallet_address: &str) {
    println!("{}", "=".repeat(80));

    let deploy_id = get_deployment_field(deployment, "deploy_id", "unknown");
    let block_number = get_deployment_field(deployment, "block_number", "0");
    let status = get_deployment_status(deployment);
    let formatted_date = format_deployment_timestamp(deployment);

    println!("Deploy ID: {}", deploy_id);
    println!("Block: #{}", block_number);
    println!("Status: {}", status);
    println!("Created at: {}", formatted_date);

    if let Some(transfers) = deployment.get("transfers").and_then(|v| v.as_array()) {
        if !transfers.is_empty() {
            println!("\n 💳  Associated Transfers:");
            for transfer in transfers {
                let from_address = get_deployment_field(transfer, "from_address", "unknown");
                let to_address = get_deployment_field(transfer, "to_address", "unknown");
                let amount_dust = get_deployment_field(transfer, "amount_dust", "0");

                if from_address == wallet_address {
                    println!("  → OUT | {} dust", amount_dust);
                    println!("     To: {}", to_address);
                } else {
                    println!("  ← IN  | {} dust", amount_dust);
                    println!("     From: {}", from_address);
                }
            }
        }
    }
}
