#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_ibc::message_bridge::{ExecuteMsg, InstantiateMsg, QueryMsg};
use cosmwasm_std::{Binary, Empty};

use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_andromeda_message_bridge() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_message_bridge_instantiate_msg(kernel_address: Option<String>) -> InstantiateMsg {
    InstantiateMsg { kernel_address }
}

pub fn mock_message_bridge_supported_chains() -> QueryMsg {
    QueryMsg::SupportedChains {}
}

pub fn mock_message_bridge_channel_id(chain: String) -> QueryMsg {
    QueryMsg::ChannelID { chain }
}

pub fn mock_save_channel(channel: String, chain: String) -> ExecuteMsg {
    ExecuteMsg::SaveChannel { channel, chain }
}

pub fn mock_send_message(recipient: String, chain: String, message: Binary) -> ExecuteMsg {
    ExecuteMsg::SendMessage {
        chain,
        recipient,
        message,
    }
}
