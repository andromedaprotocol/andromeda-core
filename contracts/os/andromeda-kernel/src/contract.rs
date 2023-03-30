use ado_base::state::ADOContract;
use andromeda_os::messages::{AMPMsg, AMPPkt};
use andromeda_os::{
    adodb::QueryMsg as ADODBQueryMsg,
    kernel::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
};
use common::{
    ado_base::{AndromedaQuery, InstantiateMsg as BaseInstantiateMsg},
    encode_binary,
    error::ContractError,
};
use cosmwasm_std::{
    attr, ensure, entry_point, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply,
    ReplyOn, Response, StdError, SubMsg, WasmMsg,
};
use cw2::{get_contract_version, set_contract_version};
use semver::Version;

use crate::state::{parse_path, parse_path_direct, ADO_DB_KEY, KERNEL_ADDRESSES, VFS_KEY};

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
            ado_type: "kernel".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            operators: None,
            modules: None,
            kernel_address: None,
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
        ExecuteMsg::AMPReceive(packet) => handle_amp_packet(execute_env, packet),
        ExecuteMsg::AMPDirect {
            recipient,
            message,
            reply_on,
            exit_at_error,
            gas_limit,
        } => handle_amp_direct(
            execute_env.deps,
            execute_env.env,
            execute_env.info,
            recipient,
            message,
            reply_on,
            exit_at_error,
            gas_limit,
        ),
        ExecuteMsg::UpsertKeyAddress { key, value } => upsert_key_address(execute_env, key, value),
    }
}

pub fn handle_amp_direct(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: String,
    message: Binary,
    reply_on: Option<ReplyOn>,
    exit_at_error: Option<bool>,
    gas_limit: Option<u64>,
) -> Result<Response, ContractError> {
    let origin = info.clone().sender;
    let previous_sender = env.clone().contract.address;

    let parsed_path = parse_path_direct(
        env,
        info.clone(),
        recipient.clone(),
        message.clone(),
        info.funds.clone(),
        deps.storage,
        reply_on.clone(),
        exit_at_error.clone(),
        gas_limit.clone(),
    )?;
    // If parsed path yields a SubMsg, it means that the recipient is on another chain
    if let Some(msg) = parsed_path {
        Ok(Response::default()
            .add_submessage(msg)
            .add_attribute("action", "handle_amp_direct")
            .add_attribute("recipient", recipient)
            .add_attribute("message", message.to_string()))
    } else {
        let amp_pkt = AMPPkt::new(
            origin,
            previous_sender,
            vec![AMPMsg::new(
                recipient.clone(),
                message.clone(),
                Some(info.clone().funds),
                reply_on,
                exit_at_error,
                gas_limit,
            )],
        );
        Ok(Response::default()
            .add_submessage(SubMsg::new(WasmMsg::Execute {
                contract_addr: recipient.clone(),
                msg: to_binary(&ExecuteMsg::AMPReceive(amp_pkt))?,
                funds: info.funds,
            }))
            .add_attribute("action", "handle_amp_direct")
            .add_attribute("recipient", recipient)
            .add_attribute("message", message.to_string()))
    }
}

pub fn handle_amp_packet(
    execute_env: ExecuteEnv,
    packet: AMPPkt,
) -> Result<Response, ContractError> {
    ensure!(
        query_verify_address(
            execute_env.deps.as_ref(),
            execute_env.info.sender.to_string(),
        )?,
        ContractError::Unauthorized {}
    );

    let mut res = Response::default();

    let vfs_address = KERNEL_ADDRESSES.may_load(execute_env.deps.storage, VFS_KEY)?;
    for amp_message in packet.clone().messages {
        let parsed_path = parse_path(
            amp_message.recipient.clone(),
            packet.clone(),
            amp_message.clone(),
            execute_env.deps.storage,
        )?;
        if let Some(sub_msg) = parsed_path {
            res = res.add_submessage(sub_msg);
            continue;
        };
        let contract_addr = amp_message.get_recipient_address(
            execute_env.deps.api,
            &execute_env.deps.querier,
            vfs_address.clone(),
        )?;
        let msg = amp_message.generate_sub_message(
            contract_addr,
            packet.get_origin(),
            packet.get_previous_sender(),
            1,
        )?;

        res = res.add_submessage(msg)
    }

    // TODO: GENERATE ATTRIBUTES FROM AMP PACKET
    Ok(res)
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
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrQuery(msg) => handle_andromeda_query(deps, env, msg),
        QueryMsg::KeyAddress { key } => encode_binary(&query_key_address(deps, key)?),
        QueryMsg::VerifyAddress { address } => encode_binary(&query_verify_address(deps, address)?),
    }
}

fn handle_andromeda_query(
    deps: Deps,
    env: Env,
    msg: AndromedaQuery,
) -> Result<Binary, ContractError> {
    ADOContract::default().query(deps, env, msg, query)
}

fn query_key_address(deps: Deps, key: String) -> Result<Addr, ContractError> {
    Ok(KERNEL_ADDRESSES.load(deps.storage, &key)?)
}

fn query_verify_address(deps: Deps, address: String) -> Result<bool, ContractError> {
    let db_address = KERNEL_ADDRESSES.load(deps.storage, ADO_DB_KEY)?;
    let contract_info = deps.querier.query_wasm_contract_info(address)?;
    let query = ADODBQueryMsg::ADOType {
        code_id: contract_info.code_id,
    };

    match deps
        .querier
        .query_wasm_smart::<Option<String>>(db_address, &query)?
    {
        Some(_a) => Ok(true),
        None => Ok(false),
    }
}
