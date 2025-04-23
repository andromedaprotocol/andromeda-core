use andromeda_fungible_tokens::cw20_redeem::{
    Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg, RedemptionAssetResponse,
    RedemptionCondition, RedemptionResponse, TokenAddressResponse,
};
use andromeda_std::{
    ado_base::{InstantiateMsg as BaseInstantiateMsg, MigrateMsg},
    ado_contract::ADOContract,
    amp::Recipient,
    andr_execute_fn,
    common::{
        context::ExecuteContext,
        encode_binary,
        expiration::{expiration_from_milliseconds, get_and_validate_start_time, Expiry},
        msg_generation::generate_transfer_message,
        Milliseconds, MillisecondsDuration,
    },
    error::ContractError,
};
use cosmwasm_std::{
    attr, ensure, entry_point, from_json, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo,
    Reply, Response, StdError, Uint128,
};
use cw20::{BalanceResponse, Cw20Coin, Cw20QueryMsg, Cw20ReceiveMsg};
use cw_asset::AssetInfo;
use cw_utils::{one_coin, Expiration};

use crate::state::{REDEMPTION_CONDITION, TOKEN_ADDRESS};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-cw20-redeem";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    TOKEN_ADDRESS.save(deps.storage, &msg.token_address)?;

    let contract = ADOContract::default();
    let resp = contract.instantiate(
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

    Ok(resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.result.is_err() {
        return Err(ContractError::Std(StdError::generic_err(
            msg.result.unwrap_err(),
        )));
    }

    Ok(Response::default())
}

#[andr_execute_fn]
pub fn execute(ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(cw20_msg) => execute_receive(ctx, cw20_msg),
        ExecuteMsg::SetRedemptionCondition {
            exchange_rate,
            recipient,
            start_time,
            duration,
        } => execute_set_redemption_condition_native(
            ctx,
            exchange_rate,
            recipient,
            start_time,
            duration,
        ),
        ExecuteMsg::CancelRedemptionCondition {} => execute_cancel_redemption_condition(ctx),
        _ => ADOContract::default().execute(ctx, msg),
    }
}

pub fn execute_receive(
    ctx: ExecuteContext,
    receive_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let ExecuteContext { ref info, .. } = ctx;
    let asset_info = AssetInfo::Cw20(info.sender.clone());
    let amount_sent = receive_msg.amount;
    let sender = receive_msg.sender;

    ensure!(
        !amount_sent.is_zero(),
        ContractError::InvalidFunds {
            msg: "Cannot send a 0 amount".to_string()
        }
    );

    match from_json(&receive_msg.msg)? {
        Cw20HookMsg::StartRedemptionCondition {
            exchange_rate,
            recipient,
            start_time,
            duration,
        } => execute_set_redemption_condition_cw20(
            ctx,
            amount_sent,
            asset_info,
            sender,
            exchange_rate,
            recipient,
            start_time,
            duration,
        ),
        Cw20HookMsg::Redeem {} => execute_redeem(ctx, amount_sent, asset_info, &sender),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn execute_set_redemption_condition_cw20(
    ctx: ExecuteContext,
    amount_sent: Uint128,
    asset_sent: AssetInfo,
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
        amount: amount_sent,
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
        amount,
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

pub fn execute_redeem(
    ctx: ExecuteContext,
    amount_sent: Uint128,
    asset_info: AssetInfo,
    sender: &str,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, .. } = ctx;

    let Some(mut redemption_condition) = REDEMPTION_CONDITION.may_load(deps.storage)? else {
        return Err(ContractError::NoOngoingSale {});
    };

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

    // If there's a refund, send it back to the sender
    if !refund_amount.is_zero() {
        messages.push(generate_transfer_message(
            asset_info.clone(),
            refund_amount,
            sender.to_string(),
            None,
        )?);
    }

    // Update sale amount remaining
    redemption_condition.amount = redemption_condition.amount.checked_sub(redeemed_amount)?;
    REDEMPTION_CONDITION.save(deps.storage, &redemption_condition)?;

    let mut attributes = vec![
        attr("action", "redeem"),
        attr("purchaser", sender),
        attr("amount", redeemed_amount),
        attr("purchase_asset", asset_info.to_string()),
        attr("purchase_asset_amount_accepted", accepted_amount),
    ];

    // Add refund attribute if there was a refund
    if !refund_amount.is_zero() {
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    ADOContract::default().migrate(deps, env, CONTRACT_NAME, CONTRACT_VERSION)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::RedemptionCondition {} => encode_binary(&query_redemption_condition(deps)?),
        QueryMsg::TokenAddress {} => encode_binary(&query_token_address(deps)?),
        QueryMsg::RedemptionAsset {} => encode_binary(&query_redemption_asset(deps)?),
        QueryMsg::RedemptionAssetBalance {} => {
            encode_binary(&query_redemption_asset_balance(deps, env)?)
        }
        _ => ADOContract::default().query(deps, env, msg),
    }
}

fn query_redemption_condition(deps: Deps) -> Result<RedemptionResponse, ContractError> {
    let redemption = REDEMPTION_CONDITION.may_load(deps.storage)?;

    Ok(RedemptionResponse { redemption })
}

fn query_token_address(deps: Deps) -> Result<TokenAddressResponse, ContractError> {
    let address = TOKEN_ADDRESS
        .load(deps.storage)?
        .get_raw_address(&deps)?
        .to_string();

    Ok(TokenAddressResponse { address })
}

fn query_redemption_asset(deps: Deps) -> Result<RedemptionAssetResponse, ContractError> {
    let redemption_condition = REDEMPTION_CONDITION.load(deps.storage)?;

    Ok(RedemptionAssetResponse {
        asset: redemption_condition.asset.to_string(),
    })
}

fn query_redemption_asset_balance(deps: Deps, env: Env) -> Result<Uint128, ContractError> {
    let asset = REDEMPTION_CONDITION.load(deps.storage)?.asset;

    match asset {
        AssetInfo::Native(denom) => {
            let balance = deps.querier.query_balance(env.contract.address, denom)?;
            Ok(balance.amount)
        }
        AssetInfo::Cw20(addr) => {
            let balance_msg = Cw20QueryMsg::Balance {
                address: env.contract.address.into(),
            };
            let balance_response: BalanceResponse = deps
                .querier
                .query_wasm_smart(addr, &to_json_binary(&balance_msg)?)?;
            Ok(balance_response.balance)
        }
        // Does not support 1155 currently
        _ => Err(ContractError::InvalidAsset {
            asset: asset.to_string(),
        }),
    }
}
