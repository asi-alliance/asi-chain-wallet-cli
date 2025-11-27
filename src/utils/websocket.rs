use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt, stream::SplitSink, stream::SplitStream};
use serde_json::json;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{client::IntoClientRequest, protocol::Message},
};

pub type WsSink = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
pub type WsStream = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

pub async fn connect_graphql_websocket(graphql_url: &str) -> Result<(WsSink, WsStream)> {
    println!(">>> Connecting to WebSocket...");
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

    while let Some(msg) = read.next().await {
        if let Ok(Message::Text(text)) = msg {
            if let Ok(response) = serde_json::from_str::<serde_json::Value>(&text) {
                if response.get("type").and_then(|v| v.as_str()) == Some("connection_ack") {
                    println!("✓ Connected to WebSocket");
                    return Ok((write, read));
                }
            }
        }
    }

    Err(anyhow::anyhow!("Failed to receive connection_ack"))
}
