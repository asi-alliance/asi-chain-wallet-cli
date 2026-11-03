#[rustfmt::skip]
#[allow(dead_code)]
#[path = "../../rust-client/tests/integration/common/keys.rs"]
pub mod keys;
#[rustfmt::skip]
#[allow(dead_code)]
#[path = "../../rust-client/tests/integration/common/node.rs"]
pub mod node;
#[rustfmt::skip]
#[allow(dead_code)]
#[path = "../../rust-client/tests/integration/common/shard.rs"]
pub mod shard;

pub fn rt() -> &'static tokio::runtime::Runtime {
    static RT: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();
    RT.get_or_init(|| tokio::runtime::Runtime::new().unwrap())
}
