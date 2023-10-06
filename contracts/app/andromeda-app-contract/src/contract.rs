use crate::state::{
    generate_assign_app_message, load_component_addresses, load_component_addresses_with_name,
    load_component_descriptors, ADO_ADDRESSES, ADO_DESCRIPTORS, ADO_IDX, APP_NAME, ASSIGNED_IDX,
};
use andromeda_app::app::{
    AppComponent, ComponentAddress, ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg,
    QueryMsg,
};
use andromeda_std::ado_contract::ADOContract;
use andromeda_std::amp::AndrAddr;
use andromeda_std::common::context::ExecuteContext;
use andromeda_std::os::vfs::{convert_component_name, ExecuteMsg as VFSExecuteMsg};
use andromeda_std::{
    ado_base::InstantiateMsg as BaseInstantiateMsg,
    common::{encode_binary, response::get_reply_address},
    error::{from_semver, ContractError},
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Reply,
    Response, StdError, SubMsg, WasmMsg,
};
use cw2::{get_contract_version, set_contract_version};

use crate::{execute, query};
use semver::Version;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-app-contract";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const REGISTER_PARENT_PATH_MSG_MSG: u64 = 1002;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    APP_NAME.save(deps.storage, &msg.name)?;

    ensure!(
        msg.app_components.len() <= 50,
        ContractError::TooManyAppComponents {}
    );

    let sender = info.sender.to_string();
    let resp = ADOContract::default()
        .instantiate(
            deps.storage,
            env,
            deps.api,
            info,
            BaseInstantiateMsg {
                ado_type: "app-contract".to_string(),
                ado_version: CONTRACT_VERSION.to_string(),
                operators: None,
                kernel_address: msg.kernel_address.clone(),
                owner: msg.owner,
            },
        )?
        .add_attribute("owner", &sender)
        .add_attribute("andr_app", msg.name.clone());

    let mut msgs: Vec<SubMsg> = vec![];
    for component in msg.app_components {
        component.verify(&deps.as_ref()).unwrap();
        let comp_resp =
            execute::handle_add_app_component(&deps.querier, deps.storage, &sender, component)?;
        msgs.extend(comp_resp.messages);
    }
    let vfs_address = ADOContract::default().get_vfs_address(deps.storage, &deps.querier)?;

    let add_path_msg = VFSExecuteMsg::AddParentPath {
        name: convert_component_name(msg.name),
        parent_address: AndrAddr::from_string(format!("~{sender}")),
    };
    let cosmos_msg: CosmosMsg<Empty> = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: vfs_address.to_string(),
        msg: to_binary(&add_path_msg)?,
        funds: vec![],
    });

    let register_msg = SubMsg::reply_on_error(cosmos_msg, REGISTER_PARENT_PATH_MSG_MSG);

    Ok(resp.add_submessage(register_msg).add_submessages(msgs))
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

    let mut resp = Response::default();

    if !descriptor.name.starts_with('.') {
        let kernel_address = ADOContract::default().get_kernel_address(deps.storage)?;
        let register_component_path_msg = execute::register_component_path(
            kernel_address.to_string(),
            &deps.querier,
            descriptor.name,
            addr.clone(),
        )?;

        resp = resp.add_submessage(register_component_path_msg)
    }

    // Once all components are registered we need to register them with the VFS
    let curr_idx = ADO_IDX.load(deps.storage)?;
    if id == (curr_idx - 1).to_string() {
        // Only assign app to new components
        let assigned_idx = ASSIGNED_IDX.load(deps.storage).unwrap_or(1);
        let addresses: Vec<Addr> =
            load_component_addresses(deps.storage, Some(assigned_idx.to_string().as_str()))?;
        for address in addresses {
            let assign_app_msg =
                generate_assign_app_message(&address, env.contract.address.as_str())?;
            resp = resp
                .add_submessage(assign_app_msg)
                .add_attribute("assign_app", address);
        }
        ASSIGNED_IDX.save(deps.storage, &curr_idx)?;
    }

    Ok(resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let ctx = ExecuteContext::new(deps, info, env);
    match msg {
        ExecuteMsg::AMPReceive(pkt) => {
            ADOContract::default().execute_amp_receive(ctx, pkt, handle_execute)
        }
        _ => handle_execute(ctx, msg),
    }
}

pub fn handle_execute(ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AddAppComponent { component } => execute::handle_add_app_component(
            &ctx.deps.querier,
            ctx.deps.storage,
            ctx.info.sender.as_str(),
            component,
        ),
        ExecuteMsg::ClaimOwnership { name } => execute::claim_ownership(ctx, name),
        ExecuteMsg::ProxyMessage { msg, name } => execute::message(ctx, name, msg),
        ExecuteMsg::UpdateAddress { name, addr } => execute::update_address(ctx, name, addr),
        _ => ADOContract::default().execute(ctx, msg),
    }
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
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::GetAddress { name } => encode_binary(&query::component_address(deps, name)?),
        QueryMsg::GetAddressesWithNames {} => {
            encode_binary(&query::component_addresses_with_name(deps)?)
        }
        QueryMsg::GetComponents {} => encode_binary(&query::component_descriptors(deps)?),
        QueryMsg::Config {} => encode_binary(&query::config(deps)?),
        QueryMsg::ComponentExists { name } => encode_binary(&query::component_exists(deps, name)),
        _ => ADOContract::default().query(deps, env, msg),
    }
}
