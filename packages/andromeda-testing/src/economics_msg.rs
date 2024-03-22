use andromeda_std::{common::reply::ReplyId, os::economics::ExecuteMsg as EconomicsExecuteMsg};
use cosmwasm_std::{to_json_binary, Addr, CosmosMsg, SubMsg, WasmMsg};

pub fn generate_economics_message(payee: &str, action: &str) -> SubMsg {
    SubMsg::reply_on_error(
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "economics_contract".to_string(),
            msg: to_json_binary(&EconomicsExecuteMsg::PayFee {
                payee: Addr::unchecked(payee),
                action: action.to_string(),
            })
            .unwrap(),
            funds: vec![],
        }),
        ReplyId::PayFee.repr(),
    )
}
