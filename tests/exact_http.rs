use node_cli::{Endpoint, NodeHttp, RhoValue};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

async fn explore(expr: Value) -> node_cli::Result<node_cli::f1r3fly_api::ExploratoryValues> {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        let mut buffer = [0; 4096];
        loop {
            let read = stream.read(&mut buffer).await.unwrap();
            assert_ne!(read, 0);
            request.extend_from_slice(&buffer[..read]);
            if let Some(end) = request.windows(4).position(|part| part == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&request[..end]).to_ascii_lowercase();
                let length: usize = headers
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length:"))
                    .unwrap()
                    .trim()
                    .parse()
                    .unwrap();
                if request.len() >= end + 4 + length {
                    assert!(headers.starts_with("post /api/explore-deploy http/1.1"));
                    break;
                }
            }
        }
        let body = json!({
            "expr": [expr],
            "block": {"blockHash": "fixture-block", "blockNumber": 42},
            "cost": 1
        })
        .to_string();
        stream.write_all(format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        ).as_bytes()).await.unwrap();
    });
    let result = tokio::time::timeout(Duration::from_secs(5), async {
        NodeHttp::new(Endpoint::parse(&format!("http://{address}"))?)?
            .explore_deploy("new return in { return!(42) }")
            .await
    })
    .await
    .expect("HTTP fixture timed out");
    server.await.unwrap();
    result
}

#[tokio::test]
async fn pinned_client_reads_legacy_and_exact_http_values() {
    for data in [json!(42), json!("42")] {
        let values = explore(json!({"ExprInt": {"data": data}})).await.unwrap();
        assert_eq!(values.value().unwrap(), &RhoValue::Int(42));
        assert_eq!(values.block.number, 42);
    }
    let values = explore(json!({"ExprTuple": {"data": [
        {"ExprInt": {"data": "9007199254740993"}},
        {"ExprUint64": {"data": "18446744073709551615"}},
        {"ExprBigInt": {"data": "10000000000000000000"}}
    ]}}))
    .await
    .unwrap();
    let elements = values.value().unwrap().elements().unwrap();
    assert_eq!(elements[0], RhoValue::Int(9007199254740993));
    assert_eq!(elements[1], RhoValue::Uint64(u64::MAX));
    assert_eq!(
        elements[2].as_amount().unwrap().to_string(),
        "10000000000000000000"
    );
}

#[tokio::test]
async fn pinned_client_rejects_invalid_narrow_http_values() {
    for data in ["01", "-0", "1.5", "1e3", "9223372036854775808"] {
        assert!(explore(json!({"ExprInt": {"data": data}})).await.is_err());
    }
}
