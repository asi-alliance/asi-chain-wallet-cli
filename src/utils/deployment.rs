use crate::requests::graphql::build_recent_deployments_query;
use crate::utils::websocket::connect_graphql_websocket;
use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use futures_util::{SinkExt, Stream, StreamExt};
use tokio_tungstenite::tungstenite::{Error as WebSocketError, Message};

/*     Helpers to handle GraphQL requests     */
pub async fn fetch_recent_deployments(
    graphql_url: &str,
    address: &str,
    public_key: &str,
    limit: u32,
) -> Result<Vec<serde_json::Value>> {
    let (mut write, read) = connect_graphql_websocket(graphql_url).await?;

    let query_message = build_recent_deployments_query("init", "start", public_key, address, limit);
    write.send(Message::Text(query_message.to_string())).await?;

    read_export_response(read).await
}

async fn read_export_response(
    mut read: impl Stream<Item = Result<Message, WebSocketError>> + Unpin,
) -> Result<Vec<serde_json::Value>> {
    let mut deployments = None;
    while let Some(message) = read.next().await {
        let text = match message.context("Reading GraphQL export response")? {
            Message::Text(text) => text,
            Message::Close(_) => break,
            _ => continue,
        };
        let mut response: serde_json::Value = serde_json::from_str(&text)?;
        if response["type"] == "connection_error" {
            bail!("GraphQL connection error: {}", response["payload"]);
        }
        if response["id"] != "init" {
            continue;
        }
        match response["type"].as_str() {
            Some("error") => bail!("GraphQL export error: {}", response["payload"]),
            Some("data") => {
                let payload = &mut response["payload"];
                if let Some(errors) = payload.get("errors")
                    && !matches!(errors.as_array(), Some(list) if list.is_empty())
                {
                    bail!("GraphQL export error: {errors}");
                }
                let rows = payload
                    .pointer_mut("/data/deployments")
                    .context("GraphQL export response is missing deployments")?;
                let mut rows: Vec<serde_json::Value> = serde_json::from_value(rows.take())
                    .context("Invalid deployments in GraphQL export response")?;
                rows.reverse();
                deployments = Some(rows);
            }
            Some("complete") => {
                return deployments.context("GraphQL export completed without data");
            }
            other => bail!("Unexpected GraphQL export message type: {other:?}"),
        }
    }
    bail!("GraphQL connection closed before the export completed")
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

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::stream;
    use serde_json::{Value, json};

    async fn export(messages: Vec<Value>) -> Result<Vec<Value>> {
        let frames = messages
            .into_iter()
            .map(|msg| Ok(Message::Text(msg.to_string())));
        read_export_response(stream::iter(frames)).await
    }

    #[tokio::test]
    async fn requires_data_and_completion_even_for_an_empty_export() {
        let data = json!({"id":"init","type":"data","payload":{"data":{"deployments":[]}}});
        let complete = json!({"id":"init","type":"complete"});
        assert!(
            export(vec![data.clone(), complete.clone()])
                .await
                .unwrap()
                .is_empty()
        );
        assert!(export(vec![complete]).await.is_err());
        assert!(export(vec![data]).await.is_err());
    }

    #[tokio::test]
    async fn reports_operation_and_payload_errors_even_with_partial_data() {
        let errors = json!([{"message":"unavailable"}]);
        for frame in [
            json!({"id":"init","type":"error","payload":errors}),
            json!({"id":"init","type":"data","payload":{"data":{"deployments":[]},"errors":errors}}),
        ] {
            let err = export(vec![frame]).await.unwrap_err();
            assert!(err.to_string().contains("unavailable"));
        }
        let invalid = json!({"id":"init","type":"data","payload":{"data":{}}});
        assert!(export(vec![invalid]).await.is_err());
    }
}
