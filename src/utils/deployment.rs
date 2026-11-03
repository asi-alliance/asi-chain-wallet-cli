use crate::requests::graphql::build_recent_deployments_query;
use crate::utils::websocket::connect_graphql_websocket;
use anyhow::Result;
use chrono::{DateTime, Utc};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

/*     Helpers to handle GraphQL requests     */
pub async fn fetch_recent_deployments(
    graphql_url: &str,
    address: &str,
    public_key: &str,
    limit: u32,
) -> Result<Vec<serde_json::Value>> {
    let (mut write, mut read) = connect_graphql_websocket(graphql_url).await?;

    let query_message = build_recent_deployments_query("init", "start", public_key, address, limit);
    write.send(Message::Text(query_message.to_string())).await?;

    let mut deployments = Vec::new();

    while let Some(msg) = read.next().await {
        if let Ok(Message::Text(text)) = msg
            && let Ok(response) = serde_json::from_str::<serde_json::Value>(&text)
        {
            let msg_type = response.get("type").and_then(|v| v.as_str());
            let msg_id = response.get("id").and_then(|v| v.as_str());

            if msg_id == Some("init") {
                match msg_type {
                    Some("data") => {
                        if let Some(payload) = response.get("payload")
                            && let Some(data) = payload.get("data")
                            && let Some(deployments_data) = data.get("deployments")
                            && let Ok(mut deps) = serde_json::from_value::<Vec<serde_json::Value>>(
                                deployments_data.clone(),
                            )
                        {
                            deps.reverse();
                            deployments = deps;
                        }
                    }
                    Some("complete") => break,
                    _ => {}
                }
            }
        }
    }

    Ok(deployments)
}

pub fn get_deployment_status(deployment: &serde_json::Value) -> String {
    let status = deployment
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let finalize_status = deployment
        .get("block")
        .and_then(|b| b.get("finalization_status"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    if status == "included" && (finalize_status != "unknown" && finalize_status != "null") {
        // block with deploy can be finalized a little bit later
        finalize_status.to_string()
    } else {
        status.to_string()
    }
}

pub fn format_deployment_timestamp(deployment: &serde_json::Value) -> String {
    let created_at = deployment
        .get("created_at")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if !created_at.is_empty() {
        if let Ok(parsed_timestamp) = created_at.parse::<i64>() {
            let dt = DateTime::<Utc>::from_timestamp(parsed_timestamp / 1000, 0)
                .unwrap_or_else(Utc::now);
            return dt.format("%Y-%m-%d %H:%M:%S UTC").to_string();
        }
        created_at.to_string()
    } else {
        "N/A".to_string()
    }
}

pub fn get_transfer_direction(from_address: &str, public_key: &str) -> &'static str {
    if from_address == public_key {
        "OUT"
    } else {
        "IN"
    }
}

pub fn get_deployment_field(deployment: &serde_json::Value, field: &str, default: &str) -> String {
    deployment
        .get(field)
        .and_then(|v| v.as_str())
        .unwrap_or(default)
        .to_string()
}
