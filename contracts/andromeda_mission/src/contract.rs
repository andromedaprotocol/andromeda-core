#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, QuerierWrapper, Reply, ReplyOn, Response,
    StdError, Storage, SubMsg, WasmMsg,
};
use cw2::{get_contract_version, set_contract_version};

use crate::state::{
    add_mission_component, generate_ownership_message, load_component_addresses, ADO_ADDRESSES,
    ADO_DESCRIPTORS, MISSION_NAME,
};

use andromeda_protocol::{
    communication::{encode_binary, parse_message, AndromedaMsg, AndromedaQuery},
    error::ContractError,
    mission::{ExecuteMsg, InstantiateMsg, MigrateMsg, MissionComponent, QueryMsg},
    operators::{
        execute_update_operators, initialize_operators, query_is_operator, query_operators,
    },
    ownership::{execute_update_owner, is_contract_owner, query_contract_owner, CONTRACT_OWNER},
    require,
    response::get_reply_address,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda_primitive";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    initialize_operators(deps.storage, msg.operators)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CONTRACT_OWNER.save(deps.storage, &info.sender)?;
    MISSION_NAME.save(deps.storage, &msg.name)?;

    require(
        msg.mission.len() <= 50,
        ContractError::TooManyMissionComponents {},
    )?;
    let sender = info.sender.as_str();

    let mut resp = Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", sender)
        .add_attribute("andr_mission", msg.name);

    for component in msg.mission {
        let comp_resp =
            execute_add_mission_component(&deps.querier, deps.storage, sender, component)?;
        resp = resp.add_submessages(comp_resp.messages);
    }

    if msg.xfer_ado_ownership {
        let own_resp = execute_claim_ownership(deps.storage, sender, None)?;
        resp = resp.add_submessages(own_resp.messages)
    }

    Ok(resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
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

    let addr = get_reply_address(&msg)?;
    ADO_ADDRESSES.save(deps.storage, &id, &deps.api.addr_validate(&addr)?)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AndrReceive(msg) => execute_receive(deps, env, info, msg),
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
    }
}

fn execute_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: AndromedaMsg,
) -> Result<Response, ContractError> {
    match msg {
        AndromedaMsg::Receive(data) => {
            let received: ExecuteMsg = parse_message(data)?;
            match received {
                ExecuteMsg::AndrReceive(..) => Err(ContractError::NestedAndromedaMsg {}),
                _ => execute(deps, env, info, received),
            }
        }
        AndromedaMsg::UpdateOwner { address } => execute_update_owner(deps, info, address),
        AndromedaMsg::UpdateOperators { operators } => {
            execute_update_operators(deps, info, operators)
        }
        AndromedaMsg::Withdraw { .. } => Err(ContractError::UnsupportedOperation {}),
    }
}

fn execute_add_mission_component(
    querier: &QuerierWrapper,
    storage: &mut dyn Storage,
    sender: &str,
    component: MissionComponent,
) -> Result<Response, ContractError> {
    require(
        is_contract_owner(storage, sender)?,
        ContractError::Unauthorized {},
    )?;
    let mut resp = Response::new();

    let idx = add_mission_component(storage, &component)?;
    let inst_msg = component.generate_instantiate_msg(storage, querier, idx)?;
    resp = resp.add_submessage(inst_msg);

    Ok(resp)
}

fn execute_claim_ownership(
    storage: &mut dyn Storage,
    sender: &str,
    name_opt: Option<String>,
) -> Result<Response, ContractError> {
    require(
        is_contract_owner(storage, sender)?,
        ContractError::Unauthorized {},
    )?;

    let mut resp = Response::new();

    if let Some(name) = name_opt {
        let address = ADO_ADDRESSES.load(storage, &name)?;
        resp = resp.add_submessage(generate_ownership_message(address, sender)?);
    } else {
        let addresses = load_component_addresses(storage)?;
        for addr in addresses {
            resp = resp.add_submessage(generate_ownership_message(addr, sender)?);
        }
    }

    Ok(resp)
}

fn execute_message(
    deps: DepsMut,
    info: MessageInfo,
    name: String,
    msg: Binary,
) -> Result<Response, ContractError> {
    //Temporary until message sender attached to Andromeda Comms
    require(
        is_contract_owner(deps.storage, info.sender.as_str())?,
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
        .add_attribute("reciient", name))
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
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrQuery(msg) => handle_andromeda_query(deps, msg),
    }
}

fn handle_andromeda_query(deps: Deps, msg: AndromedaQuery) -> Result<Binary, ContractError> {
    match msg {
        AndromedaQuery::Get(data) => match data {
            None => Err(ContractError::ParsingError {
                err: String::from("No data passed with AndrGet query"),
            }),
            Some(_) => {
                //Default to get address for given ADO name
                let name: String = parse_message(data)?;
                encode_binary(&query_ado_address(deps, name)?)
            }
        },
        AndromedaQuery::Owner {} => encode_binary(&query_contract_owner(deps)?),
        AndromedaQuery::Operators {} => encode_binary(&query_operators(deps)?),
        AndromedaQuery::IsOperator { address } => {
            encode_binary(&query_is_operator(deps, &address)?)
        }
    }
}

fn query_ado_address(deps: Deps, name: String) -> Result<String, ContractError> {
    let value = ADO_ADDRESSES.load(deps.storage, &name)?;
    Ok(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            operators: vec![],
            mission: vec![],
            xfer_ado_ownership: false,
            name: String::from("Some Mission"),
        };
        let info = mock_info("creator", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }
}
