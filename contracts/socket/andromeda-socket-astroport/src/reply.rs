use andromeda_socket::astroport::PairExecuteMsg;
use andromeda_std::{
    ado_contract::ADOContract,
    amp::{
        messages::{AMPMsg, AMPPkt},
        AndrAddr, Recipient,
    },
    common::denom::Asset,
    error::ContractError,
};
use cosmwasm_std::{attr, wasm_execute, Coin, DepsMut, Env, Reply, Response, StdError, SubMsg};
use cw20::Cw20ExecuteMsg;

use crate::{
    astroport::{build_liquidity_messages, query_balance, ASTROPORT_MSG_FORWARD_ID},
    state::{
        ForwardReplyState, LIQUIDITY_PROVISION_STATE, LP_PAIR_ADDRESS, PREV_BALANCE,
        WITHDRAWAL_STATE,
    },
};

pub fn check_reply_result(msg: &Reply, operation: &str) -> Result<(), ContractError> {
    if msg.result.is_err() {
        Err(ContractError::Std(StdError::generic_err(format!(
            "Astroport {} failed with error: {:?}",
            operation,
            msg.result.clone().unwrap_err()
        ))))
    } else {
        Ok(())
    }
}

pub fn handle_astroport_swap_reply(
    deps: DepsMut,
    env: Env,
    msg: Reply,
    state: ForwardReplyState,
) -> Result<Response, ContractError> {
    let balance = query_balance(&deps.as_ref(), &env, &state.to_asset)?;
    let prev_balance = PREV_BALANCE.load(deps.storage)?;
    let return_amount = balance.checked_sub(prev_balance)?;
    PREV_BALANCE.remove(deps.storage);

    if return_amount.is_zero() {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Incomplete data in Astroport swap response: {:?}",
            msg
        ))));
    }

    let mut resp = Response::default();

    let transfer_msg = match &state.to_asset {
        Asset::NativeToken(denom) => {
            let funds = vec![Coin {
                denom: denom.to_string(),
                amount: return_amount,
            }];

            let mut pkt = if let Some(amp_ctx) = state.amp_ctx.clone() {
                AMPPkt::new(amp_ctx.get_origin(), amp_ctx.get_previous_sender(), vec![])
            } else {
                AMPPkt::new(
                    env.contract.address.clone(),
                    env.contract.address.clone(),
                    vec![],
                )
            };

            let Recipient { address, msg, .. } = &state.recipient;
            let msg = AMPMsg::new(
                address.clone(),
                msg.clone().unwrap_or_default(),
                Some(funds.clone()),
            );

            pkt = pkt.add_message(msg);
            let kernel_address = ADOContract::default().get_kernel_address(deps.storage)?;
            pkt.to_sub_msg(kernel_address, Some(funds), ASTROPORT_MSG_FORWARD_ID)?
        }
        Asset::Cw20Token(andr_addr) => {
            let Recipient { address, msg, .. } = &state.recipient;
            let transfer_msg = match msg {
                Some(msg) => Cw20ExecuteMsg::Send {
                    contract: address.get_raw_address(&deps.as_ref())?.to_string(),
                    amount: return_amount,
                    msg: msg.clone(),
                },
                None => Cw20ExecuteMsg::Transfer {
                    recipient: address.get_raw_address(&deps.as_ref())?.to_string(),
                    amount: return_amount,
                },
            };
            let wasm_msg = wasm_execute(
                andr_addr.get_raw_address(&deps.as_ref())?,
                &transfer_msg,
                vec![],
            )?;
            SubMsg::new(wasm_msg)
        }
    };
    let kernel_address = ADOContract::default().get_kernel_address(deps.storage)?;
    resp = resp.add_submessage(transfer_msg).add_attributes(vec![
        attr("action", "swap_and_forward"),
        attr("dex", "astroport"),
        attr("to_denom", state.to_asset.to_string()),
        attr("to_amount", return_amount),
        attr("recipient", state.recipient.get_addr()),
        attr("kernel_address", kernel_address),
    ]);
    Ok(resp)
}

pub fn handle_astroport_create_pair_reply(
    deps: DepsMut,
    msg: Reply,
) -> Result<Response, ContractError> {
    // Extract the pair address from the response
    let response = msg.result.unwrap();

    // Look for the pair contract address in the events
    let pair_address = response
        .events
        .iter()
        .find(|event| event.ty == "instantiate")
        .and_then(|event| {
            event
                .attributes
                .iter()
                .find(|attr| attr.key == "_contract_address")
                .map(|attr| attr.value.clone())
        })
        .ok_or_else(|| {
            ContractError::Std(StdError::generic_err(
                "Could not find pair contract address in response".to_string(),
            ))
        })?;

    // Store the pair address
    let pair_addr = AndrAddr::from_string(pair_address.clone());
    LP_PAIR_ADDRESS.save(deps.storage, &pair_addr)?;

    Ok(Response::default().add_attributes(vec![
        attr("action", "create_pair_success"),
        attr("pair_address", pair_address),
    ]))
}

pub fn handle_astroport_create_pair_and_provide_liquidity_reply(
    deps: DepsMut,
    msg: Reply,
) -> Result<Response, ContractError> {
    // Extract the pair address from the response
    let response = msg.result.unwrap();

    // Look for the pair contract address in the events
    let pair_address = response
        .events
        .iter()
        .find(|event| event.ty == "instantiate")
        .and_then(|event| {
            event
                .attributes
                .iter()
                .find(|attr| attr.key == "_contract_address")
                .map(|attr| attr.value.clone())
        })
        .ok_or_else(|| {
            ContractError::Std(StdError::generic_err(
                "Could not find pair contract address in response".to_string(),
            ))
        })?;

    let pair_addr = AndrAddr::from_string(pair_address.clone());
    LP_PAIR_ADDRESS.save(deps.storage, &pair_addr)?;

    // Load the liquidity provision parameters
    let liquidity_state = LIQUIDITY_PROVISION_STATE.load(deps.storage)?;
    LIQUIDITY_PROVISION_STATE.remove(deps.storage);

    // Build the provide liquidity message
    let provide_liquidity_msg = PairExecuteMsg::ProvideLiquidity {
        assets: liquidity_state.assets.clone(),
        slippage_tolerance: liquidity_state.slippage_tolerance,
        auto_stake: liquidity_state.auto_stake,
        receiver: liquidity_state.receiver,
    };

    let response = Response::new().add_messages(build_liquidity_messages(
        &liquidity_state.assets,
        pair_address.clone(),
        provide_liquidity_msg,
    )?);

    Ok(response.add_attributes(vec![
        attr("action", "create_pair_and_provide_liquidity_success"),
        attr("pair_address", pair_address),
        attr("liquidity_assets", format!("{:?}", liquidity_state.assets)),
    ]))
}

pub fn handle_astroport_withdraw_liquidity_reply(
    deps: DepsMut,
    msg: Reply,
) -> Result<Response, ContractError> {
    // Load the withdrawal state to get sender information
    let withdrawal_state = WITHDRAWAL_STATE.load(deps.storage)?;
    WITHDRAWAL_STATE.remove(deps.storage);

    // Parse the events to find what assets were refunded
    let response = msg.result.unwrap();
    let mut messages = vec![];

    // Look for refund_assets in the events and send them back to the user
    for event in &response.events {
        if event.ty == "wasm" {
            for attr in &event.attributes {
                if attr.key == "refund_assets" {
                    // Parse refund_assets: "63neutron1vsy34j8w9qwftp9p3pr74y8yvsdu3lt5rcx9t8s7gsxprenlqexssavs0j, 6untrn"
                    let assets: Vec<&str> = attr.value.split(", ").collect();

                    for asset_str in assets {
                        let asset_str = asset_str.trim();
                        if asset_str.is_empty() {
                            continue;
                        }

                        // Simple parsing: amount at start, rest is either contract address or denom
                        let (amount_str, remainder) = asset_str.split_at(
                            asset_str
                                .find(|c: char| !c.is_ascii_digit())
                                .unwrap_or(asset_str.len()),
                        );

                        if let Ok(amount) = amount_str.parse::<u128>() {
                            if amount > 0 {
                                let asset = if remainder.starts_with("neutron1") {
                                    Asset::Cw20Token(AndrAddr::from_string(remainder.to_string()))
                                } else {
                                    Asset::NativeToken(remainder.to_string())
                                };

                                let transfer_msg = asset.transfer(
                                    &deps.as_ref(),
                                    &withdrawal_state,
                                    amount.into(),
                                )?;
                                messages.push(transfer_msg.msg);
                            }
                        }
                    }
                    break;
                }
            }
        }
    }

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        attr("action", "withdraw_liquidity_success"),
        attr("recipient", withdrawal_state),
    ]))
}
