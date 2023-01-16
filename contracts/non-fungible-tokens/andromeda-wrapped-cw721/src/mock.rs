#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query, reply};
use andromeda_non_fungible_tokens::wrapped_cw721::{
    Cw721HookMsg, InstantiateMsg, InstantiateType, QueryMsg,
};
use cosmwasm_std::Empty;
use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_andromeda_wrapped_cw721() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query).with_reply(reply);
    Box::new(contract)
}

pub fn mock_wrapped_cw721_instantiate_msg(
    primitive_contract: String,
    cw721_instantiate_type: InstantiateType,
    can_unwrap: bool,
) -> InstantiateMsg {
    InstantiateMsg {
        primitive_contract,
        cw721_instantiate_type,
        can_unwrap,
        kernel_address: None,
    }
}

pub fn mock_wrap_nft_msg(wrapped_token_id: Option<String>) -> Cw721HookMsg {
    Cw721HookMsg::Wrap { wrapped_token_id }
}

pub fn mock_unwrap_nft_msg() -> Cw721HookMsg {
    Cw721HookMsg::Unwrap {}
}

pub fn mock_get_wrapped_cw721_sub_address() -> QueryMsg {
    QueryMsg::NFTContractAddress {}
}
