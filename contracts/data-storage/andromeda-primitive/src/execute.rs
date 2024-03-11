use andromeda_data_storage::primitive::{ExecuteMsg, Primitive, PrimitiveRestriction};
use andromeda_std::common::call_action::call_action;
use andromeda_std::{
    ado_contract::ADOContract, common::context::ExecuteContext, error::ContractError,
};
use cosmwasm_std::{ensure, Response, StdError};
use cw_utils::nonpayable;

use crate::{
    query::{get_key_or_default, has_key_permission},
    state::{DATA, KEY_OWNER, RESTRICTION},
};

pub fn handle_execute(mut ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    call_action(
        &mut ctx.deps,
        &ctx.info,
        &ctx.env,
        &ctx.amp_ctx,
        msg.as_ref(),
    )?;
    match msg {
        ExecuteMsg::UpdateRestriction { restriction } => update_restriction(ctx, restriction),
        ExecuteMsg::SetValue { key, value } => set_value(ctx, key, value),
        ExecuteMsg::DeleteValue { key } => delete_value(ctx, key),
        _ => ADOContract::default().execute(ctx, msg),
    }
}

pub fn update_restriction(
    ctx: ExecuteContext,
    restriction: PrimitiveRestriction,
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

pub fn set_value(
    ctx: ExecuteContext,
    key: Option<String>,
    value: Primitive,
) -> Result<Response, ContractError> {
    nonpayable(&ctx.info)?;
    let sender = ctx.info.sender;
    let key: &str = get_key_or_default(&key);
    ensure!(
        has_key_permission(ctx.deps.storage, &sender, key)?,
        ContractError::Unauthorized {}
    );
    DATA.update::<_, StdError>(ctx.deps.storage, key, |old| match old {
        Some(_) => Ok(value.clone()),
        None => Ok(value.clone()),
    })?;
    // Update the owner of the key
    KEY_OWNER.update::<_, StdError>(ctx.deps.storage, key, |old| match old {
        Some(old) => Ok(old),
        None => Ok(sender.clone()),
    })?;

    Ok(Response::new()
        .add_attribute("method", "set_value")
        .add_attribute("sender", sender)
        .add_attribute("key", key)
        .add_attribute("value", format!("{value:?}")))
}

pub fn delete_value(ctx: ExecuteContext, key: Option<String>) -> Result<Response, ContractError> {
    nonpayable(&ctx.info)?;
    let sender = ctx.info.sender;

    let key = get_key_or_default(&key);
    ensure!(
        has_key_permission(ctx.deps.storage, &sender, key)?,
        ContractError::Unauthorized {}
    );
    DATA.remove(ctx.deps.storage, key);
    KEY_OWNER.remove(ctx.deps.storage, key);
    Ok(Response::new()
        .add_attribute("method", "delete_value")
        .add_attribute("sender", sender)
        .add_attribute("key", key))
}
