#![cfg(not(target_arch = "wasm32"))]

use crate::contract::{execute, instantiate, query};
use andromeda_fungible_tokens::cw20_staking::{Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg};
use common::{ado_base::modules::Module, app::AndrAddress};
use cosmwasm_std::{Addr, Empty, Uint128};
use cw20::MinterResponse;
use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_andromeda_cw20_staking() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_cw20_staking_instantiate_msg(staking_token: String) -> InstantiateMsg {
    InstantiateMsg {
        staking_token: AndrAddress::from_string(staking_token),
        additional_rewards: None,
    }
}

pub fn mock_cw20_stake() -> Cw20HookMsg {
    Cw20HookMsg::StakeTokens {}
}

pub fn mock_cw20_get_staker(address: String) -> QueryMsg {
    QueryMsg::Staker { address }
}
