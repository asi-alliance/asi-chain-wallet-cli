use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};

fn create(directory: &Path) -> Command {
    #[cfg(unix)]
    let mut command = {
        let mut command = Command::new("sh");
        command
            .args(["-c", "umask 022; exec \"$@\"", "wallet-create"])
            .arg(env!("CARGO_BIN_EXE_asi-chain-wallet-cli"));
        command
    };
    #[cfg(not(unix))]
    let mut command = Command::new(env!("CARGO_BIN_EXE_asi-chain-wallet-cli"));
    command
        .args(["create", "--save"])
        .current_dir(directory)
        .env_clear()
        .env("OUTPUT_DIR", directory)
        .env("VALIDATOR_GRPC_URL", "http://localhost:40401")
        .env("VALIDATOR_HTTP_URL", "http://localhost:40403")
        .env("OBSERVER_HTTP_URL", "http://localhost:40453")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

#[test]
fn create_saves_exclusively_and_preserves_the_original_wallet() {
    let dir = tempfile::tempdir().unwrap();
    assert!(create(dir.path()).output().unwrap().status.success());
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
    let second = create(dir.path()).output().unwrap();
    assert!(!second.status.success());
    assert!(second.stdout.is_empty());
    assert!(fs::read(&path).unwrap() == original);
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn concurrent_creators_publish_exactly_one_complete_wallet() {
    let dir = tempfile::tempdir().unwrap();
    let children: Vec<_> = (0..8)
        .map(|_| create(dir.path()).spawn().unwrap())
        .collect();
    let mut successes = 0;
    for child in children {
        let output = child.wait_with_output().unwrap();
        if output.status.success() {
            successes += 1;
        } else {
            assert!(output.stdout.is_empty());
        }
    }
    assert_eq!(successes, 1);
    let wallet: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.path().join("wallet_info.json")).unwrap()).unwrap();
    let key = node_cli::SigningKey::from_hex(wallet["private_key"].as_str().unwrap()).unwrap();
    let public = node_cli::utils::CryptoUtils::serialize_public_key(&key.public_key(), false);
    assert_eq!(wallet["public_key"], public);
    assert_eq!(
        wallet["address"],
        node_cli::utils::CryptoUtils::generate_vault_address(&public).unwrap()
    );
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[cfg(unix)]
#[test]
fn existing_and_dangling_symlinks_are_never_replaced_or_followed() {
    use std::os::unix::fs::symlink;
    for exists in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("original");
        if exists {
            fs::write(&target, "original wallet bytes").unwrap();
        }
        let path = dir.path().join("wallet_info.json");
        symlink(&target, &path).unwrap();
        let output = create(dir.path()).output().unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert_eq!(fs::read_link(&path).unwrap(), target);
        if exists {
            assert_eq!(fs::read(&target).unwrap(), b"original wallet bytes");
        } else {
            assert!(!target.exists());
        }
    }
}

#[test]
fn nested_relative_output_directory_is_persisted() {
    let dir = tempfile::tempdir().unwrap();
    let output = create(dir.path())
        .env("OUTPUT_DIR", "new/parents/wallet")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(
        dir.path()
            .join("new/parents/wallet/wallet_info.json")
            .is_file()
    );
}
