#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use amp::messages::ReplyGas;
use andromeda_finance::splitter::{AddressPercent, ExecuteMsg, InstantiateMsg};
use cosmwasm_std::{Empty, ReplyOn};
use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_andromeda_splitter() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_splitter_instantiate_msg(
    recipients: Vec<AddressPercent>,
    kernel_address: impl Into<String>,
    lock_time: Option<u64>,
) -> InstantiateMsg {
    InstantiateMsg {
        recipients,
        lock_time,
        modules: None,
        kernel_address: Some(kernel_address.into()),
    }
}

pub fn mock_splitter_send_msg() -> ExecuteMsg {
    ExecuteMsg::Send {
        reply_gas: ReplyGas {
            reply_on: None,
            gas_limit: None,
        },
    }
}

pub fn mock_splitter_send_kernel_msg(
    reply_on: Option<ReplyOn>,
    gas_limit: Option<u64>,
) -> ExecuteMsg {
    ExecuteMsg::Send {
        reply_gas: ReplyGas {
            reply_on,
            gas_limit,
        },
    }
}
