use futures_util::{SinkExt, StreamExt};
use node_cli::utils::CryptoUtils;
use serde_json::{Value, json};
use std::{process::Output, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    process::Command,
};
use tokio_tungstenite::{
    accept_hdr_async,
    tungstenite::{
        Message,
        handshake::server::{ErrorResponse, Request, Response},
    },
};

struct Wallet {
    dir: tempfile::TempDir,
    public: String,
    address: String,
}
impl Wallet {
    fn public_only(empty_private: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let (_, public) = CryptoUtils::generate_key_pair();
        let public = CryptoUtils::serialize_public_key(&public, false);
        let address = CryptoUtils::generate_vault_address(&public).unwrap();
        // Uppercase is equivalent key material but must be normalized for Hasura.
        let mut record = json!({"public_key": public.to_uppercase(), "address": address});
        if empty_private {
            record["private_key"] = json!("");
        }
        std::fs::write(dir.path().join("wallet_info.json"), record.to_string()).unwrap();
        Self {
            dir,
            public,
            address,
        }
    }
    fn command(&self, args: &[&str]) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_asi-chain-wallet-cli"));
        cmd.args(args)
            .current_dir(self.dir.path())
            .env_clear()
            .kill_on_drop(true)
            .env("OUTPUT_DIR", self.dir.path())
            .env("VALIDATOR_GRPC_URL", "http://127.0.0.1:9")
            .env("VALIDATOR_HTTP_URL", "http://127.0.0.1:9")
            .env("OBSERVER_HTTP_URL", "http://127.0.0.1:9")
            .env("GRAPHQL_TIMEOUT_SECONDS", "1");
        cmd
    }
}
fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}
fn frame(value: Value) -> Message {
    Message::Text(value.to_string())
}
fn data(rows: Value) -> Message {
    frame(json!({"id":"init","type":"data","payload":{"data":{"deployments":rows}}}))
}
fn complete() -> Message {
    frame(json!({"id":"init","type":"complete"}))
}

#[expect(
    clippy::result_large_err,
    reason = "Tungstenite requires this exact handshake callback error type"
)]
fn graphql_protocol(request: &Request, mut response: Response) -> Result<Response, ErrorResponse> {
    assert_eq!(request.headers()["Sec-WebSocket-Protocol"], "graphql-ws");
    response
        .headers_mut()
        .insert("Sec-WebSocket-Protocol", "graphql-ws".parse().unwrap());
    Ok(response)
}

async fn graphql(
    wallet: &Wallet,
    args: &[&str],
    greeting: Option<Value>,
    responses: Vec<Message>,
    close: bool,
    history: bool,
) -> Output {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    let public = wallet.public.clone();
    let address = wallet.address.clone();
    let server = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut ws = accept_hdr_async(socket, graphql_protocol).await.unwrap();
        let init: Value =
            serde_json::from_str(ws.next().await.unwrap().unwrap().to_text().unwrap()).unwrap();
        assert_eq!(init["type"], "connection_init");
        let acknowledged = greeting
            .as_ref()
            .is_some_and(|v| v["type"] == "connection_ack");
        if let Some(greeting) = greeting {
            ws.send(frame(greeting)).await.unwrap();
        }
        if acknowledged {
            let query: Value =
                serde_json::from_str(ws.next().await.unwrap().unwrap().to_text().unwrap()).unwrap();
            assert_eq!(query["payload"]["variables"]["publicKey"], public);
            assert_eq!(query["payload"]["variables"]["address"], address);
            for response in responses {
                ws.send(response).await.unwrap();
            }
            if history {
                let subscription: Value =
                    serde_json::from_str(ws.next().await.unwrap().unwrap().to_text().unwrap())
                        .unwrap();
                assert_eq!(subscription["id"], "1");
                assert_eq!(subscription["payload"]["variables"]["publicKey"], public);
                ws.send(frame(
                    json!({"id":"1","type":"data","payload":{"data":{"deployments":[]}}}),
                ))
                .await
                .unwrap();
                ws.send(frame(json!({"id":"1","type":"complete"})))
                    .await
                    .unwrap();
            }
        }
        if close {
            let _ = ws.close(None).await;
        } else {
            // Keep the connection open: an operation error or deadline must
            // terminate the CLI without waiting for a server-side close.
            while let Some(Ok(message)) = ws.next().await {
                if matches!(message, Message::Close(_)) {
                    break;
                }
            }
        }
    });
    let output = tokio::time::timeout(
        Duration::from_secs(5),
        wallet.command(args).env("GRAPHQL_URL", url).output(),
    )
    .await
    .expect("CLI must terminate on error or deadline")
    .unwrap();
    tokio::time::timeout(Duration::from_secs(2), server)
        .await
        .unwrap()
        .unwrap();
    output
}
fn ack() -> Option<Value> {
    Some(json!({"type":"connection_ack"}))
}

#[tokio::test]
async fn balance_accepts_public_only_and_empty_private_key_wallets() {
    for empty in [false, true] {
        let wallet = Wallet::public_only(empty);
        let address = wallet.address.clone();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            for _ in 0..2 {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = vec![];
                let mut buffer = [0; 4096];
                while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                    let n = socket.read(&mut buffer).await.unwrap();
                    assert_ne!(n, 0);
                    request.extend_from_slice(&buffer[..n]);
                }
                let request = String::from_utf8(request).unwrap();
                let body = if request.starts_with("GET /api/status ") {
                    json!({"version":{},"address":"fixture","networkId":"test","shardId":"root","peers":0,"nodes":0,"minPhloPrice":"1","nativeTokenSymbol":"ASI","nativeTokenDecimals":18})
                } else {
                    assert!(request.starts_with(&format!("GET /api/balance/{address} ")));
                    json!({"address":address,"balance":"1000000000000000001","blockHash":"abcd","blockNumber":319})
                }.to_string();
                socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
            }
        });
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            wallet
                .command(&["balance"])
                .env("OBSERVER_HTTP_URL", url)
                .output(),
        )
        .await
        .unwrap()
        .unwrap();
        assert!(result.status.success(), "{}", text(&result));
        assert!(text(&result).contains("1000000000000000001 dust"));
        server.await.unwrap();
        let refused = wallet
            .command(&["send", "--to", &wallet.address, "--amount", "1"])
            .output()
            .await
            .unwrap();
        assert!(!refused.status.success());
        assert!(refused.stdout.is_empty());
        assert!(!text(&refused).contains("Deploy ID:"));
        assert!(text(&refused).contains("private_key") || text(&refused).contains("private key"));
    }
}

#[tokio::test]
async fn public_only_history_reaches_subscription_and_export_preserves_exact_data() {
    let amount = "115792089237316195423570985008687907853269984665640564039457584007913129639935";
    for empty in [false, true] {
        let wallet = Wallet::public_only(empty);
        let amount_json: Value = if empty {
            serde_json::from_str(amount).unwrap()
        } else {
            json!(amount)
        };
        let rows = json!([{"deploy_id":"test-deploy","block_number":319,"status":"included","created_at":"2026-11-20T10:00:00Z","block":{"finalization_status":"finalized"},"transfers":[{"from_address":wallet.address,"to_address":"recipient","amount_dust":amount_json}]}]);
        let history = graphql(
            &wallet,
            &["history"],
            ack(),
            vec![data(rows.clone()), complete()],
            false,
            true,
        )
        .await;
        assert!(history.status.success(), "{}", text(&history));
        assert!(text(&history).contains("Monitoring for new deployments"));
        assert!(text(&history).contains("Block: #319"));
        assert!(text(&history).contains(&format!("OUT | {amount} dust")));
        let result = graphql(
            &wallet,
            &["export", "--output", "export.json"],
            ack(),
            vec![data(rows.clone()), complete()],
            false,
            false,
        )
        .await;
        assert!(result.status.success(), "{}", text(&result));
        let exported: Value =
            serde_json::from_slice(&std::fs::read(wallet.dir.path().join("export.json")).unwrap())
                .unwrap();
        assert_eq!(exported[0]["block_number"], "319");
        assert_eq!(exported[0]["transfer_amount"], amount);
        assert_eq!(exported[0]["transfer_direction"], "OUT");
        let csv = graphql(
            &wallet,
            &["export", "--format", "csv", "--output", "export.csv"],
            ack(),
            vec![data(rows), complete()],
            false,
            false,
        )
        .await;
        assert!(csv.status.success(), "{}", text(&csv));
        let mut reader = csv::Reader::from_path(wallet.dir.path().join("export.csv")).unwrap();
        let record = reader.records().next().unwrap().unwrap();
        assert_eq!(&record[1], "319");
        assert_eq!(&record[5], amount);
    }
}

#[tokio::test]
async fn export_only_reports_empty_after_successful_data_and_completion() {
    let wallet = Wallet::public_only(false);
    let result = graphql(
        &wallet,
        &["export"],
        ack(),
        vec![data(json!([])), complete()],
        false,
        false,
    )
    .await;
    assert!(result.status.success(), "{}", text(&result));
    assert!(text(&result).contains("No deployments found"));
    assert!(!wallet.dir.path().join("exports").exists());
}

#[tokio::test]
async fn export_reports_server_errors_without_waiting_for_close_or_overwriting_files() {
    for (greeting, responses) in [
        (
            Some(json!({"type":"connection_error","payload":{"message":"permission denied"}})),
            vec![],
        ),
        (
            ack(),
            vec![frame(
                json!({"id":"init","type":"error","payload":[{"message":"permission denied"}]}),
            )],
        ),
        (
            ack(),
            vec![frame(
                json!({"id":"init","type":"data","payload":{"data":{"deployments":[]},"errors":[{"message":"permission denied"}]}}),
            )],
        ),
    ] {
        let wallet = Wallet::public_only(false);
        let path = wallet.dir.path().join("export.json");
        std::fs::write(&path, "previous export").unwrap();
        let result = graphql(
            &wallet,
            &["export", "--output", "export.json"],
            greeting,
            responses,
            false,
            false,
        )
        .await;
        assert!(!result.status.success());
        assert!(
            text(&result).contains("permission denied"),
            "{}",
            text(&result)
        );
        assert!(!text(&result).contains("No deployments found"));
        assert_eq!(std::fs::read(path).unwrap(), b"previous export");
    }
}

#[tokio::test]
async fn incomplete_invalid_and_stalled_exports_fail() {
    for (greeting, responses, close) in [
        (ack(), vec![], true),
        (ack(), vec![complete()], true),
        (ack(), vec![data(json!([]))], true),
        (ack(), vec![data(json!(null)), complete()], false),
        (ack(), vec![data(json!([null])), complete()], false),
        (
            ack(),
            vec![data(json!([])), data(json!([])), complete()],
            false,
        ),
        (ack(), vec![Message::Text("not JSON".into())], false),
        (ack(), vec![Message::Binary(vec![0])], false),
        (ack(), vec![data(json!([]))], false),
        (None, vec![], false),
    ] {
        let wallet = Wallet::public_only(false);
        let result = graphql(
            &wallet,
            &["export", "--output", "export.json"],
            greeting,
            responses,
            close,
            false,
        )
        .await;
        assert!(!result.status.success(), "{}", text(&result));
        assert!(!text(&result).contains("No deployments found"));
        assert!(!wallet.dir.path().join("export.json").exists());
    }
}
