use andromeda_fungible_tokens::cw20_exchange::Sale;
use andromeda_std::{
    amp::Recipient,
    common::{
        context::ExecuteContext,
        denom::Asset,
        expiration::Expiry,
        msg_generation::{generate_transfer_message, generate_transfer_message_recipient},
        Milliseconds, MillisecondsDuration,
    },
    error::ContractError,
};
use cosmwasm_std::{attr, ensure, Response, Uint128};
use cw_utils::one_coin;

use crate::state::{SALE, TOKEN_ADDRESS};

#[allow(clippy::too_many_arguments)]
pub fn execute_start_sale(
    ctx: ExecuteContext,
    amount: Uint128,
    asset: Asset,
    exchange_rate: Uint128,
    // The original sender of the CW20::Send message
    sender: String,
    // The recipient of the sale proceeds
    recipient: Option<Recipient>,
    start_time: Option<Expiry>,
    duration: Option<MillisecondsDuration>,
) -> Result<Response, ContractError> {
    let recipient = Recipient::validate_or_default(recipient, &ctx, sender.as_str())?;

    let ExecuteContext {
        deps, env, info, ..
    } = ctx;

    let token_addr = TOKEN_ADDRESS.load(deps.storage)?;

    ensure!(
        asset != Asset::Cw20Token(token_addr.clone()),
        ContractError::InvalidAsset {
            asset: asset.to_string()
        }
    );
    ensure!(
        !exchange_rate.is_zero(),
        ContractError::InvalidZeroAmount {}
    );
    ensure!(
        ctx.contract.is_contract_owner(deps.storage, &sender)?,
        ContractError::Unauthorized {}
    );
    // Message sender in this case should be the token address
    ensure!(
        info.sender == token_addr.get_raw_address(&deps.as_ref())?,
        ContractError::InvalidFunds {
            msg: "Incorrect CW20 provided for sale".to_string()
        }
    );

    let start_time = match start_time {
        Some(s) => {
            // Check that the start time is in the future
            s.validate(&env.block)?
        }
        // Set start time to current time if not provided
        None => Expiry::FromNow(Milliseconds::zero()),
    }
    .get_time(&env.block);

    let end_time = match duration {
        Some(e) => {
            if e.is_zero() {
                // If duration is 0, set end time to none
                None
            } else {
                // Set end time to current time + duration
                Some(Expiry::FromNow(e).get_time(&env.block))
            }
        }
        None => None,
    };

    // Do not allow duplicate sales
    let asset_str = asset.inner(&deps.as_ref())?;
    let current_sale = SALE.may_load(deps.storage, &asset_str)?;
    ensure!(current_sale.is_none(), ContractError::SaleNotEnded {});

    let sale = Sale {
        start_amount: amount,
        remaining_amount: amount,
        exchange_rate,
        recipient,
        start_time,
        end_time,
    };
    SALE.save(deps.storage, &asset_str, &sale)?;

    Ok(Response::default().add_attributes(vec![
        attr("action", "start_sale"),
        attr("asset", asset.to_string()),
        attr("rate", exchange_rate),
        attr("amount", amount),
        attr("start_time", start_time.to_string()),
        attr("end_time", end_time.unwrap_or_default().to_string()),
    ]))
}

pub fn execute_purchase(
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
    let Some(mut sale) = SALE.may_load(deps.storage, &asset_sent_str)? else {
        return Err(ContractError::NoOngoingSale {});
    };

    // Check if sale has started
    ensure!(
        sale.start_time.is_expired(&ctx.env.block),
        ContractError::SaleNotStarted {}
    );
    // Check if sale has ended
    if let Some(end_time) = sale.end_time {
        ensure!(
            !end_time.is_expired(&ctx.env.block),
            ContractError::SaleEnded {}
        );
    }

    let purchased = amount_sent.checked_div(sale.exchange_rate).unwrap();
    let remainder = amount_sent.checked_sub(purchased.checked_mul(sale.exchange_rate)?)?;

    ensure!(
        !purchased.is_zero(),
        ContractError::InvalidFunds {
            msg: "Not enough funds sent to purchase a token".to_string()
        }
    );
    ensure!(
        sale.remaining_amount >= purchased,
        ContractError::NotEnoughTokens {}
    );

    // If purchase was rounded down return funds to purchaser
    if !remainder.is_zero() {
        resp = resp
            .add_submessage(generate_transfer_message(
                &deps.as_ref(),
                asset_sent.clone(),
                remainder,
                sender.to_string(),
                None,
            )?)
            .add_attribute("refunded_amount", remainder);
    }

    // Transfer tokens to purchaser recipient
    let token_addr = TOKEN_ADDRESS.load(deps.storage)?;

    let token_asset = Asset::Cw20Token(token_addr);
    let sub_msg = generate_transfer_message_recipient(
        &deps.as_ref(),
        token_asset,
        purchased,
        recipient.clone(),
        None,
    )?;

    resp = resp.add_submessage(sub_msg);

    // Update sale amount remaining
    sale.remaining_amount = sale.remaining_amount.checked_sub(purchased)?;
    SALE.save(deps.storage, &asset_sent_str, &sale)?;

    // Transfer exchanged asset to recipient
    resp = resp.add_submessage(generate_transfer_message_recipient(
        &deps.as_ref(),
        asset_sent.clone(),
        amount_sent - remainder,
        sale.recipient.clone(),
        None,
    )?);

    Ok(resp.add_attributes(vec![
        attr("action", "purchase"),
        attr("purchaser", sender),
        attr("recipient", recipient.address.to_string()),
        attr("amount", purchased),
        attr("purchase_asset", asset_sent.to_string()),
        attr("purchase_asset_amount_send", amount_sent - remainder),
        attr("recipient", sale.recipient.address.to_string()),
    ]))
}

pub fn execute_purchase_native(
    ctx: ExecuteContext,
    recipient: Option<Recipient>,
) -> Result<Response, ContractError> {
    let ExecuteContext { ref info, .. } = ctx;
    let sender = info.sender.to_string();
    let recipient = Recipient::validate_or_default(recipient, &ctx, sender.as_str())?;

    // Only allow one coin for purchasing
    let payment = one_coin(info)?;
    let asset = Asset::NativeToken(payment.denom.to_string());
    let amount = payment.amount;

    execute_purchase(ctx, amount, asset, recipient, &sender)
}

pub fn execute_cancel_sale(ctx: ExecuteContext, asset: Asset) -> Result<Response, ContractError> {
    let ExecuteContext { deps, info, .. } = ctx;

    let Some(sale) = SALE.may_load(deps.storage, &asset.inner(&deps.as_ref())?)? else {
        return Err(ContractError::NoOngoingSale {});
    };

    let mut resp = Response::default();

    // Refund any remaining amount
    if !sale.remaining_amount.is_zero() {
        let token_addr = TOKEN_ADDRESS.load(deps.storage)?;

        let token = Asset::Cw20Token(token_addr);
        resp = resp
            .add_submessage(generate_transfer_message(
                &deps.as_ref(),
                token,
                sale.remaining_amount,
                info.sender.to_string(),
                None,
            )?)
            .add_attribute("refunded_amount", sale.remaining_amount);
    }

    // Sale can now be removed
    SALE.remove(deps.storage, &asset.to_string());

    Ok(resp.add_attributes(vec![
        attr("action", "cancel_sale"),
        attr("asset", asset.to_string()),
    ]))
}
