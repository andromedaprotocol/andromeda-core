use andromeda_finance::mint_burn::{
    Cw20HookMsg, Cw721HookMsg, ExecuteMsg, OrderInfo, OrderStatus, Resource, ResourceRequirement,
};
use andromeda_std::{
    ado_contract::ADOContract,
    amp::AndrAddr,
    common::context::ExecuteContext,
    common::denom::{SEND_CW20_ACTION, SEND_NFT_ACTION},
    error::ContractError,
};
use cosmwasm_std::{from_json, to_json_binary, CosmosMsg, Deps, Response, Uint128, WasmMsg};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use cw721::{Cw721ExecuteMsg, Cw721ReceiveMsg};
use cw721_base::ExecuteMsg as BaseCw721ExecuteMsg;
use std::collections::HashMap;

use crate::state::{NEXT_ORDER_ID, ORDERS};

pub fn handle_receive_cw721(
    mut ctx: ExecuteContext,
    receive_msg: Cw721ReceiveMsg,
) -> Result<Response, ContractError> {
    ADOContract::default().is_permissioned(
        ctx.deps.branch(),
        ctx.env.clone(),
        SEND_NFT_ACTION,
        ctx.info.sender.clone(),
    )?;

    let received_token_id = receive_msg.token_id;
    let original_sender = AndrAddr::from_string(receive_msg.sender);

    match from_json(&receive_msg.msg)? {
        Cw721HookMsg::FillOrder {
            order_id,
            recipient,
        } => execute_fill_order(
            ctx,
            order_id,
            original_sender,
            recipient,
            Uint128::one(),
            Some(received_token_id),
        ),
    }
}

pub fn handle_receive_cw20(
    mut ctx: ExecuteContext,
    receive_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    ADOContract::default().is_permissioned(
        ctx.deps.branch(),
        ctx.env.clone(),
        SEND_CW20_ACTION,
        ctx.info.sender.clone(),
    )?;

    let original_sender = AndrAddr::from_string(receive_msg.sender);
    let cw20_amount = receive_msg.amount;

    match from_json(&receive_msg.msg)? {
        Cw20HookMsg::FillOrder {
            order_id,
            recipient,
        } => execute_fill_order(ctx, order_id, original_sender, recipient, cw20_amount, None),
    }
}

pub fn execute_create_order(
    ctx: ExecuteContext,
    msg: ExecuteMsg,
    requirements: Vec<ResourceRequirement>,
    output: Resource,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        mut deps,
        info,
        env,
        ..
    } = ctx;
    let sender = info.sender.clone();

    msg.requirements_number_validate()?;

    let order_id = NEXT_ORDER_ID.load(deps.storage)?;

    if ORDERS
        .may_load(deps.storage, order_id.clone().u128())?
        .is_some()
    {
        return Err(ContractError::CustomError {
            msg: "Order ID already exists".to_string(),
        });
    }

    // Initialize all resource requirements with empty deposit tracking
    let mut initialized_requirements = Vec::new();
    for requirement in requirements {
        requirement.validate(deps.branch(), env.clone())?;

        let new_req = ResourceRequirement {
            resource: requirement.clone().resource,
            amount: requirement.amount,
            deposits: HashMap::new(), // No deposits yet
        };
        initialized_requirements.push(new_req);
    }

    let new_order = OrderInfo {
        requirements: initialized_requirements,
        output,
        order_status: OrderStatus::NotCompleted,
        output_recipient: None,
    };

    ORDERS.save(deps.storage, order_id.u128(), &new_order)?;

    let next_order_id = order_id.checked_add(Uint128::one())?;
    NEXT_ORDER_ID.save(deps.storage, &next_order_id)?;

    Ok(Response::new()
        .add_attribute("method", "create_order")
        .add_attribute("order_id", order_id)
        .add_attribute("sender", sender))
}

pub fn execute_fill_order(
    ctx: ExecuteContext,
    order_id: Uint128,
    original_sender: AndrAddr,
    recipient: Option<AndrAddr>,
    amount: Uint128,
    received_token_id: Option<String>,
) -> Result<Response, ContractError> {
    let contract_addr = ctx.info.sender.clone();
    let original_sender_str = original_sender
        .clone()
        .get_raw_address(&ctx.deps.as_ref())?
        .to_string();

    let mut response = Response::new();

    let mut order = ORDERS
        .load(ctx.deps.storage, order_id.u128())
        .map_err(|_| ContractError::CustomError {
            msg: "Not existed order".to_string(),
        })?;

    if order.order_status == OrderStatus::Completed {
        return Err(ContractError::CustomError {
            msg: "Already completed order".to_string(),
        });
    }

    if order.order_status == OrderStatus::Cancelled {
        return Err(ContractError::CustomError {
            msg: "Already cancelled order".to_string(),
        });
    }

    let mut excess_amount = Uint128::zero();
    let mut refund_nft: Option<String> = None; // To handle NFT refunds
    let order_clone = order.clone();

    for requirement in &mut order.requirements {
        match &requirement.resource {
            Resource::Cw20Token { cw20_addr } => {
                if cw20_addr.clone().get_raw_address(&ctx.deps.as_ref())? == contract_addr {
                    let user_deposit = requirement
                        .deposits
                        .entry(original_sender_str.clone())
                        .or_insert(Uint128::zero());

                    let remaining = requirement.amount.checked_sub(*user_deposit)?;

                    if amount.le(&remaining) {
                        *user_deposit = user_deposit.checked_add(amount)?;
                    } else {
                        *user_deposit = requirement.amount;
                        excess_amount = amount.checked_sub(remaining)?;
                    }
                }
            }
            Resource::Nft {
                cw721_addr,
                token_id,
            } => {
                if cw721_addr.get_raw_address(&ctx.deps.as_ref())? == contract_addr {
                    if let Some(received_token_id) = received_token_id.clone() {
                        if token_id == &received_token_id {
                            // Allow deposit of any required NFT from the same contract
                            if !order_clone.requirements.iter().any(|r| {
                                matches!(&r.resource, Resource::Nft { cw721_addr: addr, token_id: tid }
                                    if addr == cw721_addr && tid == &received_token_id)
                            }) {
                                return Err(ContractError::CustomError {
                                    msg: format!("Received token ID {} is not required for this order", received_token_id),
                                });
                            }

                            let user_deposit = requirement
                                .deposits
                                .entry(original_sender_str.clone())
                                .or_insert(Uint128::zero());

                            let remaining = requirement.amount.checked_sub(*user_deposit)?;

                            if amount.le(&remaining) {
                                *user_deposit = user_deposit.checked_add(Uint128::one())?;
                            } else {
                                *user_deposit = requirement.amount;
                                refund_nft = Some(token_id.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    let user_fulfilled = order.requirements.iter().all(|r| {
        r.deposits
            .get(&original_sender_str.clone())
            .unwrap_or(&Uint128::zero())
            .ge(&r.amount)
    });

    // Save the updated order if not complete
    ORDERS.save(ctx.deps.storage, order_id.clone().u128(), &order)?;

    // Refund excess tokens (only applicable for CW20)
    if excess_amount.gt(&Uint128::zero()) {
        let refund_msg = WasmMsg::Execute {
            contract_addr: contract_addr.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: original_sender_str.clone(),
                amount: excess_amount,
            })?,
            funds: vec![],
        };
        response = response.clone().add_message(refund_msg);
    }

    // Refund the NFT if applicable
    if let Some(token_id) = refund_nft {
        let refund_msg = WasmMsg::Execute {
            contract_addr: contract_addr.to_string(),
            msg: to_json_binary(&Cw721ExecuteMsg::TransferNft {
                recipient: original_sender_str.clone(),
                token_id,
            })?,
            funds: vec![],
        };
        response = response.clone().add_message(refund_msg);
    }

    // If the order is fully filled, burn tokens/NFTs and mint the output
    if user_fulfilled {
        let mint_recipient = match recipient {
            Some(recipient) => recipient,
            None => original_sender.clone(),
        };

        order.order_status = OrderStatus::Completed;
        order.output_recipient = Some(mint_recipient.clone());
        ORDERS.save(ctx.deps.storage, order_id.clone().u128(), &order)?;
        let complete_order_msgs = complete_order(ctx, order.clone(), mint_recipient.clone())?;

        response = response
            .clone()
            .add_messages(complete_order_msgs)
            .add_attribute("order_status", "completed");
    }

    response = response
        .clone()
        .add_attribute("method", "fill_order")
        .add_attribute("order_id", order_id)
        .add_attribute("contract_addr", contract_addr.clone());

    Ok(response)
}

// Complete an order (burn resources and mint output)
fn complete_order(
    ctx: ExecuteContext,
    order: OrderInfo,
    recipient: AndrAddr,
) -> Result<Vec<CosmosMsg>, ContractError> {
    let mut messages = vec![];

    for requirement in &order.requirements {
        for (user, amount) in &requirement.deposits {
            if amount.gt(&Uint128::zero()) {
                let is_burning = *user == recipient.get_raw_address(&ctx.deps.as_ref())?;
                let burn_or_refund_msg = generate_burn_or_refund_msg(
                    ctx.deps.as_ref(),
                    requirement,
                    user.to_string(),
                    amount,
                    is_burning,
                )?;

                messages.push(burn_or_refund_msg);
            }
        }
    }

    // Mint the output resource
    let mint_msg = generate_mint_msg(ctx.deps.as_ref(), order, recipient)?;
    messages.push(mint_msg);

    Ok(messages)
}

fn generate_mint_msg(
    deps: Deps,
    order: OrderInfo,
    recipient: AndrAddr,
) -> Result<CosmosMsg, ContractError> {
    let msg = match &order.output {
        // Mint CW20 tokens
        Resource::Cw20Token { cw20_addr } => {
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cw20_addr.get_raw_address(&deps)?.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Mint {
                    recipient: recipient.get_raw_address(&deps)?.to_string(),
                    amount: Uint128::one(), // Fixed amount; adjust if dynamic minting is required
                })?,
                funds: vec![],
            })
        }
        // Mint an NFT
        Resource::Nft {
            cw721_addr,
            token_id,
        } => {
            let mint_token_id = token_id.clone();
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cw721_addr.get_raw_address(&deps)?.to_string(),
                msg: to_json_binary(&BaseCw721ExecuteMsg::<String, ()>::Mint {
                    token_id: mint_token_id.clone(),
                    owner: recipient.get_raw_address(&deps)?.to_string(),
                    token_uri: None, // Add a URI if needed
                    extension: "Andromeda".to_string(),
                })?,
                funds: vec![],
            })
        }
    };
    Ok(msg)
}

fn generate_burn_or_refund_msg(
    deps: Deps,
    requirement: &ResourceRequirement,
    user: String,
    amount: &Uint128,
    is_burning: bool,
) -> Result<CosmosMsg, ContractError> {
    let msg = match &requirement.resource {
        Resource::Cw20Token { cw20_addr } => {
            let contract_addr = cw20_addr.get_raw_address(&deps)?.to_string();
            let msg = if is_burning {
                Cw20ExecuteMsg::Burn { amount: *amount }
            } else {
                Cw20ExecuteMsg::Transfer {
                    recipient: user.clone(),
                    amount: *amount,
                }
            };
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg: to_json_binary(&msg)?,
                funds: vec![],
            })
        }
        Resource::Nft {
            cw721_addr,
            token_id,
        } => {
            let contract_addr = cw721_addr.get_raw_address(&deps)?.to_string();
            let msg = if is_burning {
                Cw721ExecuteMsg::Burn {
                    token_id: token_id.clone(),
                }
            } else {
                Cw721ExecuteMsg::TransferNft {
                    recipient: user.clone(),
                    token_id: token_id.clone(),
                }
            };
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg: to_json_binary(&msg)?,
                funds: vec![],
            })
        }
    };
    Ok(msg)
}

pub fn execute_cancel_order(
    ctx: ExecuteContext,
    order_id: Uint128,
) -> Result<Response, ContractError> {
    let mut order = ORDERS
        .load(ctx.deps.storage, order_id.clone().u128())
        .map_err(|_| ContractError::CustomError {
            msg: "Not existed order".to_string(),
        })?;
    if order.order_status == OrderStatus::Completed {
        return Err(ContractError::CustomError {
            msg: "Already completed order".to_string(),
        });
    }

    if order.order_status == OrderStatus::Cancelled {
        return Err(ContractError::CustomError {
            msg: "Already cancelled order".to_string(),
        });
    }

    let mut messages: Vec<CosmosMsg> = vec![];

    for requirement in &order.clone().requirements {
        for (user, amount) in &requirement.deposits {
            if (*amount).gt(&Uint128::zero()) {
                let refund_msg = generate_burn_or_refund_msg(
                    ctx.deps.as_ref(),
                    requirement,
                    user.to_string(),
                    amount,
                    false,
                )?;
                messages.push(refund_msg);
            }
        }
    }

    order.order_status = OrderStatus::Cancelled;
    ORDERS.save(ctx.deps.storage, order_id.clone().u128(), &order)?;

    let sender = ctx.info.sender.clone();

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("method", "cancel_order")
        .add_attribute("order_id", order_id)
        .add_attribute("sender", sender))
}
