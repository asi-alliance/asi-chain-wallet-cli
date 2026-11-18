//! Fixture helpers copied from asi-alliance/asi-chain-client at
//! 5d1c9b4d12e33ec8b64a74a28df196ef03478e73 under Apache-2.0.

#[allow(dead_code)]
pub mod keys;
#[allow(dead_code)]
pub mod node;
#[allow(dead_code)]
pub mod shard;

pub fn rt() -> &'static tokio::runtime::Runtime {
    static RT: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();
    RT.get_or_init(|| tokio::runtime::Runtime::new().unwrap())
}
