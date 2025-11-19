use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct WalletInfo {
    pub private_key: String,
    pub public_key: String,
    pub address: String,
}
