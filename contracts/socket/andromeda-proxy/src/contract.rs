use crate::state::{
    authorize, get_reply_id, ADMINS, BATCH_REPLY_ID_FAIL_ON_ERROR, BATCH_REPLY_ID_IGNORE_ERROR,
    REPLY_ID,
};
use andromeda_socket::proxy::{
    ExecuteMsg, ExecutionType, InitParams, InstantiateMsg, Operation, QueryMsg,
};
use andromeda_std::{
    ado_base::{InstantiateMsg as BaseInstantiateMsg, MigrateMsg},
    ado_contract::ADOContract,
    amp::AndrAddr,
    andr_execute_fn,
    common::context::ExecuteContext,
    error::ContractError,
    os::aos_querier::AOSQuerier,
};
use cosmwasm_std::{
    attr, entry_point, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply, Response,
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
        } => send_instantiate(&ctx, init_params, message, admin, label, None),
        ExecuteMsg::Execute {
            contract_addr,
            message,
        } => send_execute(&ctx, contract_addr, message, None),
        ExecuteMsg::BatchExecute { operations } => send_batch_execute(ctx, operations),
        ExecuteMsg::ModifyAdmins { admins } => execute_modify_admins(ctx, admins),
        _ => ADOContract::default().execute(ctx, msg),
    }
}

// Used for cross-chain creation of denom
fn send_execute(
    ctx: &ExecuteContext,
    contract_addr: AndrAddr,
    message: Binary,
    fail_on_error: Option<bool>,
) -> Result<Response, ContractError> {
    authorize(ctx)?;

    let reply_id = get_reply_id(fail_on_error);
    // Forward the message
    let sub_msg = SubMsg::reply_on_error(
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: contract_addr
                .get_raw_address(&ctx.deps.as_ref())?
                .into_string(),
            msg: message,
            funds: ctx.info.funds.clone(),
        }),
        reply_id,
    );

    Ok(Response::default()
        .add_submessage(sub_msg)
        .add_attribute("action", "send_execute"))
}

fn send_batch_execute(
    ctx: ExecuteContext,
    operations: Vec<Operation>,
) -> Result<Response, ContractError> {
    authorize(&ctx)?;

    let mut resp = Response::default();

    for op in operations {
        let result = match op.execution {
            ExecutionType::Instantiate {
                init_params,
                message,
                admin,
                label,
            } => send_instantiate(
                &ctx,
                init_params,
                message,
                admin,
                label,
                Some(op.fail_on_error),
            ),
            ExecutionType::Execute {
                contract_addr,
                message,
            } => send_execute(&ctx, contract_addr, message, Some(op.fail_on_error)),
            ExecutionType::Migrate {
                contract_addr,
                new_code_id,
                migrate_msg,
            } => send_migrate(
                &ctx,
                contract_addr,
                new_code_id,
                migrate_msg,
                Some(op.fail_on_error),
            ),
        };

        match result {
            Ok(new_res) => {
                resp = resp
                    .add_submessages(new_res.messages)
                    .add_attributes(new_res.attributes);
            }
            Err(e) if op.fail_on_error => return Err(e),
            Err(_) => {} // ignore error, continue loop
        }
    }

    Ok(resp)
}

fn send_instantiate(
    ctx: &ExecuteContext,
    init_params: InitParams,
    message: Binary,
    admin: Option<AndrAddr>,
    label: Option<String>,
    fail_on_error: Option<bool>,
) -> Result<Response, ContractError> {
    authorize(ctx)?;

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

    let admin = match admin {
        Some(admin) => Some(admin.get_raw_address(&ctx.deps.as_ref())?.into_string()),
        None => None,
    };
    let reply_id = get_reply_id(fail_on_error);
    // Forward the message
    let sub_msg = SubMsg::reply_on_error(
        CosmosMsg::Wasm(WasmMsg::Instantiate {
            admin,
            code_id,
            msg: message,
            funds: ctx.info.funds.clone(),
            label: label.unwrap_or("default".to_string()),
        }),
        reply_id,
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

fn send_migrate(
    ctx: &ExecuteContext,
    contract_addr: AndrAddr,
    new_code_id: u64,
    migrate_msg: Binary,
    fail_on_error: Option<bool>,
) -> Result<Response, ContractError> {
    authorize(ctx)?;
    let reply_id = get_reply_id(fail_on_error);
    let sub_msg = SubMsg::reply_on_error(
        CosmosMsg::Wasm(WasmMsg::Migrate {
            contract_addr: contract_addr
                .get_raw_address(&ctx.deps.as_ref())?
                .into_string(),
            new_code_id,
            msg: migrate_msg,
        }),
        reply_id,
    );

    Ok(Response::default()
        .add_submessage(sub_msg)
        .add_attribute("action", "send_migrate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    ADOContract::default().query(deps, env, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    ADOContract::default().migrate(deps, env, CONTRACT_NAME, CONTRACT_VERSION)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        REPLY_ID => {
            if msg.result.is_err() {
                Err(ContractError::Std(StdError::generic_err(format!(
                    "Contract error: {:?}",
                    msg.result.unwrap_err()
                ))))
            } else {
                Ok(
                    Response::default()
                        .add_attributes(vec![attr("action", "message_sent_success")]),
                )
            }
        }
        BATCH_REPLY_ID_FAIL_ON_ERROR => Err(ContractError::Std(StdError::generic_err(format!(
            "Contract error: {:?}",
            msg.result.unwrap_err()
        )))),
        BATCH_REPLY_ID_IGNORE_ERROR => Ok(Response::default()),
        _ => Err(ContractError::Std(StdError::generic_err(
            "Invalid Reply ID".to_string(),
        ))),
    }
}
