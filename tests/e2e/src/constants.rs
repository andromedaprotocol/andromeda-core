use cw_orch::{
    environment::{ChainKind, NetworkInfo},
    prelude::ChainInfo,
};

pub const USER_MNEMONIC: &str = "across left ignore gold echo argue track joy hire release captain enforce hotel wide flash hotel brisk joke midnight duck spare drop chronic stool";

pub const RECIPIENT_MNEMONIC_1: &str = "anger couple segment silk office amazing fat fortune arrow course love fabric pitch parade stone deliver answer mule text social truth gravity patch safe";

pub const RECIPIENT_MNEMONIC_2: &str = "envelope loyal junk top magic fun face gorilla large clay blur explain narrow intact fortune charge measure modify embrace there spare wood drip dignity";

pub const PURCHASER_MNEMONIC_1: &str = "drift taxi hip erosion trade army illegal party eager deliver season nature section brick twin gallery rate visual wood knee veteran regret steel okay";

pub const TERRA_NETWORK: NetworkInfo = NetworkInfo {
    chain_name: "terra",
    pub_address_prefix: "terra",
    coin_type: 330u32,
};

pub const LOCAL_TERRA: ChainInfo = ChainInfo {
    kind: ChainKind::Local,
    chain_id: "localterraa-1",
    gas_denom: "uluna",
    gas_price: 0.15,
    grpc_urls: &["http://localhost:20331"],
    network_info: TERRA_NETWORK,
    lcd_url: None,
    fcd_url: None,
};

pub const WASM_NETWORK: NetworkInfo = NetworkInfo {
    chain_name: "wasm",
    pub_address_prefix: "wasm",
    coin_type: 118u32,
};

pub const LOCAL_WASM: ChainInfo = ChainInfo {
    kind: ChainKind::Local,
    chain_id: "localwasma-1",
    gas_denom: "ubindo",
    gas_price: 0.15,
    grpc_urls: &["http://localhost:20341"],
    network_info: WASM_NETWORK,
    lcd_url: None,
    fcd_url: None,
};
