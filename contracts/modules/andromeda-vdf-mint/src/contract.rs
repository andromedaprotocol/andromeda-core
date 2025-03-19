use andromeda_modules::vdf_mint::{
    ExecuteMsg, GetActorsResponse, GetLastMintTimestampSecondsResponse,
    GetMintCooldownMinutesResponse, InstantiateMsg, QueryMsg,
};
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
    os::aos_querier::AOSQuerier,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    ensure, entry_point, to_json_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    Reply, Response, StdError, Storage, Uint64, WasmMsg,
};

use crate::state::{ACTORS, CW721_ADDRESS, LAST_MINT_TIMESTAMP, MINT_COOLDOWN_MINUTES};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-vdf-mint";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const VDF_MINT_ACTION: &str = "vdf_mint";
const DEFAULT_MINT_COOLDOWN_MINUTES: u64 = 5_u64;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let resp = ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        &deps.querier,
        info.clone(),
        BaseInstantiateMsg {
            ado_type: CONTRACT_NAME.to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            kernel_address: msg.kernel_address,
            owner: msg.owner,
        },
    )?;

    let cw721_address = msg.cw721_address;
    let raw_cw721_address = cw721_address.get_raw_address(&deps.as_ref())?;

    // Verify the address is a contract and has the correct ADO type
    let contract_info = deps
        .querier
        .query_wasm_contract_info(raw_cw721_address.clone())
        .map_err(|_| ContractError::InvalidAddress {})?;

    let adodb_addr = ADOContract::default().get_adodb_address(deps.storage, &deps.querier)?;
    let ado_type = AOSQuerier::ado_type_getter(&deps.querier, &adodb_addr, contract_info.code_id)?;

    // Ensure the contract is a CW721 type
    ensure!(
        matches!(ado_type.clone(), Some(type_str) if type_str == "cw721"),
        ContractError::InvalidADOType {
            msg: Some(format!("ADO Type must be cw721, got: {:?}", ado_type))
        }
    );

    CW721_ADDRESS.save(deps.storage, &cw721_address)?;

    ADOContract::default().permission_action(deps.storage, VDF_MINT_ACTION)?;

    let mut actors_addr: Vec<Addr> = Vec::new();

    if let Some(actors) = msg.actors {
        for actor in actors {
            let actor_raw_addr = actor.get_raw_address(&deps.as_ref())?;
            ADOContract::set_permission(
                deps.storage,
                VDF_MINT_ACTION,
                actor_raw_addr.clone(),
                Permission::Local(LocalPermission::whitelisted(None, None)),
            )?;
            actors_addr.push(actor_raw_addr);
        }
    } else {
        // Default to sender as actor if none provided
        actors_addr.push(info.sender.clone());
        ADOContract::set_permission(
            deps.storage,
            VDF_MINT_ACTION,
            info.sender,
            Permission::Local(LocalPermission::whitelisted(None, None)),
        )?;
    }
    ACTORS.save(deps.storage, &actors_addr)?;

    // Set mint cooldown - use provided value or default
    let cooldown = msg
        .mint_cooldown_minutes
        .unwrap_or(Uint64::new(DEFAULT_MINT_COOLDOWN_MINUTES));

    // Ensure cooldown is not less than minimum
    ensure!(
        cooldown.ge(&Uint64::new(DEFAULT_MINT_COOLDOWN_MINUTES)),
        ContractError::CustomError {
            msg: format!(
                "Mint cooldown should not be less than {:?}",
                DEFAULT_MINT_COOLDOWN_MINUTES
            )
        }
    );

    MINT_COOLDOWN_MINUTES.save(deps.storage, &cooldown)?;

    Ok(resp)
}

#[andr_execute_fn]
pub fn execute(ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AddActors { actors } => execute_add_actors(ctx, actors),
        ExecuteMsg::RemoveActors { actors } => execute_remove_actors(ctx, actors),
        ExecuteMsg::VdfMint { token_id, owner } => execute_vdf_mint(ctx, token_id, owner),
        _ => ADOContract::default().execute(ctx, msg),
    }
}

pub fn execute_add_actors(
    ctx: ExecuteContext,
    new_actors: Vec<AndrAddr>,
) -> Result<Response, ContractError> {
    let mut origin_actors = ACTORS
        .load(ctx.deps.storage)
        .map_err(|_| ContractError::ActorNotFound {})?;

    for actor in new_actors {
        let actor_raw_addr = actor.get_raw_address(&ctx.deps.as_ref())?;
        if origin_actors.contains(&actor_raw_addr) {
            return Err(ContractError::CustomError {
                msg: format!("Already existed actor: {:?}", actor_raw_addr.clone()),
            });
        }
        origin_actors.push(actor_raw_addr.clone());

        ADOContract::set_permission(
            ctx.deps.storage,
            VDF_MINT_ACTION,
            actor_raw_addr.clone(),
            Permission::Local(LocalPermission::whitelisted(None, None)),
        )?;
    }

    ACTORS.save(ctx.deps.storage, &origin_actors)?;

    Ok(Response::new().add_attribute("method", "add_actors"))
}

pub fn execute_remove_actors(
    ctx: ExecuteContext,
    new_actors: Vec<AndrAddr>,
) -> Result<Response, ContractError> {
    let mut origin_actors = ACTORS
        .load(ctx.deps.storage)
        .map_err(|_| ContractError::ActorNotFound {})?;

    for actor in new_actors {
        let actor_raw_addr = actor.get_raw_address(&ctx.deps.as_ref())?;
        if !origin_actors.contains(&actor_raw_addr) {
            return Err(ContractError::CustomError {
                msg: format!("Actor not found: {:?}", actor_raw_addr.clone()),
            });
        }

        // Remove actor from the list
        origin_actors.retain(|a| a != actor_raw_addr);

        // Remove permission for the actor
        ADOContract::remove_permission(ctx.deps.storage, VDF_MINT_ACTION, actor_raw_addr.clone())?;
    }

    // Save the updated actor list
    ACTORS.save(ctx.deps.storage, &origin_actors)?;

    Ok(Response::new().add_attribute("method", "remove_actors"))
}

pub fn execute_vdf_mint(
    ctx: ExecuteContext,
    token_id: String,
    owner: AndrAddr,
) -> Result<Response, ContractError> {
    let cw721_address = CW721_ADDRESS.load(ctx.deps.storage)?;
    let mint_cooldown_minutes = MINT_COOLDOWN_MINUTES.load(ctx.deps.storage)?;

    let current_time_sec = ctx.env.block.time.seconds();

    if let Some(last_mint) = LAST_MINT_TIMESTAMP.may_load(ctx.deps.storage)? {
        if current_time_sec < last_mint.u64() + mint_cooldown_minutes.u64() * 60_u64 {
            return Err(ContractError::CustomError {
                msg: "Mint cooldown active".to_string(),
            });
        }
    }

    let mint_msg: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cw721_address
            .get_raw_address(&ctx.deps.as_ref())?
            .to_string(),
        msg: to_json_binary(&andromeda_non_fungible_tokens::cw721::ExecuteMsg::Mint {
            token_id,
            owner: owner.get_raw_address(&ctx.deps.as_ref())?.to_string(),
            token_uri: None, // Add a URI if needed
            extension: andromeda_non_fungible_tokens::cw721::TokenExtension {
                publisher: "ado_publisher".to_string(),
            },
        })?,
        funds: vec![],
    });

    LAST_MINT_TIMESTAMP.save(ctx.deps.storage, &Uint64::new(current_time_sec))?;

    Ok(Response::new()
        .add_message(mint_msg)
        .add_attribute("method", "vdf_mint"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::GetActors {} => encode_binary(&get_actors(deps.storage)?),
        QueryMsg::GetLastMintTimestampSeconds {} => {
            encode_binary(&get_last_mint_timestamp_seconds(deps.storage)?)
        }
        QueryMsg::GetMintCooldownMinutes {} => {
            encode_binary(&get_mint_cooldown_minutes(deps.storage)?)
        }
        _ => ADOContract::default().query(deps, env, msg),
    }
}

pub fn get_actors(storage: &dyn Storage) -> Result<GetActorsResponse, ContractError> {
    let actors = ACTORS
        .load(storage)
        .map_err(|_| ContractError::CustomError {
            msg: "Not existed".to_string(),
        });
    match actors {
        Ok(actors) => Ok(GetActorsResponse { actors }),
        Err(err) => Err(err),
    }
}

pub fn get_last_mint_timestamp_seconds(
    storage: &dyn Storage,
) -> Result<GetLastMintTimestampSecondsResponse, ContractError> {
    let last_mint_timestamp_seconds =
        LAST_MINT_TIMESTAMP
            .load(storage)
            .map_err(|_| ContractError::CustomError {
                msg: "Not existed".to_string(),
            });
    match last_mint_timestamp_seconds {
        Ok(last_mint_timestamp_seconds) => Ok(GetLastMintTimestampSecondsResponse {
            last_mint_timestamp_seconds,
        }),
        Err(err) => Err(err),
    }
}

pub fn get_mint_cooldown_minutes(
    storage: &dyn Storage,
) -> Result<GetMintCooldownMinutesResponse, ContractError> {
    let mint_cooldown_minutes =
        MINT_COOLDOWN_MINUTES
            .load(storage)
            .map_err(|_| ContractError::CustomError {
                msg: "Not existed".to_string(),
            });
    match mint_cooldown_minutes {
        Ok(mint_cooldown_minutes) => Ok(GetMintCooldownMinutesResponse {
            mint_cooldown_minutes,
        }),
        Err(err) => Err(err),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    ADOContract::default().migrate(deps, env, CONTRACT_NAME, CONTRACT_VERSION)
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
