use std::{fs, process::Command};

#[test]
fn create_saves_exclusively_and_preserves_the_original_wallet() {
    let dir = tempfile::tempdir().unwrap();
    let create = || {
        Command::new(env!("CARGO_BIN_EXE_asi-chain-wallet-cli"))
            .args(["create", "--save"])
            .current_dir(dir.path())
            .env_clear()
            .env("OUTPUT_DIR", dir.path())
            .env("VALIDATOR_GRPC_URL", "http://localhost:40401")
            .env("VALIDATOR_HTTP_URL", "http://localhost:40403")
            .env("OBSERVER_HTTP_URL", "http://localhost:40453")
            .output()
            .unwrap()
    };
    assert!(create().status.success());
    let path = dir.path().join("wallet_info.json");
    let original = fs::read(&path).unwrap();
    let wallet: serde_json::Value = serde_json::from_slice(&original).unwrap();
    assert!(
        wallet["private_key"]
            .as_str()
            .is_some_and(|key| key.len() == 64)
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    let second = create();
    assert!(!second.status.success());
    assert!(second.stdout.is_empty());
    assert!(fs::read(&path).unwrap() == original);
}
