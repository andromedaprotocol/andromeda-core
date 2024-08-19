#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, ensure, Addr, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    Response, StdError, Storage, SubMsg, Uint128,
};

use andromeda_data_storage::matrix::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_data_storage::matrix::{GetMatrixResponse, Matrix, MatrixRestriction};
use andromeda_std::{
    ado_base::{
        rates::{Rate, RatesMessage},
        InstantiateMsg as BaseInstantiateMsg, MigrateMsg,
    },
    ado_contract::ADOContract,
    amp::AndrAddr,
    common::{
        actions::call_action, context::ExecuteContext, encode_binary, rates::get_tax_amount, Funds,
    },
    error::ContractError,
};

use cw_utils::nonpayable;

use crate::state::{DEFAULT_KEY, KEY_OWNER, MATRIX, RESTRICTION};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-matrix";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

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
        info,
        BaseInstantiateMsg {
            ado_type: CONTRACT_NAME.to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            kernel_address: msg.kernel_address,
            owner: msg.owner,
        },
    )?;
    RESTRICTION.save(deps.storage, &msg.restriction)?;
    Ok(resp)
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::GetMatrix { key } => encode_binary(&get_matrix(deps.storage, key)?),
        QueryMsg::AllKeys {} => encode_binary(&all_keys(deps.storage)?),
        QueryMsg::OwnerKeys { owner } => encode_binary(&owner_keys(&deps, owner)?),
        _ => ADOContract::default().query(deps, env, msg),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    ADOContract::default().migrate(deps, CONTRACT_NAME, CONTRACT_VERSION)
}

pub fn handle_execute(mut ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    let action = msg.as_ref().to_string();
    call_action(
        &mut ctx.deps,
        &ctx.info,
        &ctx.env,
        &ctx.amp_ctx,
        msg.as_ref(),
    )?;

    match msg.clone() {
        ExecuteMsg::UpdateRestriction { restriction } => update_restriction(ctx, restriction),
        ExecuteMsg::StoreMatrix { key, data } => store_matrix(ctx, key, data, action),
        ExecuteMsg::DeleteMatrix { key } => delete_matrix(ctx, key),
        ExecuteMsg::Rates(rates_message) => match rates_message {
            RatesMessage::SetRate { rate, .. } => match rate {
                Rate::Local(local_rate) => {
                    // Percent rates aren't applicable in this case, so we enforce Flat rates
                    ensure!(local_rate.value.is_flat(), ContractError::InvalidRate {});
                    ADOContract::default().execute(ctx, msg)
                }
                Rate::Contract(_) => ADOContract::default().execute(ctx, msg),
            },
            RatesMessage::RemoveRate { .. } => ADOContract::default().execute(ctx, msg),
        },
        _ => ADOContract::default().execute(ctx, msg),
    }
}

/// ============================== Execution Functions ============================== ///
pub fn update_restriction(
    ctx: ExecuteContext,
    restriction: MatrixRestriction,
) -> Result<Response, ContractError> {
    nonpayable(&ctx.info)?;
    let sender = ctx.info.sender;
    ensure!(
        ADOContract::default().is_owner_or_operator(ctx.deps.storage, sender.as_ref())?,
        ContractError::Unauthorized {}
    );
    RESTRICTION.save(ctx.deps.storage, &restriction)?;
    Ok(Response::new()
        .add_attribute("method", "update_restriction")
        .add_attribute("sender", sender))
}

pub fn store_matrix(
    ctx: ExecuteContext,
    key: Option<String>,
    data: Matrix,
    action: String,
) -> Result<Response, ContractError> {
    let sender = ctx.info.sender.clone();
    let key: &str = get_key_or_default(&key);
    ensure!(
        has_key_permission(ctx.deps.storage, &sender, key)?,
        ContractError::Unauthorized {}
    );
    // Validate the data
    data.validate_matrix()?;

    let tax_response = tax_store_matrix(ctx.deps.as_ref(), &ctx.info, action)?;

    MATRIX.update::<_, StdError>(ctx.deps.storage, key, |old| match old {
        Some(_) => Ok(data.clone()),
        None => Ok(data.clone()),
    })?;
    // Update the owner of the key
    KEY_OWNER.update::<_, StdError>(ctx.deps.storage, key, |old| match old {
        Some(old) => Ok(old),
        None => Ok(sender.clone()),
    })?;

    let mut response = Response::new()
        .add_attribute("method", "store_matrix")
        .add_attribute("sender", sender)
        .add_attribute("key", key)
        .add_attribute("data", format!("{data:?}"));

    if let Some(tax_response) = tax_response {
        response = response.add_submessages(tax_response.1);
        let refund = tax_response.0.try_get_coin()?;
        if !refund.amount.is_zero() {
            return Ok(response.add_message(CosmosMsg::Bank(BankMsg::Send {
                to_address: ctx.info.sender.into_string(),
                amount: vec![refund],
            })));
        }
    }

    Ok(response)
}

pub fn delete_matrix(ctx: ExecuteContext, key: Option<String>) -> Result<Response, ContractError> {
    nonpayable(&ctx.info)?;
    let sender = ctx.info.sender;

    let key = get_key_or_default(&key);
    ensure!(
        has_key_permission(ctx.deps.storage, &sender, key)?,
        ContractError::Unauthorized {}
    );
    MATRIX.remove(ctx.deps.storage, key);
    KEY_OWNER.remove(ctx.deps.storage, key);
    Ok(Response::new()
        .add_attribute("method", "delete_value")
        .add_attribute("sender", sender)
        .add_attribute("key", key))
}

/// ============================== Query Functions ============================== ///
pub fn get_matrix(
    storage: &dyn Storage,
    key: Option<String>,
) -> Result<GetMatrixResponse, ContractError> {
    let key = get_key_or_default(&key);
    let data = MATRIX.load(storage, key)?;
    Ok(GetMatrixResponse {
        key: key.to_string(),
        data,
    })
}

pub fn all_keys(storage: &dyn Storage) -> Result<Vec<String>, ContractError> {
    let keys = MATRIX
        .keys(storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|key| key.unwrap())
        .collect();
    Ok(keys)
}

pub fn owner_keys(deps: &Deps, owner: AndrAddr) -> Result<Vec<String>, ContractError> {
    let owner = owner.get_raw_address(deps)?;
    let keys = KEY_OWNER
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .filter(|x| x.as_ref().unwrap().1 == owner)
        .map(|key| key.unwrap().0)
        .collect();
    Ok(keys)
}

pub fn get_key_or_default(name: &Option<String>) -> &str {
    match name {
        None => DEFAULT_KEY,
        Some(s) => s,
    }
}

/// ==============================  ============================== ///
pub fn has_key_permission(
    storage: &dyn Storage,
    addr: &Addr,
    key: &str,
) -> Result<bool, ContractError> {
    let is_operator = ADOContract::default().is_owner_or_operator(storage, addr.as_str())?;
    let allowed = match RESTRICTION.load(storage)? {
        MatrixRestriction::Private => is_operator,
        MatrixRestriction::Public => true,
        MatrixRestriction::Restricted => match KEY_OWNER.load(storage, key).ok() {
            Some(owner) => addr == owner,
            None => true,
        },
    };
    Ok(is_operator || allowed)
}

fn tax_store_matrix(
    deps: Deps,
    info: &MessageInfo,
    action: String,
) -> Result<Option<(Funds, Vec<SubMsg>)>, ContractError> {
    let default_coin = coin(0_u128, "uandr".to_string());
    let sent_funds = info.funds.first().unwrap_or(&default_coin);

    let transfer_response = ADOContract::default().query_deducted_funds(
        deps,
        action,
        Funds::Native(sent_funds.clone()),
    )?;

    if let Some(transfer_response) = transfer_response {
        let remaining_funds = transfer_response.leftover_funds.try_get_coin()?;
        let tax_amount = get_tax_amount(
            &transfer_response.msgs,
            remaining_funds.amount,
            remaining_funds.amount,
        );

        let refund = if sent_funds.amount > tax_amount {
            sent_funds.amount.checked_sub(tax_amount)?
        } else {
            Uint128::zero()
        };

        let after_tax_payment = Coin {
            denom: remaining_funds.denom,
            amount: refund,
        };
        Ok(Some((
            Funds::Native(after_tax_payment),
            transfer_response.msgs,
        )))
    } else {
        Ok(None)
    }
}
