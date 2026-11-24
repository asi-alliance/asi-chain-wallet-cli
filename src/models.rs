use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct WalletIdentity {
    pub public_key: String,
    pub address: String,
}

#[derive(Serialize, Deserialize)]
pub struct WalletInfo {
    pub private_key: String,
    #[serde(flatten)]
    pub identity: WalletIdentity,
}
