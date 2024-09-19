use cw_orch::prelude::ChainInfo;
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;

#[derive(Serialize, Deserialize, Debug)]
pub struct AirdropRequest {
    /// Address of the address asking for funds
    pub address: String,
    /// Denom asked for
    pub denom: String,
}
async fn airdrop(addr: String, chain: &ChainInfo) {
    let client = reqwest::Client::new();
    let url = chain.fcd_url.unwrap_or("http://localhost:8001");
    let url = format!("{}/credit", url);
    client
        .post(url)
        .json(&AirdropRequest {
            address: addr.to_string(),
            denom: chain.gas_denom.to_string(),
        })
        .send()
        .await
        .unwrap();
}

pub fn fund(addr: String, chain_info: &ChainInfo) {
    let rt_handle = Runtime::new().unwrap();
    rt_handle.block_on(airdrop(addr, chain_info));
}
