use andromeda_data_storage::primitive::{ExecuteMsg, Primitive, PrimitiveRestriction};
use andromeda_std::{
    ado_base::rates::{Rate, RatesMessage},
    ado_contract::ADOContract,
    common::{actions::call_action, context::ExecuteContext, rates::get_tax_amount, Funds},
    error::ContractError,
};
use cosmwasm_std::{
    coin, ensure, BankMsg, Coin, CosmosMsg, CustomQuery, Deps, MessageInfo, Response, StdError,
    SubMsg, Uint128,
};
use cw_utils::nonpayable;

use crate::{
    query::{get_key_or_default, has_key_permission},
    state::{DATA, KEY_OWNER, RESTRICTION},
};

pub fn handle_execute<C: CustomQuery>(
    mut ctx: ExecuteContext<C>,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
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
        ExecuteMsg::SetValue { key, value } => set_value(ctx, key, value, action),
        ExecuteMsg::DeleteValue { key } => delete_value(ctx, key),
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

pub fn update_restriction<C: CustomQuery>(
    ctx: ExecuteContext<C>,
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

pub fn set_value<C: CustomQuery>(
    ctx: ExecuteContext<C>,
    key: Option<String>,
    value: Primitive,
    action: String,
) -> Result<Response, ContractError> {
    let sender = ctx.info.sender.clone();
    let key: &str = get_key_or_default(&key);
    ensure!(
        has_key_permission(ctx.deps.storage, &sender, key)?,
        ContractError::Unauthorized {}
    );
    // Validate the primitive value
    value.validate(ctx.deps.api)?;

    let tax_response = tax_set_value(ctx.deps.as_ref(), &ctx.info, action)?;

    DATA.update::<_, StdError>(ctx.deps.storage, key, |old| match old {
        Some(_) => Ok(value.clone()),
        None => Ok(value.clone()),
    })?;
    // Update the owner of the key
    KEY_OWNER.update::<_, StdError>(ctx.deps.storage, key, |old| match old {
        Some(old) => Ok(old),
        None => Ok(sender.clone()),
    })?;

    let mut response = Response::new()
        .add_attribute("method", "set_value")
        .add_attribute("sender", sender)
        .add_attribute("key", key)
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

pub fn delete_value<C: CustomQuery>(
    ctx: ExecuteContext<C>,
    key: Option<String>,
) -> Result<Response, ContractError> {
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

fn tax_set_value<C: CustomQuery>(
    deps: Deps<C>,
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
