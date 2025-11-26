use crate::config::Config;
use crate::requests::graphql::{
    build_new_deployments_subscription, build_recent_deployments_query,
};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::collections::HashSet;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, protocol::Message},
};

pub async fn handle_get_history(config: &Config, limit: Option<u32>) -> Result<()> {
    let wallet = config.get_wallet()?;
    let limit = limit.unwrap_or(10);

    println!("\n>>> Starting real-time deploy monitoring...");
    println!("Wallet: {}", wallet.address);
    println!("Public Key: {}", wallet.public_key);
    println!("{}", "=".repeat(80));

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
    let mut request = graphql_url.into_client_request()?;
    request
        .headers_mut()
        .insert("Sec-WebSocket-Protocol", "graphql-ws".parse().unwrap());

    println!(">>> Connecting to WebSocket...");

    let (ws_stream, _) = connect_async(request)
        .await
        .context("Failed to connect to WebSocket")?;

    let (mut write, mut read) = ws_stream.split();

    let connection_init = json!({
        "type": "connection_init",
        "payload": {}
    });
    write
        .send(Message::Text(connection_init.to_string()))
        .await
        .context("Failed to send connection_init")?;

    while let Some(msg) = read.next().await {
        if let Ok(Message::Text(text)) = msg {
            if let Ok(response) = serde_json::from_str::<serde_json::Value>(&text) {
                if response.get("type").and_then(|v| v.as_str()) == Some("connection_ack") {
                    println!("✓ Connected to WebSocket");
                    break;
                }
            }
        }
    }

    // Fetch last N deployments for specific wallet
    let initial_message =
        build_recent_deployments_query("init", "start", public_key, address, limit);

    write
        .send(Message::Text(initial_message.to_string()))
        .await
        .context("Failed to send initial query")?;

    // Process initial results
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
                                            println!("\n[HIST] Recent History (last {}):\n", limit);

                                            for deployment in &deployments {
                                                display_deployment(deployment, public_key);

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
                                                        display_deployment(&deployment, public_key);
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

fn display_deployment(deployment: &serde_json::Value, public_key: &str) {
    println!("{}", "=".repeat(80));

    let deploy_id = deployment
        .get("deploy_id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let block_number = deployment
        .get("block_number")
        .and_then(|v| v.as_str())
        .unwrap_or("0");
    let status = deployment
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let finalize_status = deployment
        .get("block")
        .and_then(|b| b.get("finalization_status"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let created_at = deployment
        .get("created_at")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let formatted_date = if !created_at.is_empty() {
        if let Ok(parsed_timestamp) = created_at.parse::<i64>() {
            let dt = DateTime::<Utc>::from_timestamp(parsed_timestamp / 1000, 0)
                .unwrap_or_else(|| Utc::now());
            dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
        } else {
            created_at.to_string()
        }
    } else {
        "N/A".to_string()
    };

    println!("Deploy ID: {}", deploy_id);
    println!("Block: #{}", block_number);
    println!(
        "Status: {}",
        if status == "included" {
            finalize_status
        } else {
            status
        }
    );
    println!("Created at: {}", formatted_date);

    if let Some(transfers) = deployment.get("transfers").and_then(|v| v.as_array()) {
        if !transfers.is_empty() {
            println!("\n 💳  Associated Transfers:");
            for transfer in transfers {
                let from_address = transfer
                    .get("from_address")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let to_address = transfer
                    .get("to_address")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let amount_dust = transfer
                    .get("amount_dust")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0");

                if from_address == public_key {
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
