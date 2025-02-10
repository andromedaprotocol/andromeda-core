use crate::{
    query::has_permission,
    state::{DATA, DATA_OWNER, RESTRICTION},
};
use andromeda_math::point::{PointCoordinate, PointRestriction};
use andromeda_std::{common::context::ExecuteContext, error::ContractError};
use cosmwasm_std::{ensure, Response};

pub fn update_restriction(
    ctx: ExecuteContext,
    restriction: PointRestriction,
) -> Result<Response, ContractError> {
    let sender = ctx.info.sender;
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
