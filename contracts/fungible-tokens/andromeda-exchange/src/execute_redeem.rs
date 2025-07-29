use andromeda_fungible_tokens::exchange::{ExchangeRate, Redeem};
use andromeda_std::{
    amp::Recipient,
    common::{
        context::ExecuteContext,
        denom::Asset,
        msg_generation::{generate_transfer_message, generate_transfer_message_recipient},
        schedule::Schedule,
    },
    error::ContractError,
};
use cosmwasm_std::{attr, ensure, Decimal256, Response, Uint128, Uint256};
use cw_utils::one_coin;

use crate::state::REDEEM;

#[allow(clippy::too_many_arguments)]
pub fn execute_start_redeem(
    ctx: ExecuteContext,
    amount: Uint128,
    // The asset sent with the message
    asset: Asset,
    // The accepted asset to be redeemed for the asset sent
    redeem_asset: Asset,
    exchange_rate: ExchangeRate,
    // The original sender of the CW20::Send message
    sender: String,
    // The recipient of the redeem proceeds
    recipient: Option<Recipient>,
    schedule: Schedule,
) -> Result<Response, ContractError> {
    let recipient = Recipient::validate_or_default(recipient, &ctx, sender.as_str())?;

    let ExecuteContext { deps, env, .. } = ctx;
    ensure!(
        asset != redeem_asset,
        ContractError::InvalidAsset {
            asset: format!(
                "The asset sent: {} cannot be the same as the redeem asset",
                asset
            )
        }
    );
    let derived_exchange_rate = exchange_rate.get_exchange_rate(amount)?;
    ensure!(
        !derived_exchange_rate.is_zero(),
        ContractError::InvalidZeroAmount {}
    );

    let (start_time, end_time) = schedule.validate(&env.block)?;
    // Do not allow duplicate redeems
    let redeem_asset_str = redeem_asset.inner(&deps.as_ref())?;
    let current_redeem = REDEEM.may_load(deps.storage, &redeem_asset_str)?;
    if let Some(redeem) = current_redeem {
        // The old redeem should either be expired or have no amount left
        ensure!(
            redeem
                .end_time
                .map(|e| e.is_expired(&env.block))
                .unwrap_or(false)
                || redeem.amount.is_zero(),
            ContractError::RedeemNotEnded {}
        );
    }

    let redeem = Redeem {
        asset: asset.clone(),
        amount,
        amount_paid_out: Uint128::zero(),
        exchange_rate: derived_exchange_rate,
        exchange_type: exchange_rate.clone(),
        recipient,
        start_time,
        end_time,
    };
    REDEEM.save(deps.storage, &redeem_asset_str, &redeem)?;

    Ok(Response::default().add_attributes(vec![
        attr("action", "start_redeem"),
        attr("redeem_asset", redeem_asset.to_string()),
        attr("asset", asset.to_string()),
        attr("rate", derived_exchange_rate.to_string()),
        attr("exchange_type", exchange_rate.to_string()),
        attr("amount", amount),
        attr("start_time", start_time.to_string()),
        attr("end_time", end_time.unwrap_or_default().to_string()),
    ]))
}

pub fn execute_replenish_redeem(
    ctx: ExecuteContext,
    amount: Uint128,
    asset: Asset,
    redeem_asset: Asset,
    exchange_rate_type: Option<ExchangeRate>,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, env, .. } = ctx;
    let redeem_asset_str = redeem_asset.inner(&deps.as_ref())?;
    // Ensure that the redeem exists
    let Some(mut redeem) = REDEEM.may_load(deps.storage, &redeem_asset_str)? else {
        return Err(ContractError::NoOngoingRedeem {});
    };
    // Ensure that the correct asset is being replenished
    ensure!(
        redeem.asset == asset,
        ContractError::InvalidAsset {
            asset: asset.to_string()
        }
    );
    // Ensure that the redeem has not ended
    if let Some(end_time) = redeem.end_time {
        ensure!(
            !end_time.is_expired(&env.block),
            ContractError::RedeemEnded {}
        );
    }
    // Add the replenishing amount to the redeem
    redeem.amount = redeem.amount.checked_add(amount)?;

    // Adjust the rate and type only if a a new one was provided by the user
    if let Some(exchange_rate_type) = exchange_rate_type {
        let exchange_rate_amount = exchange_rate_type.get_exchange_rate(redeem.amount)?;
        redeem.exchange_rate = exchange_rate_amount;
        redeem.exchange_type = exchange_rate_type;
    }

    REDEEM.save(deps.storage, &redeem_asset_str, &redeem)?;

    Ok(Response::default().add_attributes(vec![
        attr("action", "replenish_redeem"),
        attr("redeem_asset", redeem_asset.to_string()),
        attr("asset", asset.to_string()),
        attr("amount_added", amount),
    ]))
}

pub fn execute_redeem(
    ctx: ExecuteContext,
    amount_sent: Uint128,
    asset_sent: Asset,
    recipient: Recipient,
    // For refund purposes
    sender: &str,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, .. } = ctx;

    let mut resp = Response::default();

    let asset_sent_str = asset_sent.inner(&deps.as_ref())?;
    let Some(mut redeem) = REDEEM.may_load(deps.storage, &asset_sent_str)? else {
        return Err(ContractError::NoOngoingRedeem {});
    };

    // Check if redeem has started
    ensure!(
        redeem.start_time.is_expired(&ctx.env.block),
        ContractError::RedeemNotStarted {}
    );
    // Check if redeem has ended
    if let Some(end_time) = redeem.end_time {
        ensure!(
            !end_time.is_expired(&ctx.env.block),
            ContractError::RedeemEnded {}
        );
    }

    let payment_decimal = Decimal256::from_ratio(amount_sent, 1u128);
    let tokens_to_receive_decimal = payment_decimal.checked_mul(redeem.exchange_rate)?;
    let potential_redeemed = tokens_to_receive_decimal.to_uint_floor();

    // Calculate actual redemption amounts
    let (redeemed_amount, amount_received, refund_amount) =
        if potential_redeemed <= redeem.amount.into() {
            (potential_redeemed, amount_sent.into(), Uint256::zero())
        } else {
            // If we don't have enough tokens, calculate the partial redemption
            let actual_redeemed: Uint256 = redeem.amount.into();

            // Convert to Decimal256 for calculation
            let redeem_amount_decimal = Decimal256::from_ratio(redeem.amount, 1u128);
            let actual_amount_needed_decimal = redeem_amount_decimal
                .checked_div(redeem.exchange_rate)
                .map_err(|_| ContractError::Overflow {})?;
            let actual_amount_needed = actual_amount_needed_decimal.to_uint_ceil();
            let amount_sent_uint = Uint256::from(amount_sent);
            let refund = amount_sent_uint.checked_sub(actual_amount_needed)?;
            (actual_redeemed, actual_amount_needed, refund)
        };

    let refund_amount: Uint128 = refund_amount
        .try_into()
        .map_err(|_| ContractError::Overflow {})?;
    let redeemed_amount: Uint128 = redeemed_amount
        .try_into()
        .map_err(|_| ContractError::Overflow {})?;

    ensure!(
        !redeemed_amount.is_zero(),
        ContractError::InvalidFunds {
            msg: "Not enough funds sent to purchase a token".to_string()
        }
    );
    ensure!(
        redeem.amount >= redeemed_amount,
        ContractError::NotEnoughTokens {}
    );

    // If purchase was rounded down return funds to purchaser
    if !refund_amount.is_zero() {
        resp = resp
            .add_submessage(generate_transfer_message(
                &deps.as_ref(),
                asset_sent.clone(),
                refund_amount,
                sender.to_string(),
                None,
            )?)
            .add_attribute("refunded_amount", refund_amount);
    }

    // Transfer tokens to the user that's redeeming
    let redeem_asset = redeem.asset.clone();
    let redeem_recipient = redeem.clone().recipient;

    let transfer_msg = generate_transfer_message_recipient(
        &deps.as_ref(),
        redeem_asset.clone(),
        redeemed_amount,
        recipient.clone(),
        None,
    )?;
    resp = resp.add_submessage(transfer_msg);

    // Update redeem amount remaining
    redeem.amount = redeem.amount.checked_sub(redeemed_amount)?;
    redeem.amount_paid_out = redeem.amount_paid_out.checked_add(redeemed_amount)?;
    REDEEM.save(deps.storage, &asset_sent_str, &redeem)?;

    // Transfer exchanged asset to recipient
    resp = resp.add_submessage(generate_transfer_message_recipient(
        &deps.as_ref(),
        asset_sent.clone(),
        amount_sent - refund_amount,
        redeem_recipient.clone(),
        None,
    )?);

    Ok(resp.add_attributes(vec![
        attr("action", "redeem"),
        attr("redeemer", sender),
        attr("recipient", recipient.address.to_string()),
        attr("amount", amount_received),
        attr("redeem_asset", redeem_asset.to_string()),
        attr("redeem_asset_amount_send", amount_sent - refund_amount),
        attr("recipient", redeem_recipient.address.to_string()),
    ]))
}

pub fn execute_start_redeem_native(
    ctx: ExecuteContext,
    redeem_asset: Asset,
    exchange_rate: ExchangeRate,
    recipient: Option<Recipient>,
    schedule: Schedule,
) -> Result<Response, ContractError> {
    let ExecuteContext { ref info, .. } = ctx;

    let native_funds_sent = one_coin(info)?;
    let amount_sent = native_funds_sent.amount;
    let asset_sent = Asset::NativeToken(native_funds_sent.denom.to_string());
    let sender = info.sender.to_string();

    execute_start_redeem(
        ctx,
        amount_sent,
        asset_sent,
        redeem_asset,
        exchange_rate,
        sender,
        recipient,
        schedule,
    )
}

pub fn execute_replenish_redeem_native(
    ctx: ExecuteContext,
    redeem_asset: Asset,
    exchange_rate_type: Option<ExchangeRate>,
) -> Result<Response, ContractError> {
    let ExecuteContext { ref info, .. } = ctx;

    let native_funds_sent = one_coin(info)?;
    let amount_sent = native_funds_sent.amount;
    let asset_sent = Asset::NativeToken(native_funds_sent.denom.to_string());

    execute_replenish_redeem(
        ctx,
        amount_sent,
        asset_sent,
        redeem_asset,
        exchange_rate_type,
    )
}

pub fn execute_redeem_native(
    ctx: ExecuteContext,
    recipient: Option<Recipient>,
) -> Result<Response, ContractError> {
    let ExecuteContext { ref info, .. } = ctx;

    let sender = info.sender.to_string();
    let asset_sent = one_coin(info)?;
    let amount_sent = asset_sent.amount;
    let asset_sent = Asset::NativeToken(asset_sent.denom.to_string());

    let recipient = Recipient::validate_or_default(recipient, &ctx, sender.as_str())?;

    execute_redeem(ctx, amount_sent, asset_sent, recipient, &sender)
}

pub fn execute_cancel_redeem(ctx: ExecuteContext, asset: Asset) -> Result<Response, ContractError> {
    let ExecuteContext { deps, info, .. } = ctx;
    let asset_str = asset.inner(&deps.as_ref())?;

    let Some(redeem) = REDEEM.may_load(deps.storage, &asset_str)? else {
        return Err(ContractError::NoOngoingRedeem {});
    };

    let mut resp = Response::default();

    // Refund any remaining amount
    if !redeem.amount.is_zero() {
        let token = redeem.asset;
        resp = resp
            .add_submessage(generate_transfer_message(
                &deps.as_ref(),
                token,
                redeem.amount,
                info.sender.to_string(),
                None,
            )?)
            .add_attribute("refunded_amount", redeem.amount);
    }

    // Redeem can now be removed
    REDEEM.remove(deps.storage, asset_str.as_str());

    Ok(resp.add_attributes(vec![
        attr("action", "cancel_redeem"),
        attr("asset", asset.to_string()),
    ]))
}
