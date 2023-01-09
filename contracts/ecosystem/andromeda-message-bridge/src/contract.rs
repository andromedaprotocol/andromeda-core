use andromeda_ibc::message_bridge::{ExecuteMsg, IbcExecuteMsg, InstantiateMsg, QueryMsg};

use ado_base::ADOContract;
use common::{ado_base::InstantiateMsg as BaseInstantiateMsg, error::ContractError};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, IbcMsg, IbcTimeout, MessageInfo, Response, StdError,
    WasmMsg,
};
use cw2::set_contract_version;

const CONTRACT_NAME: &str = "crates.io:andromeda-message-bridge";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "message-bridge".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            operators: None,
            modules: None,
            primitive_contract: None,
        },
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    _deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::SendMessage {
            channel,
            target,
            message,
        } => Ok(Response::new()
            .add_attribute("method", "execute_send_message")
            .add_attribute("channel", channel.clone())
            // outbound IBC message, where packet is then received on other chain
            .add_message(IbcMsg::SendPacket {
                channel_id: channel,
                data: to_binary(&IbcExecuteMsg::SendMessage { target, message })?,
                timeout: IbcTimeout::with_timestamp(env.block.time.plus_seconds(300)),
            })),
    }
}

/// called on IBC packet receive in other chain
pub fn try_wasm_msg(_deps: DepsMut, target: String, message: Binary) -> Result<WasmMsg, StdError> {
    let wasm_msg = WasmMsg::Execute {
        contract_addr: target,
        msg: message,
        funds: vec![],
    };
    Ok(wasm_msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {}
}
