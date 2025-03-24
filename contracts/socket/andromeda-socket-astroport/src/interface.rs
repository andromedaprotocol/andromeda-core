use std::str::FromStr;

use andromeda_std::{
    ado_base::MigrateMsg, amp::Recipient, common::denom::Asset, contract_interface,
    deploy::ADOMetadata,
};
use cosmrs::{cosmwasm::MsgExecuteContract, AccountId};
use cosmwasm_std::{to_json_binary, Decimal, Uint128};
use cw_orch::core::serde_json;
use cw_orch_daemon::{Daemon, DaemonBase, TxSender, Wallet};

use andromeda_socket::astroport::{
    Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg, SwapOperation,
};

pub const CONTRACT_ID: &str = "socket_astroport";

contract_interface!(
    SocketAstroportContract,
    CONTRACT_ID,
    "andromeda_socket_astroport.wasm"
);

impl SocketAstroportContract<DaemonBase<Wallet>> {
    #[allow(clippy::too_many_arguments)]
    pub fn execute_swap_from_cw20(
        self,
        daemon: &Daemon,
        from_asset_addr: &str,
        from_amount: Uint128,
        to_asset: Asset,
        recipient: Option<Recipient>,
        max_spread: Option<Decimal>,
        minimum_receive: Option<Uint128>,
        operations: Option<Vec<SwapOperation>>,
    ) {
        let hook_msg = Cw20HookMsg::SwapAndForward {
            to_asset,
            recipient,
            max_spread,
            minimum_receive,
            operations,
        };
        let cw_20_transfer_msg = cw20::Cw20ExecuteMsg::Send {
            contract: self.addr_str().unwrap(),
            amount: from_amount,
            msg: to_json_binary(&hook_msg).unwrap(),
        };
        let exec_msg: MsgExecuteContract = MsgExecuteContract {
            sender: daemon.sender().account_id(),
            contract: AccountId::from_str(from_asset_addr).unwrap(),
            msg: serde_json::to_vec(&cw_20_transfer_msg).unwrap(),
            funds: vec![],
        };

        daemon
            .rt_handle
            .block_on(async { daemon.sender().commit_tx(vec![exec_msg], None).await })
            .unwrap();
    }
}
