#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_std::{
    amp::AndrAddr,
    os::economics::{ExecuteMsg, InstantiateMsg, QueryMsg},
};
use cosmwasm_std::{Empty, Uint128};
use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_andromeda_economics() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_economics_instantiate_msg(
    kernel_address: impl Into<String>,
    owner: Option<String>,
) -> InstantiateMsg {
    InstantiateMsg {
        kernel_address: kernel_address.into(),
        owner,
    }
}

pub fn mock_deposit(address: Option<AndrAddr>) -> ExecuteMsg {
    ExecuteMsg::Deposit { address }
}

pub fn mock_withdraw(amount: Option<Uint128>, asset: String) -> ExecuteMsg {
    ExecuteMsg::Withdraw { amount, asset }
}

pub fn mock_withdraw_cw20(amount: Option<Uint128>, asset: String) -> ExecuteMsg {
    ExecuteMsg::WithdrawCW20 { amount, asset }
}

pub fn mock_balance(address: AndrAddr, asset: impl Into<String>) -> QueryMsg {
    QueryMsg::Balance {
        address,
        asset: asset.into(),
    }
}
