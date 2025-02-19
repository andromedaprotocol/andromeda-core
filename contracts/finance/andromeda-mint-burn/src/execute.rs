use andromeda_finance::mint_burn::{
    Cw20HookMsg, Cw721HookMsg, ExecuteMsg, OrderInfo, OrderStatus, Resource, ResourceRequirement,
};
use andromeda_non_fungible_tokens::cw721::TokenExtension;
use andromeda_std::{
    amp::AndrAddr,
    common::context::ExecuteContext,
    common::denom::{SEND_CW20_ACTION, SEND_NFT_ACTION},
    error::ContractError,
};
use cosmwasm_std::{
    ensure, from_json, to_json_binary, CosmosMsg, Deps, Response, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use cw721::{Cw721ExecuteMsg, Cw721ReceiveMsg};
use std::collections::HashMap;

use crate::state::{NEXT_ORDER_ID, ORDERS};

pub fn handle_receive_cw721(
    mut ctx: ExecuteContext,
    receive_msg: Cw721ReceiveMsg,
) -> Result<Response, ContractError> {
    ctx.contract.is_permissioned(
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
    ctx.contract.is_permissioned(
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
        contract,
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
        requirement.validate(deps.branch(), env.clone(), &contract)?;

        let new_requirement = ResourceRequirement {
            resource: requirement.clone().resource,
            amount: requirement.amount,
            deposits: HashMap::new(), // No deposits yet
        };
        initialized_requirements.push(new_requirement);
    }

    let new_order = OrderInfo {
        requirements: initialized_requirements,
        output,
        status: OrderStatus::NotCompleted,
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

    ensure!(
        order.status == OrderStatus::NotCompleted,
        ContractError::CustomError {
            msg: format!("Already {:?} Order", order.status)
        }
    );

    let mut excess_amount = Uint128::zero();
    let mut refund_nft: Option<String> = None; // To handle NFT refunds
    let order_clone = order.clone();

    let mut valid_cw20_token = false;
    let mut valid_nft_token = false;

    for requirement in &mut order.requirements {
        match &requirement.resource {
            Resource::Cw20Token { cw20_addr } => {
                if cw20_addr.clone().get_raw_address(&ctx.deps.as_ref())? == contract_addr {
                    valid_cw20_token = true;

                    process_cw20_deposit(
                        requirement,
                        original_sender_str.clone(),
                        amount,
                        &mut excess_amount,
                    )?;
                }
            }
            Resource::Nft {
                cw721_addr,
                token_id,
            } => {
                if cw721_addr.get_raw_address(&ctx.deps.as_ref())? == contract_addr {
                    valid_nft_token = true;

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

                            process_nft_deposit(
                                requirement,
                                original_sender_str.clone(),
                                received_token_id,
                                &mut refund_nft,
                            )?;
                        }
                    }
                }
            }
        }
    }

    // If a CW20 deposit was made, but the token was incorrect, throw an error
    if received_token_id.is_none() && amount.gt(&Uint128::zero()) && !valid_cw20_token {
        return Err(ContractError::CustomError {
            msg: "Invalid CW20 token sent".to_string(),
        });
    }

    // If an NFT was sent but was not required, throw an error
    if received_token_id.is_some() && !valid_nft_token {
        return Err(ContractError::CustomError {
            msg: format!(
                "Invalid CW721 token sent: {:?} is not part of this order",
                contract_addr
            ),
        });
    }

    let user_fulfilled = check_order_fulfillment(&order, &original_sender_str);

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

        order.status = OrderStatus::Completed;
        order.output_recipient = Some(mint_recipient.clone());
        ORDERS.save(ctx.deps.storage, order_id.clone().u128(), &order)?;
        let complete_order_msgs = complete_order(
            ctx,
            order.clone(),
            mint_recipient.clone(),
            original_sender.clone(),
        )?;

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

fn process_cw20_deposit(
    requirement: &mut ResourceRequirement,
    sender: String,
    amount: Uint128,
    excess_amount: &mut Uint128,
) -> Result<(), ContractError> {
    let user_deposit = requirement
        .deposits
        .entry(sender.clone())
        .or_insert(Uint128::zero());
    let remaining = requirement.amount.checked_sub(*user_deposit)?;

    if amount.le(&remaining) {
        *user_deposit = user_deposit.checked_add(amount)?;
    } else {
        *user_deposit = requirement.amount;
        *excess_amount = amount.checked_sub(remaining)?;
    }
    Ok(())
}

fn process_nft_deposit(
    requirement: &mut ResourceRequirement,
    sender: String,
    received_token_id: String,
    refund_nft: &mut Option<String>,
) -> Result<(), ContractError> {
    let user_deposit = requirement
        .deposits
        .entry(sender.clone())
        .or_insert(Uint128::zero());
    let remaining = requirement.amount.checked_sub(*user_deposit)?;

    if remaining.gt(&Uint128::zero()) {
        *user_deposit = user_deposit.checked_add(Uint128::one())?;
    } else {
        *refund_nft = Some(received_token_id.clone());
    }
    Ok(())
}

fn check_order_fulfillment(order: &OrderInfo, sender: &String) -> bool {
    order.requirements.iter().all(|r| {
        r.deposits
            .get(sender)
            .unwrap_or(&Uint128::zero())
            .ge(&r.amount)
    })
}

// Complete an order (burn resources and mint output)
fn complete_order(
    ctx: ExecuteContext,
    order: OrderInfo,
    mint_recipient: AndrAddr,
    original_sender: AndrAddr,
) -> Result<Vec<CosmosMsg>, ContractError> {
    let mut messages = vec![];

    for requirement in &order.requirements {
        for (user, amount) in &requirement.deposits {
            if amount.gt(&Uint128::zero()) {
                let is_burning = *user == original_sender.get_raw_address(&ctx.deps.as_ref())?;
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
    let mint_msg = generate_mint_msg(ctx.deps.as_ref(), order, mint_recipient)?;
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
                msg: to_json_binary(&andromeda_non_fungible_tokens::cw721::ExecuteMsg::Mint {
                    token_id: mint_token_id.clone(),
                    owner: recipient.get_raw_address(&deps)?.to_string(),
                    token_uri: None, // Add a URI if needed
                    extension: TokenExtension {
                        publisher: "ado_publisher".to_string(),
                    },
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

    ensure!(
        order.status == OrderStatus::NotCompleted,
        ContractError::CustomError {
            msg: format!("Already {:?} Order", order.status)
        }
    );

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

    order.status = OrderStatus::Cancelled;
    ORDERS.save(ctx.deps.storage, order_id.clone().u128(), &order)?;

    let sender = ctx.info.sender.clone();

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("method", "cancel_order")
        .add_attribute("order_id", order_id)
        .add_attribute("sender", sender))
}
