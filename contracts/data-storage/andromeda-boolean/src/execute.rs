use crate::{
    contract::SET_DELETE_VALUE_ACTION,
    state::{DATA, DATA_OWNER, RESTRICTION},
};
use andromeda_data_storage::boolean::BooleanRestriction;
use andromeda_std::{
    ado_contract::ADOContract,
    common::{context::ExecuteContext, rates::get_tax_amount, Funds},
    error::ContractError,
};
use cosmwasm_std::{
    coin, ensure, BankMsg, Coin, CosmosMsg, Deps, MessageInfo, Response, SubMsg, Uint128,
};

pub fn update_restriction(
    ctx: ExecuteContext,
    restriction: BooleanRestriction,
) -> Result<Response, ContractError> {
    let sender = ctx.info.sender;
    RESTRICTION.save(ctx.deps.storage, &restriction)?;
    Ok(Response::new()
        .add_attribute("method", "update_restriction")
        .add_attribute("sender", sender))
}

pub fn set_value(
    mut ctx: ExecuteContext,
    value: bool,
    action: String,
) -> Result<Response, ContractError> {
    let sender = ctx.info.sender.clone();
    let restriction = RESTRICTION.load(ctx.deps.storage)?;
    if restriction == BooleanRestriction::Private {
        let has_permission = ADOContract::default()
            .is_permissioned(
                ctx.deps.branch(),
                ctx.env.clone(),
                SET_DELETE_VALUE_ACTION,
                ctx.info.sender.clone(),
            )
            .is_ok();
        ensure!(has_permission, ContractError::Unauthorized {});
    } else if restriction == BooleanRestriction::Restricted {
        let addr = sender.as_str();
        let is_operator = ADOContract::default().is_owner_or_operator(ctx.deps.storage, addr)?;
        let allowed = match DATA_OWNER.load(ctx.deps.storage).ok() {
            Some(owner) => addr == owner,
            None => true,
        };
        ensure!(is_operator || allowed, ContractError::Unauthorized {});
    }

    let tax_response = tax_set_value(ctx.deps.as_ref(), &ctx.info, action)?;

    DATA.save(ctx.deps.storage, &value.clone())?;
    DATA_OWNER.save(ctx.deps.storage, &sender)?;

    let mut response = Response::new()
        .add_attribute("method", "set_value")
        .add_attribute("sender", sender)
        .add_attribute("value", format!("{value:?}"));

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

pub fn delete_value(mut ctx: ExecuteContext) -> Result<Response, ContractError> {
    let sender = ctx.info.sender.clone();
    let restriction = RESTRICTION.load(ctx.deps.storage)?;
    if restriction == BooleanRestriction::Private {
        let has_permission = ADOContract::default()
            .is_permissioned(
                ctx.deps.branch(),
                ctx.env.clone(),
                SET_DELETE_VALUE_ACTION,
                ctx.info.sender.clone(),
            )
            .is_ok();
        ensure!(has_permission, ContractError::Unauthorized {});
    } else if restriction == BooleanRestriction::Restricted {
        let addr = sender.as_str();
        let is_operator = ADOContract::default().is_owner_or_operator(ctx.deps.storage, addr)?;
        let allowed = match DATA_OWNER.load(ctx.deps.storage).ok() {
            Some(owner) => addr == owner,
            None => true,
        };
        ensure!(is_operator || allowed, ContractError::Unauthorized {});
    }

    DATA.remove(ctx.deps.storage);
    DATA_OWNER.remove(ctx.deps.storage);
    Ok(Response::new()
        .add_attribute("method", "delete_value")
        .add_attribute("sender", sender))
}

fn tax_set_value(
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
