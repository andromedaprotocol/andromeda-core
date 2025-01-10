use crate::state::{
    add_app_component, generate_assign_app_message, generate_ownership_message,
    load_component_addresses, ADO_ADDRESSES, APP_NAME,
};
use andromeda_app::app::{AppComponent, ComponentType};
use andromeda_std::common::{context::ExecuteContext, reply::ReplyId};
use andromeda_std::error::ContractError;
use andromeda_std::os::aos_querier::AOSQuerier;
use andromeda_std::os::vfs::ExecuteMsg as VFSExecuteMsg;
use andromeda_std::{ado_contract::ADOContract, amp::AndrAddr};

use cosmwasm_std::{
    ensure, to_json_binary, Addr, Binary, CosmosMsg, QuerierWrapper, ReplyOn, Response, Storage,
    SubMsg, WasmMsg,
};

pub fn handle_add_app_component(
    ctx: ExecuteContext,
    component: AppComponent,
) -> Result<Response, ContractError> {
    let querier = &ctx.deps.querier;
    let env = ctx.env;
    let sender = ctx.info.sender.as_str();

    let maybe_app_component = ADO_ADDRESSES.may_load(ctx.deps.storage, &component.name)?;
    ensure!(
        maybe_app_component.is_none(),
        ContractError::InvalidComponent {
            name: "Component name already taken".to_string()
        }
    );

    ensure!(
        !matches!(component.component_type, ComponentType::CrossChain(..)),
        ContractError::CrossChainComponentsCurrentlyDisabled {}
    );

    let idx = add_app_component(ctx.deps.storage, &component)?;
    ensure!(idx < 50, ContractError::TooManyAppComponents {});

    let adodb_addr = ADOContract::default().get_adodb_address(ctx.deps.storage, querier)?;
    let vfs_addr = ADOContract::default().get_vfs_address(ctx.deps.storage, querier)?;

    let mut resp = Response::new()
        .add_attribute("method", "add_app_component")
        .add_attribute("name", component.name.clone())
        .add_attribute("type", component.ado_type.clone());

    let app_name = APP_NAME.load(ctx.deps.storage)?;
    let new_addr = component.get_new_addr(
        ctx.deps.api,
        &adodb_addr,
        querier,
        env.contract.address.clone(),
    )?;
    let registration_msg = component.generate_vfs_registration(
        new_addr.clone(),
        &env.contract.address,
        &app_name,
        // TODO: Fix this in future for x-chain components
        None,
        &adodb_addr,
        &vfs_addr,
    )?;

    if let Some(registration_msg) = registration_msg {
        resp = resp.add_submessage(registration_msg);
    }

    let inst_msg = component.generate_instantiation_message(
        querier,
        &adodb_addr,
        &env.contract.address,
        sender,
        idx,
    )?;

    if let Some(inst_msg) = inst_msg {
        resp = resp.add_submessage(inst_msg);
    }

    if let ComponentType::Symlink(ref val) = component.component_type {
        let component_address: Addr = val.get_raw_address(&ctx.deps.as_ref())?;
        ADO_ADDRESSES.save(ctx.deps.storage, &component.name, &component_address)?;
    } else if let ComponentType::New(_) = component.component_type {
        ensure!(
            new_addr.is_some(),
            ContractError::InvalidComponent {
                name: "Could not generate address for new component".to_string()
            }
        );
        ADO_ADDRESSES.save(
            ctx.deps.storage,
            &component.name,
            &new_addr.clone().unwrap(),
        )?;
    }

    let event = component.generate_event(new_addr);
    resp = resp.add_event(event);

    Ok(resp)
}

pub fn claim_ownership(
    ctx: ExecuteContext,
    name_opt: Option<String>,
    new_owner: Option<AndrAddr>,
) -> Result<Response, ContractError> {
    ensure!(
        ADOContract::default().is_contract_owner(ctx.deps.storage, ctx.info.sender.as_str())?
            || ctx.env.contract.address == ctx.info.sender,
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
            let curr_owner = AOSQuerier::ado_owner_getter(&ctx.deps.querier, &address)?;
            // Get the AndrAddr's raw address if available, else get the message sender's address.
            if curr_owner == ctx.env.contract.address {
                let new_owner = if let Some(new_owner) = new_owner.clone() {
                    new_owner.get_raw_address(&ctx.deps.as_ref())?
                } else {
                    ctx.info.sender.clone()
                };
                msgs.push(generate_ownership_message(address, new_owner.as_str())?);
            }
        }
    }

    Ok(Response::new()
        .add_submessages(msgs)
        .add_attribute("method", "claim_ownership"))
}

pub fn message(ctx: ExecuteContext, name: String, msg: Binary) -> Result<Response, ContractError> {
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

pub fn has_update_address_privilege(
    storage: &dyn Storage,
    sender: &str,
    current_addr: &str,
) -> Result<bool, ContractError> {
    Ok(ADOContract::default().is_contract_owner(storage, sender)? || sender == current_addr)
}

pub fn update_address(
    ctx: ExecuteContext,
    name: String,
    addr: String,
) -> Result<Response, ContractError> {
    ctx.deps.api.addr_validate(addr.as_str())?;
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
            kernel_address,
            &deps.querier,
            name,
            deps.api.addr_validate(&addr)?,
        )?;

        resp = resp.add_submessage(register_component_path_msg)
    }

    Ok(resp)
}

pub fn register_component_path(
    kernel_address: Addr,
    querier: &QuerierWrapper,
    name: impl Into<String>,
    address: Addr,
) -> Result<SubMsg, ContractError> {
    let vfs_address: Addr = AOSQuerier::vfs_address_getter(querier, &kernel_address)?;

    let add_path_msg = VFSExecuteMsg::AddPath {
        name: name.into(),
        address,
        parent_address: None,
    };
    let cosmos_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: vfs_address.to_string(),
        msg: to_json_binary(&add_path_msg)?,
        funds: vec![],
    });

    Ok(SubMsg::reply_on_error(
        cosmos_msg,
        ReplyId::RegisterPath.repr(),
    ))
}

pub fn assign_app_to_components(ctx: ExecuteContext) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps, env, info, ..
    } = ctx;
    let mut resp = Response::default();
    ensure!(
        info.sender == env.contract.address,
        ContractError::Unauthorized {}
    );

    let addresses = load_component_addresses(deps.storage, None)?;
    for address in addresses {
        let assign_app_msg = generate_assign_app_message(&address, env.contract.address.as_str())?;
        resp = resp
            .add_submessage(assign_app_msg)
            .add_attribute("assign_app", address);
    }

    Ok(resp)
}
