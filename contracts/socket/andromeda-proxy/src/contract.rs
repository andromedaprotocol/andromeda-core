use crate::state::{authorize, ADMINS, LOCKED};
use andromeda_socket::proxy::{
    AllLockedResponse, ExecuteMsg, InitParams, InstantiateMsg, LockedInfo, LockedResponse, QueryMsg,
};
use andromeda_std::{
    ado_base::{InstantiateMsg as BaseInstantiateMsg, MigrateMsg},
    ado_contract::ADOContract,
    amp::AndrAddr,
    andr_execute_fn,
    common::{context::ExecuteContext, encode_binary},
    error::ContractError,
    os::aos_querier::AOSQuerier,
};
use cosmwasm_std::{
    attr, entry_point, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdError, SubMsg, WasmMsg,
};
use cw2::set_contract_version;

const CONTRACT_NAME: &str = "crates.io:andromeda-proxy";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let inst_resp = ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        &deps.querier,
        info.clone(),
        BaseInstantiateMsg {
            ado_type: CONTRACT_NAME.to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            kernel_address: msg.kernel_address.clone(),
            owner: msg.owner,
        },
    )?;

    ADMINS.save(deps.storage, &msg.admins)?;

    Ok(inst_resp
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[andr_execute_fn]
pub fn execute(ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Instantiate {
            init_params,
            message,
            admin,
            label,
        } => send_instantiate(ctx, init_params, message, admin, label),
        ExecuteMsg::Execute {
            contract_addr,
            message,
        } => send_execute(ctx, contract_addr, message),
        ExecuteMsg::ModifyAdmins { admins } => execute_modify_admins(ctx, admins),
        _ => ADOContract::default().execute(ctx, msg),
    }
}

// Used for cross-chain creation of denom
fn send_execute(
    ctx: ExecuteContext,
    contract_addr: AndrAddr,
    message: Binary,
) -> Result<Response, ContractError> {
    authorize(&ctx)?;

    // Forward the message
    let sub_msg = SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract_addr
            .get_raw_address(&ctx.deps.as_ref())?
            .into_string(),
        msg: message,
        funds: ctx.info.funds,
    }));

    Ok(Response::default()
        .add_submessage(sub_msg)
        .add_attribute("action", "send_execute"))
}

fn send_instantiate(
    ctx: ExecuteContext,
    init_params: InitParams,
    message: Binary,
    admin: Option<String>,
    label: Option<String>,
) -> Result<Response, ContractError> {
    authorize(&ctx)?;

    let code_id = match init_params {
        InitParams::CodeId(code_id) => code_id,
        InitParams::AdoVersion(ado_version) => {
            let adodb_addr = AOSQuerier::adodb_address_getter(
                &ctx.deps.querier,
                &ctx.contract.get_kernel_address(ctx.deps.storage)?,
            )?;
            AOSQuerier::code_id_getter(&ctx.deps.querier, &adodb_addr, ado_version.as_str())?
        }
    };

    // Forward the message
    let sub_msg = SubMsg::reply_always(
        CosmosMsg::Wasm(WasmMsg::Instantiate {
            admin,
            code_id,
            msg: message,
            funds: ctx.info.funds,
            label: label.unwrap_or("default".to_string()),
        }),
        1,
    );

    Ok(Response::default()
        .add_attribute("action", "send_instantiate")
        .add_submessage(sub_msg))
}

fn execute_modify_admins(
    ctx: ExecuteContext,
    admins: Vec<String>,
) -> Result<Response, ContractError> {
    authorize(&ctx)?;

    ADMINS.save(ctx.deps.storage, &admins)?;

    Ok(Response::default().add_attribute("action", "modify_admins"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::Locked { cw20_addr } => encode_binary(&query_locked(deps, cw20_addr)?),
        QueryMsg::AllLocked {} => encode_binary(&query_all_locked(deps)?),
        _ => ADOContract::default().query(deps, env, msg),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    ADOContract::default().migrate(deps, env, CONTRACT_NAME, CONTRACT_VERSION)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        1 => {
            if msg.result.is_err() {
                Err(ContractError::Std(StdError::generic_err(format!(
                    "Contract instantiation error: {:?}",
                    msg.result.unwrap_err()
                ))))
            } else {
                Ok(Response::default()
                    .add_attributes(vec![attr("action", "contract_instantiated")]))
            }
        }
        _ => Err(ContractError::Std(StdError::generic_err(
            "Invalid Reply ID".to_string(),
        ))),
    }
}

fn query_locked(deps: Deps, cw20_addr: Addr) -> Result<LockedResponse, ContractError> {
    let amount = LOCKED
        .may_load(deps.storage, cw20_addr)?
        .unwrap_or_default();
    Ok(LockedResponse { amount })
}

fn query_all_locked(deps: Deps) -> Result<AllLockedResponse, ContractError> {
    let locked: Vec<LockedInfo> = LOCKED
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .filter_map(|item| {
            let (cw20_addr, amount) = item.ok()?;
            if !amount.is_zero() {
                Some(LockedInfo { cw20_addr, amount })
            } else {
                None
            }
        })
        .collect();
    Ok(AllLockedResponse { locked })
}
