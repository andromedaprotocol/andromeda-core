use andromeda_std::ado_base::InstantiateMsg as BaseInstantiateMsg;
use andromeda_std::ado_contract::ADOContract;
use andromeda_std::amp::addresses::AndrAddr;
use andromeda_std::amp::messages::{AMPMsg, AMPPkt};
use andromeda_std::amp::ADO_DB_KEY;
use andromeda_std::common::encode_binary;
use andromeda_std::error::ContractError;
use andromeda_std::ibc::message_bridge::ExecuteMsg as IBCBridgeExecMsg;
use andromeda_std::os::aos_querier::AOSQuerier;
use andromeda_std::os::kernel::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use cosmwasm_std::{
    attr, ensure, entry_point, wasm_execute, Addr, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, Reply, Response, StdError, SubMsg, WasmMsg,
};
use cw2::{get_contract_version, set_contract_version};
use semver::Version;

use crate::state::{IBC_BRIDGE, KERNEL_ADDRESSES};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-kernel";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    ADOContract::default().instantiate(
        deps.storage,
        env.clone(),
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "kernel".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            operators: None,
            kernel_address: env.contract.address.to_string(),
            owner: msg.owner,
        },
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.result.is_err() {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "{}:{}",
            msg.id,
            msg.result.unwrap_err()
        ))));
    }

    Ok(Response::default())
}

pub struct ExecuteEnv<'a> {
    deps: DepsMut<'a>,
    pub env: Env,
    pub info: MessageInfo,
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let execute_env = ExecuteEnv { deps, env, info };

    match msg {
        ExecuteMsg::AMPReceive(packet) => handle_amp_packet(execute_env, packet),
        ExecuteMsg::Send { message } => handle_send(execute_env, message),
        ExecuteMsg::UpsertKeyAddress { key, value } => upsert_key_address(execute_env, key, value),
    }
}

pub fn handle_send(execute_env: ExecuteEnv, message: AMPMsg) -> Result<Response, ContractError> {
    let ExecuteEnv { deps, info, .. } = execute_env;
    let origin = info.clone().sender;
    let recipient = message.recipient.get_raw_address(&deps.as_ref())?;
    //TODO: ADD IBC HANDLING

    Ok(Response::default()
        .add_submessage(SubMsg::new(WasmMsg::Execute {
            contract_addr: recipient.to_string(),
            msg: message.message.clone(),
            funds: info.funds,
        }))
        .add_attribute("action", "handle_amp_message")
        .add_attribute("recipient", message.recipient)
        .add_attribute("message", message.message.to_string())
        .add_attribute("origin", origin.to_string()))
}

pub fn handle_amp_packet(
    execute_env: ExecuteEnv,
    packet: AMPPkt,
) -> Result<Response, ContractError> {
    ensure!(
        query_verify_address(
            execute_env.deps.as_ref(),
            execute_env.info.sender.to_string(),
        )? || packet.ctx.get_origin() == execute_env.info.sender,
        ContractError::Unauthorized {}
    );
    ensure!(
        packet.ctx.id == 0,
        ContractError::InvalidPacket {
            error: Some("Packet ID cannot be provided from outside the Kernel".into())
        }
    );

    let mut res = Response::default();
    ensure!(
        !packet.messages.is_empty(),
        ContractError::InvalidPacket {
            error: Some("No messages supplied".to_string())
        }
    );
    for (idx, message) in packet.messages.iter().enumerate() {
        if let Some(protocol) = message.recipient.get_protocol() {
            match protocol {
                "ibc" => {
                    let bridge_addr =
                        KERNEL_ADDRESSES.may_load(execute_env.deps.storage, IBC_BRIDGE)?;
                    if let Some(bridge_addr) = bridge_addr {
                        if let Some(chain) = message.recipient.get_chain() {
                            let msg = IBCBridgeExecMsg::SendMessage {
                                chain: chain.to_string(),
                                recipient: AndrAddr::from_string(message.recipient.get_raw_path()),
                                message: message.message.clone(),
                            };
                            let cosmos_msg =
                                wasm_execute(bridge_addr.clone(), &msg, message.funds.clone())?;
                            res = res
                                .add_submessage(SubMsg::reply_always(cosmos_msg, 1))
                                .add_attribute("action", "handle_amp_packet")
                                .add_attribute(
                                    format!("recipient:{}", idx),
                                    message.recipient.clone(),
                                )
                                .add_attribute("message", message.message.to_string());
                        } else {
                            return Err(ContractError::InvalidPacket {
                                error: Some("Chain not provided".to_string()),
                            });
                        }
                    } else {
                        return Err(ContractError::InvalidPacket {
                            error: Some("IBC not enabled in kernel".to_string()),
                        });
                    }
                }
                &_ => panic!("Invalid protocol"),
            }
        } else {
            let recipient_addr = message
                .recipient
                .get_raw_address(&execute_env.deps.as_ref())?;
            let msg = message.message.clone();
            if Binary::default() == msg {
                ensure!(
                    !message.funds.is_empty(),
                    ContractError::InvalidPacket {
                        error: Some("No message or funds supplied".to_string())
                    }
                );

                // The message is a bank message
                let sub_msg = BankMsg::Send {
                    to_address: recipient_addr.to_string(),
                    amount: message.funds.clone(),
                };

                res = res
                    .add_submessage(SubMsg::reply_on_error(CosmosMsg::Bank(sub_msg), 1))
                    .add_attributes(vec![
                        attr(format!("recipient:{}", idx), recipient_addr),
                        attr("bank_send_amount", message.funds[0].to_string()),
                    ]);
            } else {
                let origin = packet.ctx.get_origin();
                let previous_sender = execute_env.info.sender.to_string();

                let amp_msg = AMPMsg::new(
                    recipient_addr.clone(),
                    msg,
                    Some(vec![message.funds[0].clone()]),
                );

                let new_packet = AMPPkt::new(origin, previous_sender, vec![amp_msg]);

                let sub_msg = new_packet.to_sub_msg(
                    recipient_addr.clone(),
                    Some(vec![message.funds[0].clone()]),
                    idx as u64,
                )?;
                // TODO: ADD ID
                res = res
                    .add_submessage(sub_msg)
                    .add_attributes(vec![attr(format!("recipient:{}", idx), recipient_addr)]);
            }
        }
    }
    Ok(res.add_attribute("action", "handle_amp_packet"))
}

fn upsert_key_address(
    execute_env: ExecuteEnv,
    key: String,
    value: String,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    ensure!(
        contract.is_contract_owner(execute_env.deps.storage, execute_env.info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    // Updates to new value
    if KERNEL_ADDRESSES.has(execute_env.deps.storage, &key) {
        KERNEL_ADDRESSES.remove(execute_env.deps.storage, &key)
    }

    KERNEL_ADDRESSES.save(
        execute_env.deps.storage,
        &key,
        &execute_env.deps.api.addr_validate(&value)?,
    )?;

    Ok(Response::default().add_attributes(vec![
        attr("action", "upsert_key_address"),
        attr("key", key),
        attr("value", value),
    ]))
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

fn from_semver(err: semver::Error) -> StdError {
    StdError::generic_err(format!("Semver: {err}"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::KeyAddress { key } => encode_binary(&query_key_address(deps, key)?),
        QueryMsg::VerifyAddress { address } => encode_binary(&query_verify_address(deps, address)?),
    }
}

fn query_key_address(deps: Deps, key: String) -> Result<Addr, ContractError> {
    Ok(KERNEL_ADDRESSES.load(deps.storage, &key)?)
}

fn query_verify_address(deps: Deps, address: String) -> Result<bool, ContractError> {
    let db_address = KERNEL_ADDRESSES.load(deps.storage, ADO_DB_KEY)?;
    let contract_info = deps.querier.query_wasm_contract_info(address)?;

    let ado_type = AOSQuerier::ado_type_getter(&deps.querier, &db_address, contract_info.code_id)?;
    Ok(ado_type.is_some())
}
