use crate::state::{
    add_app_component, generate_assign_app_message, generate_ownership_message,
    load_component_addresses, load_component_addresses_with_name, load_component_descriptors,
    ADO_ADDRESSES, ADO_DESCRIPTORS, APP_NAME,
};
use ado_base::ADOContract;
use andromeda_app::app::{
    AppComponent, ComponentAddress, ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg,
    QueryMsg,
};
use common::{
    ado_base::{AndromedaQuery, InstantiateMsg as BaseInstantiateMsg},
    encode_binary,
    error::ContractError,
    parse_message,
    response::get_reply_address,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, QuerierWrapper, Reply,
    ReplyOn, Response, StdError, Storage, SubMsg, WasmMsg,
};
use cw2::{get_contract_version, set_contract_version};

use semver::Version;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-app-contract";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    APP_NAME.save(deps.storage, &msg.name)?;
    ensure!(msg.app_components.len() <= 50, ContractError::TooManyAppComponents {});

    let sender = info.sender.to_string();
    let resp = ADOContract::default()
        .instantiate(
            deps.storage,
            env,
            deps.api,
            info,
            BaseInstantiateMsg {
                ado_type: "app".to_string(),
                ado_version: CONTRACT_VERSION.to_string(),
                operators: None,
                modules: None,
                primitive_contract: Some(msg.primitive_contract),
            },
        )?
        .add_attribute("owner", &sender)
        .add_attribute("andr_app", msg.name);

    let mut msgs: Vec<SubMsg> = vec![];
    for component in msg.app_components {
        let comp_resp = execute_add_app_component(&deps.querier, deps.storage, &sender, component)?;
        msgs.extend(comp_resp.messages);
    }

    Ok(resp.add_submessages(msgs))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.result.is_err() {
        return Err(ContractError::Std(StdError::generic_err(
            msg.result.unwrap_err(),
        )));
    }

    let id = msg.id.to_string();
    let descriptor = ADO_DESCRIPTORS.load(deps.storage, &id)?;

    let addr_str = get_reply_address(msg)?;
    let addr = &deps.api.addr_validate(&addr_str)?;
    ADO_ADDRESSES.save(deps.storage, &descriptor.name, addr)?;
    let assign_app = generate_assign_app_message(addr, env.contract.address.as_ref())?;
    Ok(Response::default().add_submessage(assign_app))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AndrReceive(msg) => {
            ADOContract::default().execute(deps, env, info, msg, execute)
        }
        ExecuteMsg::AddAppComponent { component } => {
            execute_add_app_component(&deps.querier, deps.storage, info.sender.as_str(), component)
        }
        ExecuteMsg::ClaimOwnership { name } => {
            execute_claim_ownership(deps.storage, info.sender.as_str(), name)
        }
        ExecuteMsg::ProxyMessage { msg, name } => execute_message(deps, info, name, msg),
        ExecuteMsg::UpdateAddress { name, addr } => execute_update_address(deps, info, name, addr),
    }
}

fn execute_add_app_component(
    querier: &QuerierWrapper,
    storage: &mut dyn Storage,
    sender: &str,
    component: AppComponent,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    ensure!(
        contract.is_contract_owner(storage, sender)?,
        ContractError::Unauthorized {}
    );

    let current_addr = ADO_ADDRESSES.may_load(storage, &component.name)?;
    ensure!(current_addr.is_none(), ContractError::NameAlreadyTaken {});

    // This is a default value that will be overridden on `reply`.
    ADO_ADDRESSES.save(storage, &component.name, &Addr::unchecked(""))?;

    let idx = add_app_component(storage, &component)?;
    let inst_msg = contract.generate_instantiate_msg(
        storage,
        querier,
        idx,
        component.instantiate_msg,
        component.ado_type.clone(),
        sender.to_string(),
    )?;

    Ok(Response::new()
        .add_submessage(inst_msg)
        .add_attribute("method", "add_app_component")
        .add_attribute("name", component.name)
        .add_attribute("type", component.ado_type))
}

fn execute_claim_ownership(
    storage: &mut dyn Storage,
    sender: &str,
    name_opt: Option<String>,
) -> Result<Response, ContractError> {
    ensure!(
        ADOContract::default().is_contract_owner(storage, sender)?,
        ContractError::Unauthorized {}
    );

    let mut msgs: Vec<SubMsg> = vec![];
    if let Some(name) = name_opt {
        let address = ADO_ADDRESSES.load(storage, &name)?;
        msgs.push(generate_ownership_message(address, sender)?);
    } else {
        let addresses = load_component_addresses(storage)?;
        for address in addresses {
            msgs.push(generate_ownership_message(address, sender)?);
        }
    }

    Ok(Response::new()
        .add_submessages(msgs)
        .add_attribute("method", "claim_ownership"))
}

fn execute_message(
    deps: DepsMut,
    info: MessageInfo,
    name: String,
    msg: Binary,
) -> Result<Response, ContractError> {
    //Temporary until message sender attached to Andromeda Comms
    ensure!(
        ADOContract::default().is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    let addr = ADO_ADDRESSES.load(deps.storage, name.as_str())?;
    let proxy_msg = SubMsg {
        id: 102,
        reply_on: ReplyOn::Error,
        msg: CosmosMsg::Wasm(WasmMsg::Execute {
            msg,
            funds: info.funds,
            contract_addr: addr.to_string(),
        }),
        gas_limit: None,
    };

    Ok(Response::default()
        .add_submessage(proxy_msg)
        .add_attribute("method", "app_message")
        .add_attribute("recipient", name))
}

fn has_update_address_privilege(
    storage: &dyn Storage,
    sender: &str,
    current_addr: &str,
) -> Result<bool, ContractError> {
    Ok(ADOContract::default().is_contract_owner(storage, sender)? || sender == current_addr)
}

fn execute_update_address(
    deps: DepsMut,
    info: MessageInfo,
    name: String,
    addr: String,
) -> Result<Response, ContractError> {
    let ado_addr = ADO_ADDRESSES.load(deps.storage, &name)?;
    ensure!(
        has_update_address_privilege(deps.storage, info.sender.as_str(), ado_addr.as_str())?,
        ContractError::Unauthorized {}
    );

    let new_addr = deps.api.addr_validate(&addr)?;
    ADO_ADDRESSES.save(deps.storage, &name, &new_addr)?;

    Ok(Response::default()
        .add_attribute("method", "update_address")
        .add_attribute("name", name)
        .add_attribute("address", addr))
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
        QueryMsg::GetAddress { name } => encode_binary(&query_component_address(deps, name)?),
        QueryMsg::GetAddresses {} => encode_binary(&query_component_addresses(deps)?),
        QueryMsg::GetComponents {} => encode_binary(&query_component_descriptors(deps)?),
        QueryMsg::Config {} => encode_binary(&query_config(deps)?),
        QueryMsg::ComponentExists { name } => encode_binary(&query_component_exists(deps, name)),
    }
}

fn handle_andromeda_query(
    deps: Deps,
    env: Env,
    msg: AndromedaQuery,
) -> Result<Binary, ContractError> {
    match msg {
        AndromedaQuery::Get(data) => match data {
            None => Err(ContractError::ParsingError {
                err: String::from("No data passed with AndrGet query"),
            }),
            Some(_) => {
                //Default to get address for given ADO name
                let name: String = parse_message(&data)?;
                encode_binary(&query_component_address(deps, name)?)
            }
        },
        _ => ADOContract::default().query(deps, env, msg, query),
    }
}

fn query_component_address(deps: Deps, name: String) -> Result<String, ContractError> {
    let value = ADO_ADDRESSES.load(deps.storage, &name)?;
    Ok(value.to_string())
}

fn query_component_descriptors(deps: Deps) -> Result<Vec<AppComponent>, ContractError> {
    let value = load_component_descriptors(deps.storage)?;
    Ok(value)
}

fn query_component_exists(deps: Deps, name: String) -> bool {
    ADO_ADDRESSES.has(deps.storage, &name)
}

fn query_component_addresses(deps: Deps) -> Result<Vec<ComponentAddress>, ContractError> {
    let value = load_component_addresses_with_name(deps.storage)?;
    Ok(value)
}

fn query_config(deps: Deps) -> Result<ConfigResponse, ContractError> {
    let name = APP_NAME.load(deps.storage)?;
    let owner = ADOContract::default().query_contract_owner(deps)?.owner;

    Ok(ConfigResponse { name, owner })
}
