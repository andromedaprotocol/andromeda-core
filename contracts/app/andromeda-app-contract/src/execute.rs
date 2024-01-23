use crate::state::{
    add_app_component, generate_assign_app_message, generate_ownership_message,
    load_component_addresses, ADO_ADDRESSES,
};
use andromeda_app::app::{AppComponent, ComponentType};
use andromeda_std::ado_contract::ADOContract;
use andromeda_std::common::context::ExecuteContext;
use andromeda_std::error::ContractError;
use andromeda_std::os::aos_querier::AOSQuerier;
use andromeda_std::os::vfs::ExecuteMsg as VFSExecuteMsg;

use crate::reply::ReplyId;
use cosmwasm_std::{
    ensure, to_binary, Addr, Binary, CosmosMsg, Order, QuerierWrapper, ReplyOn, Response, Storage,
    SubMsg, WasmMsg,
};

pub fn handle_add_app_component(
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

    let amount = ADO_ADDRESSES
        .keys(storage, None, None, Order::Ascending)
        .count();
    ensure!(amount < 50, ContractError::TooManyAppComponents {});

    let current_addr = ADO_ADDRESSES.may_load(storage, &component.name)?;
    ensure!(current_addr.is_none(), ContractError::NameAlreadyTaken {});

    let idx = add_app_component(storage, &component)?;

    let mut resp = Response::new()
        .add_attribute("method", "add_app_component")
        .add_attribute("name", component.name.clone())
        .add_attribute("type", component.ado_type.clone());

    match component.component_type {
        ComponentType::New(instantiate_msg) => {
            let inst_msg = generate_instantiate_msg(
                storage,
                querier,
                idx,
                instantiate_msg,
                component.ado_type.clone(),
                sender.to_string(),
            )?;
            resp = resp.add_submessage(inst_msg);
            ADO_ADDRESSES.save(storage, &component.name, &Addr::unchecked(""))?;
        }
        ComponentType::Symlink(symlink) => {
            let msg = VFSExecuteMsg::AddSymlink {
                name: component.name,
                symlink,
                parent_address: None,
            };
            let cosmos_msg = CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: AOSQuerier::vfs_address_getter(
                    querier,
                    &contract.get_kernel_address(storage)?,
                )?
                .to_string(),
                msg: to_binary(&msg)?,
                funds: vec![],
            });
            let sub_msg = SubMsg::reply_on_error(cosmos_msg, ReplyId::RegisterPath.repr());
            resp = resp.add_submessage(sub_msg);
        }
        _ => return Err(ContractError::Unauthorized {}),
    }

    Ok(resp)
}

fn generate_instantiate_msg(
    storage: &mut dyn Storage,
    querier: &QuerierWrapper,
    msg_id: u64,
    msg: Binary,
    ado_type: String,
    sender: String,
) -> Result<SubMsg, ContractError> {
    let adodb_addr = ADOContract::default().get_adodb_address(storage, querier)?;
    match AOSQuerier::code_id_getter(querier, &adodb_addr, &ado_type) {
        Err(_) => Err(ContractError::InvalidModule {
            msg: Some(String::from(
                "ADO type provided does not have a valid Code Id",
            )),
        }),
        Ok(code_id) => Ok(SubMsg {
            id: msg_id,
            reply_on: ReplyOn::Always,
            msg: CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: Some(sender),
                code_id,
                msg,
                funds: vec![],
                label: format!("Instantiate: {ado_type}"),
            }),
            gas_limit: None,
        }),
    }
}

pub fn claim_ownership(
    ctx: ExecuteContext,
    name_opt: Option<String>,
    new_owner: Option<Addr>,
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
            if curr_owner == ctx.env.contract.address {
                msgs.push(generate_ownership_message(
                    address,
                    new_owner
                        .clone()
                        .unwrap_or(ctx.info.sender.clone())
                        .as_str(),
                )?);
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
        msg: to_binary(&add_path_msg)?,
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
