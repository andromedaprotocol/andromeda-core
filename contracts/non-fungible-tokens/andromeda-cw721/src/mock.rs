#![cfg(not(target_arch = "wasm32"))]

use crate::contract::{execute, instantiate, query};
use andromeda_non_fungible_tokens::cw721::{InstantiateMsg, QueryMsg};
use common::{ado_base::modules::Module, app::AndrAddress};
use cosmwasm_std::Empty;
use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_andromeda_cw721() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_cw721_instantiate_msg(
    name: String,
    symbol: String,
    minter: String,
    modules: Option<Vec<Module>>,
) -> InstantiateMsg {
    InstantiateMsg {
        name,
        symbol,
        minter: AndrAddress { identifier: minter },
        modules,
    }
}

pub fn mock_cw721_owner_of(token_id: String, include_expired: Option<bool>) -> QueryMsg {
    QueryMsg::OwnerOf {
        token_id,
        include_expired,
    }
}
