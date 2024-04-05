#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_fungible_tokens::cw20::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::modules::Module, amp::AndrAddr};
use cosmwasm_std::{Binary, Empty, Uint128};
use cw20::MinterResponse;
use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_andromeda_cw20() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_minter(minter: String, cap: Option<Uint128>) -> MinterResponse {
    MinterResponse { minter, cap }
}

#[allow(clippy::too_many_arguments)]
pub fn mock_cw20_instantiate_msg(
    owner: Option<String>,
    name: String,
    symbol: String,
    decimals: u8,
    initial_balances: Vec<cw20::Cw20Coin>,
    mint: Option<MinterResponse>,
    modules: Option<Vec<Module>>,
    kernel_address: String,
) -> InstantiateMsg {
    InstantiateMsg {
        name,
        symbol,
        decimals,
        initial_balances,
        mint,
        marketing: None,
        modules,
        kernel_address,
        owner,
    }
}

pub fn mock_get_cw20_balance(address: impl Into<String>) -> QueryMsg {
    QueryMsg::Balance {
        address: address.into(),
    }
}
pub fn mock_get_version() -> QueryMsg {
    QueryMsg::Version {}
}

pub fn mock_cw20_send(contract: AndrAddr, amount: Uint128, msg: Binary) -> ExecuteMsg {
    ExecuteMsg::Send {
        contract,
        amount,
        msg,
    }
}

pub fn mock_cw20_transfer(recipient: AndrAddr, amount: Uint128) -> ExecuteMsg {
    ExecuteMsg::Transfer { recipient, amount }
}
