use anyhow::{Context, Result, bail};
use futures_util::{SinkExt, StreamExt, stream::SplitSink, stream::SplitStream};
use serde_json::json;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{client::IntoClientRequest, protocol::Message},
};

pub type WsSink = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
pub type WsStream = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

pub async fn connect_graphql_websocket(
    graphql_url: &str,
    timeout: Duration,
) -> Result<(WsSink, WsStream)> {
    tokio::time::timeout(timeout, connect(graphql_url))
        .await
        .context("Timed out waiting for GraphQL connection acknowledgement")?
}

async fn connect(graphql_url: &str) -> Result<(WsSink, WsStream)> {
    println!("\n>>> Connecting to WebSocket...");
    let mut request = graphql_url.into_client_request()?;
    request
        .headers_mut()
        .insert("Sec-WebSocket-Protocol", "graphql-ws".parse().unwrap());

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

    while let Some(message) = read.next().await {
        let message = message.context("Reading GraphQL connection acknowledgement")?;
        match message {
            Message::Text(text) => {
                let response: serde_json::Value =
                    serde_json::from_str(&text).context("Invalid GraphQL connection response")?;
                match response.get("type").and_then(|value| value.as_str()) {
                    Some("connection_ack") => {
                        println!("✓ Connected to WebSocket");
                        return Ok((write, read));
                    }
                    Some("connection_error") | Some("error") => {
                        bail!("GraphQL connection error: {}", response["payload"]);
                    }
                    Some("ka") => {}
                    other => bail!("Unexpected GraphQL connection message: {other:?}"),
                }
            }
            Message::Ping(_) | Message::Pong(_) => {}
            Message::Close(_) => break,
            _ => bail!("Expected a GraphQL text response"),
        }
    }
    bail!("GraphQL connection closed before acknowledgement")
}
