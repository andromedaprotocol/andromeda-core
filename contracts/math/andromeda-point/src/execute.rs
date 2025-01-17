use andromeda_math::point::{ExecuteMsg, PointCoordinate, PointRestriction};
use andromeda_std::{
    ado_contract::ADOContract,
    common::{actions::call_action, context::ExecuteContext},
    error::ContractError,
};
use cosmwasm_std::{ensure, Response};
use cw_utils::nonpayable;

use crate::{
    query::has_permission,
    state::{DATA, DATA_OWNER, RESTRICTION},
};

pub fn handle_execute(mut ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    let action_response = call_action(
        &mut ctx.deps,
        &ctx.info,
        &ctx.env,
        &ctx.amp_ctx,
        msg.as_ref(),
    )?;

    let res = match msg.clone() {
        ExecuteMsg::UpdateRestriction { restriction } => update_restriction(ctx, restriction),
        ExecuteMsg::SetPoint { point } => set_point(ctx, point),
        ExecuteMsg::DeletePoint {} => delete_point(ctx),
        _ => ADOContract::default().execute(ctx, msg),
    }?;

    Ok(res
        .add_submessages(action_response.messages)
        .add_attributes(action_response.attributes)
        .add_events(action_response.events))
}

pub fn update_restriction(
    ctx: ExecuteContext,
    restriction: PointRestriction,
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

pub fn set_point(ctx: ExecuteContext, point: PointCoordinate) -> Result<Response, ContractError> {
    let sender = ctx.info.sender.clone();
    ensure!(
        has_permission(ctx.deps.storage, &sender)?,
        ContractError::Unauthorized {}
    );

    DATA.save(ctx.deps.storage, &point.clone())?;
    DATA_OWNER.save(ctx.deps.storage, &sender)?;

    let response = Response::new()
        .add_attribute("method", "set_point")
        .add_attribute("sender", sender)
        .add_attribute("point", format!("{point:?}"));

    Ok(response)
}

pub fn delete_point(ctx: ExecuteContext) -> Result<Response, ContractError> {
    nonpayable(&ctx.info)?;
    let sender = ctx.info.sender;
    ensure!(
        has_permission(ctx.deps.storage, &sender)?,
        ContractError::Unauthorized {}
    );
    DATA.remove(ctx.deps.storage);
    DATA_OWNER.remove(ctx.deps.storage);
    Ok(Response::new()
        .add_attribute("method", "delete_point")
        .add_attribute("sender", sender))
}
