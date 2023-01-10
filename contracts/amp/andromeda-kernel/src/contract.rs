use ado_base::state::ADOContract;
use amp::{
    kernel::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    messages::AMPPkt,
};
use common::{
    ado_base::{AndromedaQuery, InstantiateMsg as BaseInstantiateMsg},
    error::ContractError,
};
use cosmwasm_std::{
    ensure, entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError,
};
use cw2::{get_contract_version, set_contract_version};
use semver::Version;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-kernel";
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
            ado_type: "adodb".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            operators: None,
            modules: None,
            primitive_contract: None,
        },
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.result.is_err() {
        return Err(ContractError::Std(StdError::generic_err(
            msg.result.unwrap_err(),
        )));
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
        ExecuteMsg::Receive(packet) => handle_amp_packet(execute_env, packet),
        _ => Ok(Response::default()),
    }
}

pub fn handle_amp_packet(
    execute_env: ExecuteEnv,
    packet: AMPPkt,
) -> Result<Response, ContractError> {
    //TODO: Sender authorisation
    let mut res = Response::default();
    // Batched message implementation
    // let message_recipients = packet.get_unique_recipients();
    // for recipient in message_recipients {
    //     // Contract address is resolved here to reduce gas costs for repeated recipients
    //     let contract_addr = recipient.clone(); //TODO: ADD NAMESPACING RESOLVER
    //     let messages = packet.get_messages_for_recipient(recipient);
    //     for message in messages {
    //         let sub_msg = message.generate_message(
    //             contract_addr.clone(),
    //             packet.get_origin(),
    //             packet.get_previous_sender(),
    //             1,
    //         )?;

    //         res = res.add_submessage(sub_msg);
    //     }
    // }

    for message in packet.messages.to_vec() {
        let contract_addr =
            message.get_recipient_address(execute_env.deps.api, &execute_env.deps.querier, None)?;
        let msg = message.generate_message(
            contract_addr,
            packet.get_origin(),
            packet.get_previous_sender(),
            1,
        )?;

        res = res.add_submessage(msg)
    }

    Ok(res)
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
    StdError::generic_err(format!("Semver: {}", err))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrQuery(msg) => handle_andromeda_query(deps, env, msg),
    }
}

fn handle_andromeda_query(
    deps: Deps,
    env: Env,
    msg: AndromedaQuery,
) -> Result<Binary, ContractError> {
    match msg {
        _ => ADOContract::default().query(deps, env, msg, query),
    }
}
