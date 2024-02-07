use crate::reply::{on_component_instantiation, ReplyId};
use crate::state::{create_cross_chain_message, get_chain_info, APP_NAME};
use andromeda_app::app::{
    AppComponent, ComponentType, CrossChainComponent, ExecuteMsg, InstantiateMsg, MigrateMsg,
    QueryMsg,
};
use andromeda_std::ado_contract::ADOContract;
use andromeda_std::amp::AndrAddr;
use andromeda_std::common::context::ExecuteContext;
use andromeda_std::os::vfs::{convert_component_name, ExecuteMsg as VFSExecuteMsg};
use andromeda_std::{
    ado_base::InstantiateMsg as BaseInstantiateMsg,
    common::encode_binary,
    error::{from_semver, ContractError},
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, to_binary, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Reply, Response,
    StdError, SubMsg, WasmMsg,
};
use cw2::{get_contract_version, set_contract_version};

use crate::{execute, query};
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

    ensure!(
        msg.app_components.len() <= 50,
        ContractError::TooManyAppComponents {}
    );

    let sender = msg.owner.clone().unwrap_or(info.sender.to_string());
    let mut resp = ADOContract::default()
        .instantiate(
            deps.storage,
            env.clone(),
            deps.api,
            info.clone(),
            BaseInstantiateMsg {
                ado_type: "app-contract".to_string(),
                ado_version: CONTRACT_VERSION.to_string(),
                operators: None,
                kernel_address: msg.kernel_address.clone(),
                owner: msg.owner.clone(),
            },
        )?
        .add_attribute("owner", &msg.owner.clone().unwrap_or(sender.clone()))
        .add_attribute("andr_app", msg.name.clone());

    let mut msgs: Vec<SubMsg> = vec![];
    let app_name = msg.name;
    for component in msg.app_components.clone() {
        component.verify(&deps.as_ref()).unwrap();
        match component.component_type {
            ComponentType::CrossChain(CrossChainComponent { chain, .. }) => {
                let chain_info = get_chain_info(chain.clone(), msg.chain_info.clone());
                ensure!(
                    chain_info.is_some(),
                    ContractError::InvalidComponent {
                        name: component.name.clone()
                    }
                );
                let owner_addr = chain_info.unwrap().owner;
                let name = component.name;
                let new_component = AppComponent {
                    name: name.clone(),
                    ado_type: component.ado_type,
                    component_type: ComponentType::Symlink(AndrAddr::from_string(format!(
                        "ibc://{chain}/home/{owner_addr}/{app_name}/{name}"
                    ))),
                };
                let comp_resp = execute::handle_add_app_component(
                    &deps.querier,
                    deps.storage,
                    &sender,
                    new_component,
                )?;
                msgs.extend(comp_resp.messages);
            }
            _ => {
                let comp_resp = execute::handle_add_app_component(
                    &deps.querier,
                    deps.storage,
                    &sender,
                    component,
                )?;
                msgs.extend(comp_resp.messages);
            }
        }
    }
    let vfs_address = ADOContract::default().get_vfs_address(deps.storage, &deps.querier)?;

    let add_path_msg = VFSExecuteMsg::AddChild {
        name: convert_component_name(app_name.clone()),
        parent_address: AndrAddr::from_string(sender),
    };
    let cosmos_msg: CosmosMsg<Empty> = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: vfs_address.to_string(),
        msg: to_binary(&add_path_msg)?,
        funds: vec![],
    });

    let register_msg = SubMsg::reply_on_error(cosmos_msg, ReplyId::RegisterPath.repr());
    let assign_app_msg = ExecuteMsg::AssignAppToComponents {};
    let assign_app_msg = SubMsg::reply_on_error(
        CosmosMsg::Wasm::<Empty>(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_binary(&assign_app_msg)?,
            funds: vec![],
        }),
        ReplyId::AssignApp.repr(),
    );
    resp = resp
        .add_submessage(register_msg)
        .add_submessages(msgs)
        .add_submessage(assign_app_msg);

    if let Some(chain_info) = msg.chain_info {
        for chain in chain_info.clone() {
            let sub_msg = create_cross_chain_message(
                &deps,
                app_name.clone(),
                msg.owner.clone().unwrap_or(info.sender.to_string()),
                msg.app_components.clone(),
                chain,
                chain_info.clone(),
            )?;
            resp = resp.add_submessage(sub_msg);
        }
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

    match ReplyId::from_repr(msg.id) {
        Some(ReplyId::RegisterPath) => Ok(Response::default()),
        Some(ReplyId::ClaimOwnership) => Ok(Response::default()),
        Some(ReplyId::AssignApp) => Ok(Response::default()),
        _ => on_component_instantiation(deps, msg),
    }
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
        ExecuteMsg::ClaimOwnership { name, new_owner } => {
            execute::claim_ownership(ctx, name, new_owner)
        }
        ExecuteMsg::ProxyMessage { msg, name } => execute::message(ctx, name, msg),
        ExecuteMsg::UpdateAddress { name, addr } => execute::update_address(ctx, name, addr),
        ExecuteMsg::AssignAppToComponents {} => execute::assign_app_to_components(ctx),
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
