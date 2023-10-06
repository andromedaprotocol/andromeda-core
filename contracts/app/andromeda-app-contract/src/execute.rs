use crate::state::{
    add_app_component, generate_ownership_message, load_component_addresses, ADO_ADDRESSES,
};
use andromeda_app::app::AppComponent;
use andromeda_std::ado_contract::ADOContract;
use andromeda_std::amp::VFS_KEY;
use andromeda_std::common::context::ExecuteContext;
use andromeda_std::error::ContractError;
use andromeda_std::os::{
    kernel::QueryMsg as KernelQueryMsg,
    vfs::{validate_component_name, ExecuteMsg as VFSExecuteMsg},
};

use cosmwasm_std::{
    ensure, to_binary, Addr, Binary, CosmosMsg, QuerierWrapper, ReplyOn, Response, Storage, SubMsg,
    WasmMsg,
};

pub fn handle_add_app_component(
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

pub fn claim_ownership(
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

const REGISTER_PATH_MSG_ID: u64 = 1001;

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
        parent_address: None,
    };
    let cosmos_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: vfs_address.to_string(),
        msg: to_binary(&add_path_msg)?,
        funds: vec![],
    });

    Ok(SubMsg::reply_on_error(cosmos_msg, REGISTER_PATH_MSG_ID))
}
