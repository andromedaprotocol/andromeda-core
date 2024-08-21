use cosmwasm_std::coins;
use cw_orch::prelude::ChainInfo;
use cw_orch_daemon::{DaemonBase, Wallet};
const FAUCET_MNEMONIC: &str = "increase bread alpha rigid glide amused approve oblige print asset idea enact lawn proof unfold jeans rabbit audit return chuckle valve rather cactus great";

pub fn fund(daemon: &DaemonBase<Wallet>, chain_info: &ChainInfo,  amount: u128) {
    let target_addr = daemon.sender().pub_addr_str();

    let faucet_daemon = daemon.rebuild().mnemonic(FAUCET_MNEMONIC).build().unwrap();
    let rt = faucet_daemon.rt_handle.clone();
    let wallet = faucet_daemon.sender();

    rt.block_on(wallet.bank_send(&target_addr, coins(amount, chain_info.gas_denom.to_string()))).unwrap();
}