#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_ecosystem::vault::{ExecuteMsg, InstantiateMsg, QueryMsg, StrategyType};
use andromeda_std::amp::AndrAddr;
use cosmwasm_std::{Binary, Empty};
use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_andromeda_vault() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_vault_instantiate_msg(
    kernel_address: impl Into<String>,
    owner: Option<String>,
) -> InstantiateMsg {
    InstantiateMsg {
        kernel_address: kernel_address.into(),
        owner,
    }
}

/// Used to generate a deposit message for a vault
pub fn mock_vault_deposit_msg(recipient: Option<AndrAddr>, msg: Option<Binary>) -> ExecuteMsg {
    ExecuteMsg::Deposit { recipient, msg }
}

pub fn mock_vault_get_balance(
    address: AndrAddr,
    denom: Option<String>,
    strategy: Option<StrategyType>,
) -> QueryMsg {
    QueryMsg::VaultBalance {
        address,
        strategy,
        denom,
    }
}
