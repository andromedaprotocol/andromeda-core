use andromeda_modules::address_list::{ActorPermissionResponse, IncludesActorResponse};
#[cfg(not(feature = "library"))]
use andromeda_modules::address_list::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{
    ado_base::{permissioning::LocalPermission, InstantiateMsg as BaseInstantiateMsg, MigrateMsg},
    ado_contract::ADOContract,
    common::{context::ExecuteContext, encode_binary},
    error::ContractError,
};

use cosmwasm_std::{
    attr, ensure, entry_point, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response,
};
use cw_utils::nonpayable;

use crate::state::{add_actor_permission, includes_actor, PERMISSIONS};
// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-address-list";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // If the user provided an actor and permission, save them.
    if let Some(actor_permission) = msg.actor_permission {
        let verified_address: Addr = deps.api.addr_validate(actor_permission.actor.as_str())?;
        // Permissions of type "Contract" aren't allowed in the address list contract
        if let LocalPermission::Limited { .. } = actor_permission.permission {
            return Err(ContractError::InvalidPermission {
                msg: "Limited permission is not supported in address list contract".to_string(),
            });
        }
        add_actor_permission(
            deps.storage,
            &verified_address,
            &actor_permission.permission,
        )?;
    }
    let inst_resp = ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        &deps.querier,
        info,
        BaseInstantiateMsg {
            ado_type: CONTRACT_NAME.to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            kernel_address: msg.kernel_address,
            owner: msg.owner,
        },
    )?;

    Ok(inst_resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let _contract = ADOContract::default();
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
        ExecuteMsg::AddActorPermission { actor, permission } => {
            execute_add_actor_permission(ctx, actor, permission)
        }
        ExecuteMsg::RemoveActorPermission { actor } => execute_remove_actor_permission(ctx, actor),
        _ => ADOContract::default().execute(ctx, msg),
    }
}

fn execute_add_actor_permission(
    ctx: ExecuteContext,
    actor: Addr,
    permission: LocalPermission,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, info, .. } = ctx;
    nonpayable(&info)?;
    ensure!(
        ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );
    if let LocalPermission::Limited { .. } = permission {
        return Err(ContractError::InvalidPermission {
            msg: "Limited permission is not supported in address list contract".to_string(),
        });
    }
    add_actor_permission(deps.storage, &actor, &permission)?;
    Ok(Response::new().add_attributes(vec![
        attr("action", "add_actor_permission"),
        attr("actor", actor),
        attr("permission", permission.to_string()),
    ]))
}

fn execute_remove_actor_permission(
    ctx: ExecuteContext,
    actor: Addr,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, info, .. } = ctx;
    nonpayable(&info)?;
    ensure!(
        ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );
    // Ensure that the actor is present in the permissions list
    ensure!(
        PERMISSIONS.has(deps.storage, &actor),
        ContractError::ActorNotFound {}
    );

    PERMISSIONS.remove(deps.storage, &actor);

    Ok(Response::new().add_attributes(vec![
        attr("action", "remove_actor_permission"),
        attr("actor", actor),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    ADOContract::default().migrate(deps, CONTRACT_NAME, CONTRACT_VERSION)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::IncludesActor { actor } => encode_binary(&query_actor(deps, actor)?),
        QueryMsg::ActorPermission { actor } => encode_binary(&query_actor_permission(deps, actor)?),
        _ => ADOContract::default().query(deps, env, msg),
    }
}

fn query_actor(deps: Deps, actor: Addr) -> Result<IncludesActorResponse, ContractError> {
    Ok(IncludesActorResponse {
        included: includes_actor(deps.storage, &actor)?,
    })
}

fn query_actor_permission(
    deps: Deps,
    actor: Addr,
) -> Result<ActorPermissionResponse, ContractError> {
    let permission = PERMISSIONS.may_load(deps.storage, &actor)?;
    if let Some(permission) = permission {
        Ok(ActorPermissionResponse { permission })
    } else {
        Err(ContractError::ActorNotFound {})
    }
}
