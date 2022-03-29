use crate::state::{
    add_mission_component, generate_assign_mission_message, generate_ownership_message,
    load_component_addresses, load_component_descriptors, ADO_ADDRESSES, ADO_DESCRIPTORS,
    MISSION_NAME,
};
use ado_base::ADOContract;
use andromeda_protocol::mission::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, MissionComponent, QueryMsg,
};
use common::{
    ado_base::{AndromedaQuery, InstantiateMsg as BaseInstantiateMsg},
    encode_binary,
    error::ContractError,
    parse_message, require,
    response::get_reply_address,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, QuerierWrapper, Reply, ReplyOn,
    Response, StdError, Storage, SubMsg, WasmMsg,
};
use cw2::{get_contract_version, set_contract_version};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda_mission";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    MISSION_NAME.save(deps.storage, &msg.name)?;
    require(
        msg.mission.len() <= 50,
        ContractError::TooManyMissionComponents {},
    )?;

    let sender = info.sender.to_string();
    let resp = ADOContract::default()
        .instantiate(
            deps.storage,
            deps.api,
            info,
            BaseInstantiateMsg {
                ado_type: "mission".to_string(),
                operators: Some(msg.operators),
                modules: None,
                primitive_contract: Some(msg.primitive_contract),
            },
        )?
        .add_attribute("owner", &sender)
        .add_attribute("andr_mission", msg.name);

    let mut msgs: Vec<SubMsg> = vec![];
    for component in msg.mission {
        let comp_resp =
            execute_add_mission_component(&deps.querier, deps.storage, &sender, component)?;
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
    require(
        ADO_DESCRIPTORS.load(deps.storage, &id).is_ok(),
        ContractError::InvalidReplyId {},
    )?;

    let addr_str = get_reply_address(&msg)?;
    let addr = &deps.api.addr_validate(&addr_str)?;
    ADO_ADDRESSES.save(deps.storage, &id, addr)?;
    let assign_mission = generate_assign_mission_message(addr, &env.contract.address.to_string())?;
    Ok(Response::default().add_submessage(assign_mission))
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
        ExecuteMsg::AddMissionComponent { component } => execute_add_mission_component(
            &deps.querier,
            deps.storage,
            info.sender.as_str(),
            component,
        ),
        ExecuteMsg::ClaimOwnership { name } => {
            execute_claim_ownership(deps.storage, info.sender.as_str(), name)
        }
        ExecuteMsg::ProxyMessage { msg, name } => execute_message(deps, info, name, msg),
        ExecuteMsg::UpdateAddress { name, addr } => execute_update_address(deps, info, name, addr),
    }
}

fn execute_add_mission_component(
    querier: &QuerierWrapper,
    storage: &mut dyn Storage,
    sender: &str,
    component: MissionComponent,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    require(
        contract.is_contract_owner(storage, sender)?,
        ContractError::Unauthorized {},
    )?;

    let current_addr = ADO_ADDRESSES.may_load(storage, &component.name)?;
    require(current_addr.is_none(), ContractError::NameAlreadyTaken {})?;

    let idx = add_mission_component(storage, &component)?;
    let inst_msg = contract.generate_instantiate_msg(
        storage,
        querier,
        idx,
        component.instantiate_msg,
        component.ado_type.clone(),
    )?;

    Ok(Response::new()
        .add_submessage(inst_msg)
        .add_attribute("method", "add_mission_component")
        .add_attribute("name", component.name)
        .add_attribute("type", component.ado_type))
}

fn execute_claim_ownership(
    storage: &mut dyn Storage,
    sender: &str,
    name_opt: Option<String>,
) -> Result<Response, ContractError> {
    require(
        ADOContract::default().is_contract_owner(storage, sender)?,
        ContractError::Unauthorized {},
    )?;

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
    require(
        ADOContract::default().is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;

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
        .add_attribute("method", "mission_message")
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
    require(
        has_update_address_privilege(deps.storage, info.sender.as_str(), ado_addr.as_str())?,
        ContractError::Unauthorized {},
    )?;

    let new_addr = deps.api.addr_validate(&addr)?;
    ADO_ADDRESSES.save(deps.storage, &name, &new_addr)?;

    Ok(Response::default()
        .add_attribute("method", "update_address")
        .add_attribute("name", name)
        .add_attribute("address", addr))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let version = get_contract_version(deps.storage)?;
    if version.contract != CONTRACT_NAME {
        return Err(ContractError::CannotMigrate {
            previous_contract: version.contract,
        });
    }
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrQuery(msg) => handle_andromeda_query(deps, env, msg),
        QueryMsg::GetAddress { name } => encode_binary(&query_component_address(deps, name)?),
        QueryMsg::GetAddresses {} => encode_binary(&query_component_addresses(deps)?),
        QueryMsg::GetComponents {} => encode_binary(&query_component_descriptors(deps)?),
        QueryMsg::Config {} => encode_binary(&query_config(deps)?),
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

fn query_component_descriptors(deps: Deps) -> Result<Vec<MissionComponent>, ContractError> {
    let value = load_component_descriptors(deps.storage)?;
    Ok(value)
}

fn query_component_addresses(deps: Deps) -> Result<Vec<Addr>, ContractError> {
    let value = load_component_addresses(deps.storage)?;
    Ok(value)
}

fn query_config(deps: Deps) -> Result<ConfigResponse, ContractError> {
    let name = MISSION_NAME.load(deps.storage)?;
    let owner = ADOContract::default().query_contract_owner(deps)?.owner;

    Ok(ConfigResponse { name, owner })
}
