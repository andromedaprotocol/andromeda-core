use cw_orch::{
    environment::{ChainKind, NetworkInfo},
    prelude::ChainInfo,
};

pub const ANDROMEDA_NETWORK: NetworkInfo = NetworkInfo {
    chain_name: "andromeda",
    pub_address_prefix: "andr",
    coin_type: 118u32,
};

pub const ANDROMEDA_TESTNET: ChainInfo = ChainInfo {
    chain_id: "galileo-4",
    gas_denom: "uandr",
    fcd_url: None,
    gas_price: 0.025,
    grpc_urls: &["http://137.184.182.11:9090/"],
    lcd_url: Some("http://137.184.182.11:1317/"),
    network_info: ANDROMEDA_NETWORK,
    kind: ChainKind::Testnet,
};

pub const ALL_CHAINS: &[ChainInfo] = &[ANDROMEDA_TESTNET];

pub fn get_chain(chain: String) -> ChainInfo {
    ALL_CHAINS
        .iter()
        .find(|c| c.chain_id == chain || c.network_info.chain_name == chain)
        .unwrap()
        .clone()
}
