#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_ecosystem::vault::{ExecuteMsg, InstantiateMsg, QueryMsg, StrategyType};
use andromeda_finance::splitter::AMPRecipient as Recipient;
use cosmwasm_std::{Coin, Empty};
use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_andromeda_vault() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_vault_instantiate_msg() -> InstantiateMsg {
    InstantiateMsg {
        kernel_address: Some("contract3".to_string()),
    }
}

/// Used to generate a deposit message for a vault
pub fn mock_vault_deposit_msg(
    recipient: Option<Recipient>,
    amount: Option<Coin>,
    strategy: Option<StrategyType>,
) -> ExecuteMsg {
    ExecuteMsg::Deposit {
        recipient,
        amount,
        strategy,
    }
}

pub fn mock_vault_get_balance(
    address: String,
    denom: Option<String>,
    strategy: Option<StrategyType>,
) -> QueryMsg {
    QueryMsg::Balance {
        address,
        strategy,
        denom,
    }
}
