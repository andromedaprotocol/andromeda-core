use andromeda_modules::address_list::{
    ActorPermissionResponse, IncludesActorResponse, PERMISSION_ACTORS_ACTION,
};
#[cfg(not(feature = "library"))]
use andromeda_modules::address_list::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{
    ado_base::{
        permissioning::{LocalPermission, Permission},
        InstantiateMsg as BaseInstantiateMsg, MigrateMsg,
    },
    ado_contract::ADOContract,
    amp::AndrAddr,
    andr_execute_fn,
    common::{context::ExecuteContext, encode_binary},
    error::ContractError,
};

use cosmwasm_std::{
    attr, ensure, entry_point, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdError,
};

use crate::state::{add_actors_permission, includes_actor, PERMISSIONS};
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
    if let Some(mut actor_permission) = msg.actor_permission {
        ensure!(
            !actor_permission.actors.is_empty(),
            ContractError::NoActorsProvided {}
        );
        actor_permission.permission.validate_times(&env)?;

        // If the permission is a whitelist, make sure to set last used time as none
        actor_permission.permission = if let LocalPermission::Whitelisted {
            start,
            expiration,
            frequency,
            ..
        } = actor_permission.permission
        {
            LocalPermission::Whitelisted {
                start,
                expiration,
                frequency,
                last_used: None,
            }
        } else {
            actor_permission.permission
        };

        for actor in actor_permission.actors {
            let verified_actor = actor.get_raw_address(&deps.as_ref())?;
            add_actors_permission(deps.storage, verified_actor, &actor_permission.permission)?;
        }
    }

    ADOContract::default().permission_action(deps.storage, PERMISSION_ACTORS_ACTION)?;
    ADOContract::set_permission(
        deps.storage,
        PERMISSION_ACTORS_ACTION.to_string(),
        info.sender.clone(),
        Permission::Local(LocalPermission::whitelisted(None, None, None, None)),
    )?;

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

#[andr_execute_fn]
pub fn execute(ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::PermissionActors { actors, permission } => {
            execute_permission_actors(ctx, actors, permission)
        }
        ExecuteMsg::RemovePermissions { actors } => execute_remove_permissions(ctx, actors),
        _ => ADOContract::default().execute(ctx, msg),
    }
}

fn execute_permission_actors(
    ctx: ExecuteContext,
    actors: Vec<AndrAddr>,
    permission: LocalPermission,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, env, .. } = ctx;

    ensure!(!actors.is_empty(), ContractError::NoActorsProvided {});
    permission.validate_times(&env)?;
    for actor in actors.clone() {
        let verified_actor = actor.get_raw_address(&deps.as_ref())?;
        add_actors_permission(deps.storage, verified_actor, &permission)?;
    }
    let actors_str = actors
        .iter()
        .map(|actor| actor.to_string())
        .collect::<Vec<String>>()
        .join(", ");
    Ok(Response::new().add_attributes(vec![
        attr("action", "add_actor_permission"),
        attr("actor", actors_str),
        attr("permission", permission.to_string()),
    ]))
}

fn execute_remove_permissions(
    ctx: ExecuteContext,
    actors: Vec<AndrAddr>,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, .. } = ctx;
    ensure!(!actors.is_empty(), ContractError::NoActorsProvided {});

    for actor in actors.clone() {
        let verified_actor = actor.get_raw_address(&deps.as_ref())?;
        // Ensure that the actor is present in the permissions list
        ensure!(
            PERMISSIONS.has(deps.storage, &verified_actor),
            ContractError::ActorNotFound {}
        );
        PERMISSIONS.remove(deps.storage, &verified_actor);
    }
    let actors_str = actors
        .iter()
        .map(|actor| actor.to_string())
        .collect::<Vec<String>>()
        .join(", ");

    Ok(Response::new().add_attributes(vec![
        attr("action", "remove_actor_permission"),
        attr("actor", actors_str),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    ADOContract::default().migrate(deps, env, CONTRACT_NAME, CONTRACT_VERSION)
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.result.is_err() {
        return Err(ContractError::Std(StdError::generic_err(
            msg.result.unwrap_err(),
        )));
    }

    Ok(Response::default())
}
