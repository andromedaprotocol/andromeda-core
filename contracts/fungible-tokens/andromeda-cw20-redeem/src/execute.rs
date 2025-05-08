use andromeda_fungible_tokens::cw20_redeem::RedemptionCondition;
use andromeda_std::{
    amp::Recipient,
    common::{
        context::ExecuteContext,
        expiration::{expiration_from_milliseconds, get_and_validate_start_time, Expiry},
        msg_generation::generate_transfer_message,
        Milliseconds, MillisecondsDuration,
    },
    error::ContractError,
};
use cosmwasm_std::{attr, ensure, Coin, Response, Uint128};
use cw20::Cw20Coin;
use cw_asset::AssetInfo;
use cw_utils::{one_coin, Expiration};

use crate::state::REDEMPTION_CONDITION;

#[allow(clippy::too_many_arguments)]
pub fn execute_set_redemption_condition_cw20(
    ctx: ExecuteContext,
    amount_sent: Uint128,
    asset_sent: AssetInfo,
    redeemed_asset: AssetInfo,
    sender: String,
    exchange_rate: Uint128,
    recipient: Option<Recipient>,
    start_time: Option<Expiry>,
    duration: Option<MillisecondsDuration>,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, env, .. } = ctx;

    ensure!(
        !exchange_rate.is_zero(),
        ContractError::InvalidZeroAmount {}
    );

    ensure!(
        ctx.contract.is_contract_owner(deps.storage, &sender)?,
        ContractError::Unauthorized {}
    );

    // If start time wasn't provided, it will be set as the current_time
    let (start_expiration, _current_time) = get_and_validate_start_time(&env, start_time.clone())?;

    let end_expiration = if let Some(duration) = duration {
        ensure!(!duration.is_zero(), ContractError::InvalidExpiration {});
        expiration_from_milliseconds(
            start_time
                // If start time isn't provided, it is set one second in advance from the current time
                .unwrap_or(Expiry::FromNow(Milliseconds::from_seconds(1)))
                .get_time(&env.block)
                .plus_milliseconds(duration),
        )?
    } else {
        Expiration::Never {}
    };

    // Do not allow duplicate sales
    let redemption_condition = REDEMPTION_CONDITION.may_load(deps.storage)?;
    ensure!(
        redemption_condition.is_none(),
        ContractError::RedemptionConditionAlreadyExists {}
    );

    let recipient = if let Some(recipient) = recipient {
        recipient.validate(&deps.as_ref())?;
        recipient
    } else {
        Recipient::new(sender, None)
    };

    let redemption_condition = RedemptionCondition {
        recipient,
        asset: asset_sent.clone(),
        redeemed_asset,
        amount: amount_sent,
        total_amount_redeemed: Uint128::zero(),
        exchange_rate,
        start_time: start_expiration,
        end_time: end_expiration,
    };
    REDEMPTION_CONDITION.save(deps.storage, &redemption_condition)?;

    Ok(Response::default().add_attributes(vec![
        attr("action", "start_redemption_condition"),
        attr("asset", asset_sent.to_string()),
        attr("rate", exchange_rate),
        attr("amount", amount_sent),
        attr("start_time", start_expiration.to_string()),
        attr("end_time", end_expiration.to_string()),
    ]))
}

pub fn execute_set_redemption_condition_native(
    ctx: ExecuteContext,
    redeemed_asset: AssetInfo,
    exchange_rate: Uint128,
    recipient: Option<Recipient>,
    start_time: Option<Expiry>,
    duration: Option<MillisecondsDuration>,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps, env, info, ..
    } = ctx;

    let payment = one_coin(&info)?;
    let asset = AssetInfo::Native(payment.denom.to_string());
    let amount = payment.amount;

    ensure!(
        !amount.is_zero(),
        ContractError::InvalidFunds {
            msg: "Cannot send a 0 amount".to_string()
        }
    );

    ensure!(
        !exchange_rate.is_zero(),
        ContractError::InvalidZeroAmount {}
    );

    // Check if a redemption condition already exists
    let redemption_condition = REDEMPTION_CONDITION.may_load(deps.storage)?;
    if let Some(condition) = redemption_condition {
        // If a condition exists, ensure it has expired before allowing a new one
        ensure!(
            condition.end_time.is_expired(&env.block),
            ContractError::RedemptionConditionAlreadyExists {}
        );
    }

    // If start time wasn't provided, it will be set as the current_time
    let (start_expiration, _current_time) = get_and_validate_start_time(&env, start_time.clone())?;

    let end_expiration = if let Some(duration) = duration {
        ensure!(!duration.is_zero(), ContractError::InvalidExpiration {});
        expiration_from_milliseconds(
            start_time
                // If start time isn't provided, it is set one second in advance from the current time
                .unwrap_or(Expiry::FromNow(Milliseconds::from_seconds(1)))
                .get_time(&env.block)
                .plus_milliseconds(duration),
        )?
    } else {
        Expiration::Never {}
    };

    let recipient = if let Some(recipient) = recipient {
        recipient.validate(&deps.as_ref())?;
        recipient
    } else {
        Recipient::new(info.sender.to_string(), None)
    };

    let redemption_condition = RedemptionCondition {
        recipient,
        asset: asset.clone(),
        redeemed_asset,
        amount,
        total_amount_redeemed: Uint128::zero(),
        exchange_rate,
        start_time: start_expiration,
        end_time: end_expiration,
    };
    REDEMPTION_CONDITION.save(deps.storage, &redemption_condition)?;

    Ok(Response::default().add_attributes(vec![
        attr("action", "start_redemption_condition"),
        attr("asset", asset.to_string()),
        attr("rate", exchange_rate),
        attr("amount", amount),
        attr("start_time", start_expiration.to_string()),
        attr("end_time", end_expiration.to_string()),
    ]))
}

pub fn execute_redeem_cw20(
    ctx: ExecuteContext,
    amount_sent: Uint128,
    asset_info: AssetInfo,
    sender: &str,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, .. } = ctx;

    let Some(mut redemption_condition) = REDEMPTION_CONDITION.may_load(deps.storage)? else {
        return Err(ContractError::NoOngoingSale {});
    };

    // Ensure that the provided asset is the same as the redeemed asset
    ensure!(
        asset_info == redemption_condition.redeemed_asset,
        ContractError::InvalidAsset {
            asset: asset_info.to_string(),
        }
    );

    // Check if sale has started
    ensure!(
        redemption_condition.start_time.is_expired(&ctx.env.block),
        ContractError::SaleNotStarted {}
    );
    // Check if sale has ended
    ensure!(
        !redemption_condition.end_time.is_expired(&ctx.env.block),
        ContractError::SaleEnded {}
    );

    let potential_redeemed = amount_sent.checked_mul(redemption_condition.exchange_rate)?;

    ensure!(
        !potential_redeemed.is_zero(),
        ContractError::InvalidFunds {
            msg: "Not enough funds sent to redeem".to_string()
        }
    );

    // Calculate actual redemption amounts
    let (redeemed_amount, accepted_amount, refund_amount) =
        if potential_redeemed <= redemption_condition.amount {
            (potential_redeemed, amount_sent, Uint128::zero())
        } else {
            // If we don't have enough tokens, calculate the partial redemption
            let actual_redeemed = redemption_condition.amount;
            let actual_amount_needed = redemption_condition
                .amount
                .checked_div(redemption_condition.exchange_rate)
                .map_err(|_| ContractError::Overflow {})?;
            let refund = amount_sent.checked_sub(actual_amount_needed)?;
            (actual_redeemed, actual_amount_needed, refund)
        };

    let mut messages = vec![];

    // Transfer redeemed tokens to the user
    messages.push(generate_transfer_message(
        redemption_condition.asset.clone(),
        redeemed_amount,
        sender.to_string(),
        None,
    )?);

    match asset_info {
        cw_asset::AssetInfoBase::Cw20(ref address) => {
            let recipient_msg = redemption_condition.recipient.generate_msg_cw20(
                &deps.as_ref(),
                Cw20Coin {
                    address: address.to_string(),
                    amount: accepted_amount,
                }
                .clone(),
            )?;
            messages.push(recipient_msg);
            Ok(())
        }
        _ => Err(ContractError::InvalidAsset {
            asset: asset_info.to_string(),
        }),
    }?;

    // Update sale amount remaining
    redemption_condition.amount = redemption_condition.amount.checked_sub(redeemed_amount)?;
    redemption_condition.total_amount_redeemed = redemption_condition
        .total_amount_redeemed
        .checked_add(redeemed_amount)?;
    REDEMPTION_CONDITION.save(deps.storage, &redemption_condition)?;

    let mut attributes = vec![
        attr("action", "redeem"),
        attr("purchaser", sender),
        attr("amount", redeemed_amount),
        attr("purchase_asset", asset_info.to_string()),
        attr("purchase_asset_amount_accepted", accepted_amount),
    ];

    // If there's a refund, send it back to the sender
    if !refund_amount.is_zero() {
        messages.push(generate_transfer_message(
            asset_info.clone(),
            refund_amount,
            sender.to_string(),
            None,
        )?);
        // Add refund attribute if there was a refund
        attributes.push(attr("refund_amount", refund_amount));
    }

    Ok(Response::default()
        .add_submessages(messages)
        .add_attributes(attributes))
}

pub fn execute_redeem_native(ctx: ExecuteContext) -> Result<Response, ContractError> {
    let ExecuteContext { deps, .. } = ctx;
    let payment = one_coin(&ctx.info)?;
    let amount_sent = payment.amount;
    let asset_info = AssetInfo::Native(payment.denom.to_string());
    let sender = ctx.info.sender.to_string();
    let Some(mut redemption_condition) = REDEMPTION_CONDITION.may_load(deps.storage)? else {
        return Err(ContractError::NoOngoingSale {});
    };

    // Ensure that the provided asset is the same as the redeemed asset
    ensure!(
        asset_info == redemption_condition.redeemed_asset,
        ContractError::InvalidAsset {
            asset: asset_info.to_string(),
        }
    );

    // Check if sale has started
    ensure!(
        redemption_condition.start_time.is_expired(&ctx.env.block),
        ContractError::SaleNotStarted {}
    );
    // Check if sale has ended
    ensure!(
        !redemption_condition.end_time.is_expired(&ctx.env.block),
        ContractError::SaleEnded {}
    );

    let potential_redeemed = amount_sent.checked_mul(redemption_condition.exchange_rate)?;

    ensure!(
        !potential_redeemed.is_zero(),
        ContractError::InvalidFunds {
            msg: "Not enough funds sent to redeem".to_string()
        }
    );

    // Calculate actual redemption amounts
    let (redeemed_amount, accepted_amount, refund_amount) =
        if potential_redeemed <= redemption_condition.amount {
            (potential_redeemed, amount_sent, Uint128::zero())
        } else {
            // If we don't have enough tokens, calculate the partial redemption
            let actual_redeemed = redemption_condition.amount;
            let actual_amount_needed = redemption_condition
                .amount
                .checked_div(redemption_condition.exchange_rate)
                .map_err(|_| ContractError::Overflow {})?;
            let refund = amount_sent.checked_sub(actual_amount_needed)?;
            (actual_redeemed, actual_amount_needed, refund)
        };

    let mut messages = vec![];

    // Transfer redeemed tokens to the user
    messages.push(generate_transfer_message(
        redemption_condition.asset.clone(),
        redeemed_amount,
        sender.to_string(),
        None,
    )?);

    match asset_info {
        cw_asset::AssetInfoBase::Native(ref denom) => {
            let recipient_msg: cosmwasm_std::SubMsg =
                redemption_condition.recipient.generate_direct_msg(
                    &deps.as_ref(),
                    vec![Coin {
                        denom: denom.to_string(),
                        amount: accepted_amount,
                    }],
                )?;
            messages.push(recipient_msg);
            Ok(())
        }
        _ => Err(ContractError::InvalidAsset {
            asset: asset_info.to_string(),
        }),
    }?;

    // Update sale amount remaining
    redemption_condition.amount = redemption_condition.amount.checked_sub(redeemed_amount)?;
    redemption_condition.total_amount_redeemed = redemption_condition
        .total_amount_redeemed
        .checked_add(redeemed_amount)?;
    REDEMPTION_CONDITION.save(deps.storage, &redemption_condition)?;

    let mut attributes = vec![
        attr("action", "redeem"),
        attr("purchaser", &sender),
        attr("amount", redeemed_amount),
        attr("purchase_asset", asset_info.to_string()),
        attr("purchase_asset_amount_accepted", accepted_amount),
    ];

    // If there's a refund, send it back to the sender
    if !refund_amount.is_zero() {
        messages.push(generate_transfer_message(
            asset_info.clone(),
            refund_amount,
            sender.to_string(),
            None,
        )?);
        // Add refund attribute if there was a refund
        attributes.push(attr("refund_amount", refund_amount));
    }

    Ok(Response::default()
        .add_submessages(messages)
        .add_attributes(attributes))
}

pub fn execute_cancel_redemption_condition(ctx: ExecuteContext) -> Result<Response, ContractError> {
    let ExecuteContext { deps, info, .. } = ctx;

    let Some(redemption_condition) = REDEMPTION_CONDITION.may_load(deps.storage)? else {
        return Err(ContractError::NoOngoingSale {});
    };

    let mut resp = Response::default();

    // Refund any remaining amount
    if !redemption_condition.amount.is_zero() {
        resp = resp
            .add_submessage(generate_transfer_message(
                redemption_condition.asset.clone(),
                redemption_condition.amount,
                info.sender.to_string(),
                None,
            )?)
            .add_attribute("refunded_amount", redemption_condition.amount);
    }

    // Redemption condition can now be removed
    REDEMPTION_CONDITION.remove(deps.storage);

    Ok(resp.add_attributes(vec![attr("action", "cancel_redemption_condition")]))
}
