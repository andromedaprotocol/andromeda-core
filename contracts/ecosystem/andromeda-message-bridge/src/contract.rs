use ado_base::state::ADOContract;
use common::{
    ado_base::InstantiateMsg as BaseInstantiateMsg,
    error::{from_semver, ContractError},
};
use cosmwasm_std::{ensure, entry_point, CosmosMsg};
use cosmwasm_std::{
    from_binary, to_binary, Binary, Deps, DepsMut, Env, IbcMsg, MessageInfo, Response, StdResult,
    WasmMsg,
};
use cw2::{get_contract_version, set_contract_version};

use andromeda_ibc::message_bridge::{
    CallbackMsg, ExecuteMsg, IbcOutgoingMsg, InstantiateMsg, MessageBridgePacketData, MigrateMsg,
    QueryMsg,
};
use semver::Version;

use crate::state::AUTHORIZED_USER;

const CONTRACT_NAME: &str = "crates.io:message-bridge";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    AUTHORIZED_USER.save(deps.storage, &info.sender)?;

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
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::ReceiveMessage {
            target,
            outgoing_msg,
            user_msg,
        } => execute_receive_message(deps, info, target, outgoing_msg, user_msg),
        ExecuteMsg::Callback(msg) => execute_callback(deps, env, info, msg),
        _ => Err(ContractError::ExecuteError {}),
    }
}

fn execute_callback(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: CallbackMsg,
) -> Result<Response, ContractError> {
    ensure!(
        info.sender == env.contract.address,
        ContractError::Unauthorized {}
    );
    match msg {
        CallbackMsg::HandlePacketReceive { receiver, msg } => {
            execute_handle_packet_receive(deps.as_ref(), env, info, receiver, msg)
        }
    }
}

fn execute_receive_message(
    deps: DepsMut,
    info: MessageInfo,
    target: String,
    outgoing_msg: Binary,
    user_msg: Binary,
) -> Result<Response, ContractError> {
    let authorized_user = AUTHORIZED_USER.load(deps.storage)?;
    ensure!(
        info.sender == authorized_user,
        ContractError::Unauthorized {}
    );
    do_receive_message(deps, info, target, outgoing_msg, user_msg)
}

fn do_receive_message(
    _deps: DepsMut,
    info: MessageInfo,
    target: String,
    outgoing_msg: Binary,
    user_msg: Binary,
) -> Result<Response, ContractError> {
    let outgoing_msg: IbcOutgoingMsg = from_binary(&outgoing_msg)?;

    let target_message = to_binary(&user_msg)?;
    let final_target_message: Binary = from_binary(&target_message)?;

    let packet_data = MessageBridgePacketData {
        target,
        message: final_target_message,
        sender: info.sender.to_string(),
    };

    let ibc_message = IbcMsg::SendPacket {
        channel_id: outgoing_msg.clone().channel_id,
        data: to_binary(&packet_data)?,
        timeout: outgoing_msg.timeout,
    };

    Ok(Response::default()
        .add_attribute("method", "execute_receive_message")
        .add_attribute("channel_id", outgoing_msg.channel_id)
        .add_message(ibc_message))
}

fn execute_handle_packet_receive(
    deps: Deps,
    env: Env,
    info: MessageInfo,
    receiver: String,
    msg: Binary,
) -> Result<Response, ContractError> {
    ensure!(
        info.sender == env.contract.address,
        ContractError::Unauthorized {}
    );

    let receiver = deps.api.addr_validate(&receiver)?;

    Ok(Response::default()
        .add_attribute("method", "handle_packet_receive")
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: receiver.to_string(),
            msg,
            funds: vec![],
        })))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // New version
    let version: Version = CONTRACT_VERSION.parse().map_err(from_semver)?;

    // Old version
    let stored = get_contract_version(deps.storage)?;
    let storage_version: Version = stored.version.parse().map_err(from_semver)?;

    let contract = ADOContract::default();

    ensure!(
        stored.contract == CONTRACT_NAME,
        ContractError::CannotMigrate {
            previous_contract: stored.contract,
        }
    );

    // New version has to be newer/greater than the old version
    ensure!(
        storage_version < version,
        ContractError::CannotMigrate {
            previous_contract: stored.version,
        }
    );

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Update the ADOContract's version
    contract.execute_update_version(deps)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {}
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_info, MockQuerier},
        ContractResult, CosmosMsg, Empty, IbcTimeout, QuerierResult, SubMsg, Timestamp, WasmQuery,
    };
    use cw721::NftInfoResponse;

    use super::*;
}
