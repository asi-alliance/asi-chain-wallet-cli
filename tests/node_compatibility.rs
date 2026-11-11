//! Real wallet commands against the existing immutable ASI node image.
#[path = "support/mod.rs"]
mod common;

use common::{keys, node::Node, rt, shard::Shard};
use num_bigint::BigUint;
use std::{
    io::{Read, Write},
    net::TcpListener,
    path::PathBuf,
    process::{Command, Output},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

fn wallet(key: &keys::Key, args: &[&str], extra: &[(&str, &str)]) -> Output {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/wallet-fixtures")
        .join(&key.address);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("wallet_info.json");
    if !path.exists() {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(path).unwrap();
        let json = serde_json::json!({"private_key":key.private,"public_key":key.public,"address":key.address});
        file.write_all(json.to_string().as_bytes()).unwrap();
    }
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_asi-chain-wallet-cli"));
    for (name, _) in std::env::vars_os() {
        let n = name.to_string_lossy();
        if ["VALIDATOR_", "OBSERVER_", "NODE_", "EXPECTED_", "DEPLOY_"]
            .iter()
            .any(|p| n.starts_with(p))
            || ["SHARD_ID", "OUTPUT_DIR", "GRAPHQL_URL", "VERBOSE_LOGS"].contains(&n.as_ref())
        {
            cmd.env_remove(name);
        }
    }
    cmd.current_dir(&dir)
        .args(args)
        .env("VALIDATOR_GRPC_URL", Node::Validator1.grpc_endpoint())
        .env("VALIDATOR_HTTP_URL", Node::Validator1.http_endpoint())
        .env("OBSERVER_HTTP_URL", Node::ReadOnly.http_endpoint())
        .env("OUTPUT_DIR", &dir)
        .env("VERBOSE_LOGS", "false")
        .env("DEPLOY_MAX_WAIT_SECONDS", "120")
        .env("DEPLOY_CHECK_INTERVAL_SECONDS", "2")
        .envs(extra.iter().copied())
        .output()
        .unwrap()
}
fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}
fn balance(key: &keys::Key) -> BigUint {
    let out = wallet(key, &["balance"], &[]);
    assert!(out.status.success(), "{}", text(&out));
    text(&out)
        .lines()
        .find_map(|s| s.strip_prefix("Wallet balance: "))
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .parse()
        .unwrap()
}
fn deploy_id(out: &Output) -> String {
    text(out)
        .lines()
        .find_map(|s| s.strip_prefix("Deploy ID: "))
        .expect("a recoverable deploy ID")
        .into()
}
fn observed(id: &str) {
    let deadline = Instant::now() + Duration::from_secs(90);
    let node = node_cli::NodeHttp::new(
        node_cli::Endpoint::parse(&Node::ReadOnly.http_endpoint()).unwrap(),
    )
    .unwrap();
    loop {
        let status = rt().block_on(node.deploy_finalization_status(id)).unwrap();
        if status.is_some_and(|s| s.is_terminal()) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "observer never reported a verdict for {id}"
        );
        std::thread::sleep(Duration::from_secs(2));
    }
}

/// A responsive status endpoint that deliberately never gives a terminal verdict.
/// Public balance/metadata are copied from this run's real observer.
struct Pending {
    url: String,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Pending {
    fn start(address: &str) -> Self {
        let meta = Node::ReadOnly.status().unwrap().to_string();
        let balance = rt().block_on(async {
            reqwest::get(Node::ReadOnly.http_url(&format!("/api/balance/{address}")))
                .await
                .unwrap()
                .text()
                .await
                .unwrap()
        });
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let thread = std::thread::spawn(move || {
            while !flag.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream
                            .set_read_timeout(Some(Duration::from_secs(2)))
                            .unwrap();
                        let mut buffer = [0; 8192];
                        let size = stream.read(&mut buffer).unwrap_or(0);
                        let request = String::from_utf8_lossy(&buffer[..size]);
                        let body = if request.contains("/api/status ") {
                            &meta
                        } else if request.contains("/api/balance/") {
                            &balance
                        } else {
                            r#"{"state":"Pending","latestBlockHash":null,"rejectionCount":0}"#
                        };
                        let response = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                            body.len(),
                            body
                        );
                        let _ = stream.write_all(response.as_bytes());
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(10))
                    }
                    Err(e) => panic!("mock listener: {e}"),
                }
            }
        });
        Self {
            url,
            stop,
            thread: Some(thread),
        }
    }
}
impl Drop for Pending {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.thread.take().unwrap().join().unwrap();
    }
}

#[test]
fn wallet_balance_transfer_rejection_and_timeout() {
    let existing = Command::new("docker")
        .args([
            "ps",
            "-aq",
            "--filter",
            "label=com.docker.compose.project=rust-client-it",
        ])
        .output()
        .unwrap();
    assert!(
        existing.status.success() && existing.stdout.is_empty(),
        "refusing to touch an existing integration project"
    );
    let _shard = Shard::start().expect("fresh isolated shard");
    assert_eq!(balance(&keys::TRANSFER_SENDER).to_string(), keys::FUNDED);
    let wrong = wallet(
        &keys::TRANSFER_SENDER,
        &["balance"],
        &[("EXPECTED_TOKEN_SYMBOL", "OTHER")],
    );
    assert!(!wrong.status.success());
    assert!(text(&wrong).contains("Wrong native token"));
    let zero = wallet(
        &keys::TRANSFER_SENDER,
        &[
            "send",
            "--to",
            &keys::TRANSFER_RECIPIENT.address,
            "--amount",
            "0",
        ],
        &[],
    );
    assert!(!zero.status.success());
    assert!(!text(&zero).contains("Deploy ID:"));
    let before = balance(&keys::TRANSFER_RECIPIENT);
    let amount = "9007199254740993";
    let sent = wallet(
        &keys::TRANSFER_SENDER,
        &[
            "send",
            "--to",
            &keys::TRANSFER_RECIPIENT.address,
            "--amount",
            amount,
        ],
        &[],
    );
    assert!(sent.status.success(), "{}", text(&sent));
    assert!(text(&sent).contains("Transfer complete."));
    observed(&deploy_id(&sent));
    assert_eq!(
        balance(&keys::TRANSFER_RECIPIENT) - before,
        amount.parse::<BigUint>().unwrap()
    );
    let before = balance(&keys::IDLE_RECIPIENT);
    let all = balance(&keys::OVERDRAFT_TRANSFER).to_string();
    let refused = wallet(
        &keys::OVERDRAFT_TRANSFER,
        &[
            "send",
            "--to",
            &keys::IDLE_RECIPIENT.address,
            "--amount",
            &all,
        ],
        &[],
    );
    assert!(!refused.status.success(), "a rejected transfer must fail");
    assert!(
        text(&refused).contains("Transfer failed"),
        "{}",
        text(&refused)
    );
    assert!(!text(&refused).contains("Transfer complete."));
    observed(&deploy_id(&refused));
    assert_eq!(balance(&keys::IDLE_RECIPIENT), before);
    let pending = Pending::start(&keys::RESUBMIT.address);
    let timeout = wallet(
        &keys::RESUBMIT,
        &["send", "--to", &keys::SPARE.address, "--amount", "1"],
        &[
            ("OBSERVER_HTTP_URL", &pending.url),
            ("VALIDATOR_HTTP_URL", &pending.url),
            ("DEPLOY_MAX_WAIT_SECONDS", "1"),
            ("DEPLOY_CHECK_INTERVAL_SECONDS", "1"),
        ],
    );
    assert!(!timeout.status.success());
    assert!(
        text(&timeout).contains("outcome unknown"),
        "{}",
        text(&timeout)
    );
    assert!(!text(&timeout).contains("Transfer complete."));
    observed(&deploy_id(&timeout));
}
