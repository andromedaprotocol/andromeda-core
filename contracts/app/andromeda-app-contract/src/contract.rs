use crate::state::{
    add_app_component, generate_assign_app_message, generate_ownership_message,
    load_component_addresses, load_component_addresses_with_name, load_component_descriptors,
    ADO_ADDRESSES, ADO_DESCRIPTORS, ADO_IDX, APP_NAME, ASSIGNED_IDX,
};
use andromeda_app::app::{
    AppComponent, ComponentAddress, ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg,
    QueryMsg,
};
use andromeda_std::ado_contract::ADOContract;
use andromeda_std::amp::VFS_KEY;
use andromeda_std::common::context::ExecuteContext;
use andromeda_std::os::{
    kernel::QueryMsg as KernelQueryMsg,
    vfs::{convert_component_name, validate_component_name, ExecuteMsg as VFSExecuteMsg},
};
use andromeda_std::{
    ado_base::InstantiateMsg as BaseInstantiateMsg,
    common::{encode_binary, response::get_reply_address},
    error::{from_semver, ContractError},
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo,
    QuerierWrapper, Reply, ReplyOn, Response, StdError, Storage, SubMsg, WasmMsg,
};
use cw2::{get_contract_version, set_contract_version};

use semver::Version;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-app-contract";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const REGISTER_PATH_MSG_ID: u64 = 1001;
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
            info.clone(),
            BaseInstantiateMsg {
                ado_type: "app".to_string(),
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
        let comp_resp = execute_add_app_component(&deps.querier, deps.storage, &sender, component)?;
        msgs.extend(comp_resp.messages);
    }
    let vfs_address = ADOContract::default().get_vfs_address(deps.storage, &deps.querier)?;

    let add_path_msg = VFSExecuteMsg::AddParentPath {
        name: convert_component_name(msg.name),
        parent_address: info.sender,
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
        let register_component_path_msg = register_component_path(
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

pub fn register_component_path(
    kernel_address: String,
    querier: &QuerierWrapper,
    name: impl Into<String>,
    address: Addr,
) -> Result<SubMsg, ContractError> {
    let vfs_address_query = KernelQueryMsg::KeyAddress {
        key: VFS_KEY.to_string(),
    };
    let vfs_address: Addr = querier.query_wasm_smart(kernel_address, &vfs_address_query)?;

    let add_path_msg = VFSExecuteMsg::AddPath {
        name: name.into(),
        address,
    };
    let cosmos_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: vfs_address.to_string(),
        msg: to_binary(&add_path_msg)?,
        funds: vec![],
    });

    Ok(SubMsg::reply_on_error(cosmos_msg, REGISTER_PATH_MSG_ID))
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
        ExecuteMsg::AddAppComponent { component } => execute_add_app_component(
            &ctx.deps.querier,
            ctx.deps.storage,
            ctx.info.sender.as_str(),
            component,
        ),
        ExecuteMsg::ClaimOwnership { name } => execute_claim_ownership(ctx, name),
        ExecuteMsg::ProxyMessage { msg, name } => execute_message(ctx, name, msg),
        ExecuteMsg::UpdateAddress { name, addr } => execute_update_address(ctx, name, addr),
        _ => ADOContract::default().execute(ctx, msg),
    }
}

fn execute_add_app_component(
    querier: &QuerierWrapper,
    storage: &mut dyn Storage,
    sender: &str,
    component: AppComponent,
) -> Result<Response, ContractError> {
    validate_component_name(component.name.clone())?;
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
    ctx: ExecuteContext,
    name_opt: Option<String>,
) -> Result<Response, ContractError> {
    ensure!(
        ADOContract::default().is_contract_owner(ctx.deps.storage, ctx.info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    let mut msgs: Vec<SubMsg> = vec![];
    if let Some(name) = name_opt {
        let address = ADO_ADDRESSES.load(ctx.deps.storage, &name)?;
        msgs.push(generate_ownership_message(
            address,
            ctx.info.sender.as_str(),
        )?);
    } else {
        let addresses = load_component_addresses(ctx.deps.storage, None)?;
        for address in addresses {
            msgs.push(generate_ownership_message(
                address,
                ctx.info.sender.as_str(),
            )?);
        }
    }

    Ok(Response::new()
        .add_submessages(msgs)
        .add_attribute("method", "claim_ownership"))
}

fn execute_message(
    ctx: ExecuteContext,
    name: String,
    msg: Binary,
) -> Result<Response, ContractError> {
    let ExecuteContext { info, deps, .. } = ctx;
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
    ctx: ExecuteContext,
    name: String,
    addr: String,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, info, .. } = ctx;
    let ado_addr = ADO_ADDRESSES.load(deps.storage, &name)?;
    ensure!(
        has_update_address_privilege(deps.storage, info.sender.as_str(), ado_addr.as_str())?,
        ContractError::Unauthorized {}
    );

    let new_addr = deps.api.addr_validate(&addr)?;
    ADO_ADDRESSES.save(deps.storage, &name, &new_addr)?;

    let mut resp = Response::default()
        .add_attribute("method", "update_address")
        .add_attribute("name", name.clone())
        .add_attribute("address", addr.clone());

    if !name.starts_with('.') {
        let kernel_address = ADOContract::default().get_kernel_address(deps.storage)?;
        let register_component_path_msg = register_component_path(
            kernel_address.to_string(),
            &deps.querier,
            name,
            deps.api.addr_validate(&addr)?,
        )?;

        resp = resp.add_submessage(register_component_path_msg)
    }

    Ok(resp)
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
        QueryMsg::GetAddress { name } => encode_binary(&query_component_address(deps, name)?),
        QueryMsg::GetAddressesWithNames {} => {
            encode_binary(&query_component_addresses_with_name(deps)?)
        }
        QueryMsg::GetComponents {} => encode_binary(&query_component_descriptors(deps)?),
        QueryMsg::Config {} => encode_binary(&query_config(deps)?),
        QueryMsg::ComponentExists { name } => encode_binary(&query_component_exists(deps, name)),
        _ => ADOContract::default().query(deps, env, msg),
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

fn query_component_addresses_with_name(deps: Deps) -> Result<Vec<ComponentAddress>, ContractError> {
    let value = load_component_addresses_with_name(deps.storage)?;
    Ok(value)
}

fn query_config(deps: Deps) -> Result<ConfigResponse, ContractError> {
    let name = APP_NAME.load(deps.storage)?;
    let owner = ADOContract::default().query_contract_owner(deps)?.owner;

    Ok(ConfigResponse { name, owner })
}
