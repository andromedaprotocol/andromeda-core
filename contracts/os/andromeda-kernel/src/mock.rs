#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query, reply};
use andromeda_std::{
    amp::{
        messages::{AMPMsg, AMPMsgConfig},
        AndrAddr,
    },
    os::kernel::{ExecuteMsg, InstantiateMsg, QueryMsg},
};
use cosmwasm_std::{to_json_binary, Coin, Empty};
use cw_multi_test::{Contract, ContractWrapper};
use serde::Serialize;

pub fn mock_andromeda_kernel() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query).with_reply(reply);
    Box::new(contract)
}

pub fn mock_kernel_instantiate_message(owner: Option<String>) -> InstantiateMsg {
    InstantiateMsg {
        owner,
        chain_name: "andromeda-local".to_string(),
    }
}

pub fn mock_upsert_key_address(key: impl Into<String>, value: impl Into<String>) -> ExecuteMsg {
    ExecuteMsg::UpsertKeyAddress {
        key: key.into(),
        value: value.into(),
    }
}

pub fn mock_create(
    ado_type: impl Into<String>,
    msg: impl Serialize,
    owner: Option<AndrAddr>,
    chain: Option<String>,
) -> ExecuteMsg {
    ExecuteMsg::Create {
        ado_type: ado_type.into(),
        msg: to_json_binary(&msg).unwrap(),
        owner,
        chain,
    }
}

pub fn mock_send(
    recipient: impl Into<String>,
    msg: impl Serialize,
    funds: Vec<Coin>,
    config: Option<AMPMsgConfig>,
) -> ExecuteMsg {
    ExecuteMsg::Send {
        message: AMPMsg {
            recipient: AndrAddr::from_string(recipient.into()),
            message: to_json_binary(&msg).unwrap(),
            funds,
            config: config.unwrap_or_default(),
        },
    }
}

pub fn mock_get_key_address(key: impl Into<String>) -> QueryMsg {
    QueryMsg::KeyAddress { key: key.into() }
}
