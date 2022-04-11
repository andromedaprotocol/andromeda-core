use crate::state::{
    Config, Purchase, State, AVAILABLE_TOKENS, CONFIG, PURCHASES, SALE_CONDUCTED, STATE,
};
use ado_base::ADOContract;
use andromeda_protocol::{
    crowdfund::{ExecuteMsg, InstantiateMsg, QueryMsg},
    cw721::{ExecuteMsg as Cw721ExecuteMsg, MintMsg, QueryMsg as Cw721QueryMsg, TokenExtension},
    rates::get_tax_amount,
};
use common::{
    ado_base::{recipient::Recipient, InstantiateMsg as BaseInstantiateMsg},
    encode_binary,
    error::ContractError,
    merge_sub_msgs, require, Funds,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    has_coins, Api, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order,
    QuerierWrapper, QueryRequest, Response, Storage, SubMsg, Uint128, WasmMsg, WasmQuery,
};
use cw0::Expiration;
use cw721::{OwnerOfResponse, TokensResponse};

const MAX_LIMIT: u32 = 100;
const DEFAULT_LIMIT: u32 = 50;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    CONFIG.save(
        deps.storage,
        &Config {
            token_address: msg.token_address,
            can_mint_after_sale: msg.can_mint_after_sale,
        },
    )?;
    SALE_CONDUCTED.save(deps.storage, &false)?;
    ADOContract::default().instantiate(
        deps.storage,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "crowdfund".to_string(),
            operators: None,
            modules: msg.modules,
            primitive_contract: None,
        },
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AndrReceive(msg) => {
            ADOContract::default().execute(deps, env, info, msg, execute)
        }
        ExecuteMsg::Mint(mint_msg) => execute_mint(deps, env, info, mint_msg),
        ExecuteMsg::StartSale {
            expiration,
            price,
            min_tokens_sold,
            max_amount_per_wallet,
            recipient,
        } => execute_start_sale(
            deps,
            env,
            info,
            expiration,
            price,
            min_tokens_sold,
            max_amount_per_wallet,
            recipient,
        ),
        ExecuteMsg::Purchase { token_id } => execute_purchase(deps, env, info, token_id),
        ExecuteMsg::ClaimRefund {} => execute_claim_refund(deps, env, info),
        ExecuteMsg::EndSale { limit } => execute_end_sale(deps, env, limit),
    }
}

fn execute_mint(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mint_msg: Box<MintMsg<TokenExtension>>,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    require(
        contract.is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;
    // Can only mint when no sale is ongoing.
    require(
        STATE.may_load(deps.storage)?.is_none(),
        ContractError::SaleStarted {},
    )?;
    let sale_conducted = SALE_CONDUCTED.load(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;
    require(
        config.can_mint_after_sale || !sale_conducted,
        ContractError::CannotMintAfterSaleConducted {},
    )?;
    // We allow for owners other than the contract, incase the creator wants to set aside a few
    // tokens for some other use, say airdrop, team allocation, etc.  Only those which have the
    // contract as the owner will be available to sell.
    if mint_msg.owner == env.contract.address {
        // Mark token as available to purchase in next sale.
        AVAILABLE_TOKENS.save(deps.storage, &mint_msg.token_id, &true)?;
    }
    let mission_contract = contract.get_mission_contract(deps.storage)?;
    let contract_addr =
        config
            .token_address
            .get_address(deps.api, &deps.querier, mission_contract)?;
    Ok(Response::new()
        .add_attribute("action", "mint")
        .add_message(WasmMsg::Execute {
            contract_addr,
            msg: encode_binary(&Cw721ExecuteMsg::Mint(mint_msg))?,
            funds: vec![],
        }))
}

#[allow(clippy::too_many_arguments)]
fn execute_start_sale(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    expiration: Expiration,
    price: Coin,
    min_tokens_sold: Uint128,
    max_amount_per_wallet: Option<Uint128>,
    recipient: Recipient,
) -> Result<Response, ContractError> {
    require(
        ADOContract::default().is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;
    require(
        !matches!(expiration, Expiration::Never {}),
        ContractError::ExpirationMustNotBeNever {},
    )?;
    require(
        !expiration.is_expired(&env.block),
        ContractError::ExpirationInPast {},
    )?;
    SALE_CONDUCTED.save(deps.storage, &true)?;
    let state = STATE.may_load(deps.storage)?;
    require(state.is_none(), ContractError::SaleStarted {})?;
    let max_amount_per_wallet = max_amount_per_wallet.unwrap_or_else(|| Uint128::from(1u128));
    // This is to prevent cloning price.
    let price_str = price.to_string();
    STATE.save(
        deps.storage,
        &State {
            expiration,
            price,
            min_tokens_sold,
            max_amount_per_wallet,
            amount_sold: Uint128::zero(),
            amount_to_send: Uint128::zero(),
            amount_transferred: Uint128::zero(),
            recipient,
        },
    )?;

    SALE_CONDUCTED.save(deps.storage, &true)?;

    Ok(Response::new()
        .add_attribute("action", "start_sale")
        .add_attribute("expiration", expiration.to_string())
        .add_attribute("price", price_str)
        .add_attribute("min_tokens_sold", min_tokens_sold)
        .add_attribute("max_amount_per_wallet", max_amount_per_wallet))
}

fn execute_purchase(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
) -> Result<Response, ContractError> {
    let sender = info.sender.to_string();
    let state = STATE.may_load(deps.storage)?;
    require(state.is_some(), ContractError::NoOngoingSale {})?;

    let mut state = state.unwrap();
    require(
        !state.expiration.is_expired(&env.block),
        ContractError::NoOngoingSale {},
    )?;

    let config = CONFIG.load(deps.storage)?;
    let token_owner_res = query_owner_of(
        &deps.querier,
        config.token_address.get_address(
            deps.api,
            &deps.querier,
            ADOContract::default().get_mission_contract(deps.storage)?,
        )?,
        token_id.clone(),
    );
    require(
        token_owner_res.is_ok() && token_owner_res.unwrap() == env.contract.address,
        ContractError::TokenNotForSale {},
    )?;

    let token_is_available = AVAILABLE_TOKENS.has(deps.storage, &token_id);
    require(token_is_available, ContractError::TokenAlreadyPurchased {})?;

    AVAILABLE_TOKENS.remove(deps.storage, &token_id);

    let mut purchases = PURCHASES
        .may_load(deps.storage, &sender)?
        .unwrap_or_default();

    require(
        purchases.len() < state.max_amount_per_wallet.u128() as usize,
        ContractError::PurchaseLimitReached {},
    )?;
    require(
        has_coins(&info.funds, &state.price),
        ContractError::InsufficientFunds {},
    )?;
    let (msgs, _events, remainder) = ADOContract::default().on_funds_transfer(
        deps.storage,
        deps.api,
        deps.querier,
        sender.clone(),
        Funds::Native(state.price.clone()),
        encode_binary(&"")?,
    )?;
    let remaining_amount = remainder.try_get_coin()?;

    state.amount_to_send += remaining_amount.amount;

    let tax_amount = get_tax_amount(&msgs, state.price.amount, remaining_amount.amount);
    // require that the sender has sent enough for taxes
    require(
        has_coins(
            &info.funds,
            &Coin {
                denom: state.price.denom.clone(),
                amount: state.price.amount + tax_amount,
            },
        ),
        ContractError::InsufficientFunds {},
    )?;

    let purchase = Purchase {
        token_id: token_id.clone(),
        tax_amount,
        msgs,
        purchaser: sender.clone(),
    };

    purchases.push(purchase);
    PURCHASES.save(deps.storage, &sender, &purchases)?;

    state.amount_sold += Uint128::from(1u128);
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("action", "purchase")
        .add_attribute("token_id", token_id))
}

fn execute_claim_refund(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let state = STATE.may_load(deps.storage)?;
    require(state.is_some(), ContractError::NoOngoingSale {})?;
    let state = state.unwrap();
    require(
        state.expiration.is_expired(&env.block),
        ContractError::SaleNotEnded {},
    )?;
    require(
        state.amount_sold < state.min_tokens_sold,
        ContractError::MinSalesExceeded {},
    )?;

    let purchases = PURCHASES.may_load(deps.storage, info.sender.as_str())?;
    require(purchases.is_some(), ContractError::NoPurchases {})?;
    let purchases = purchases.unwrap();
    let refund_msg = process_refund(deps.storage, &purchases, &state.price);
    let mut resp = Response::new();
    if let Some(refund_msg) = refund_msg {
        resp = resp.add_message(refund_msg);
    }

    Ok(resp.add_attribute("action", "claim_refund"))
}

fn execute_end_sale(
    deps: DepsMut,
    env: Env,
    limit: Option<u32>,
) -> Result<Response, ContractError> {
    let state = STATE.may_load(deps.storage)?;
    require(state.is_some(), ContractError::NoOngoingSale {})?;
    let state = state.unwrap();
    require(
        state.expiration.is_expired(&env.block),
        ContractError::SaleNotEnded {},
    )?;
    if state.amount_sold < state.min_tokens_sold {
        issue_refunds_and_burn_tokens(deps, env, limit)
    } else {
        transfer_tokens_and_send_funds(deps, env, limit)
    }
}

fn issue_refunds_and_burn_tokens(
    deps: DepsMut,
    env: Env,
    limit: Option<u32>,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    require(limit > 0, ContractError::LimitMustNotBeZero {})?;
    let mut refund_msgs: Vec<CosmosMsg> = vec![];
    // Issue refunds for `limit` number of users.
    let purchases: Vec<Vec<Purchase>> = PURCHASES
        .range(deps.storage, None, None, Order::Ascending)
        .take(limit)
        .flatten()
        .map(|(_v, p)| p)
        .collect();
    for purchase_vec in purchases.iter() {
        let refund_msg = process_refund(deps.storage, purchase_vec, &state.price);
        if let Some(refund_msg) = refund_msg {
            refund_msgs.push(refund_msg);
        }
    }

    // Burn `limit` number of tokens
    let burn_msgs = get_burn_messages(
        deps.storage,
        &deps.querier,
        deps.api,
        env.contract.address.to_string(),
        limit,
    )?;

    if burn_msgs.is_empty() && purchases.is_empty() {
        // When all tokens have been burned and all purchases have been refunded, the sale is over.
        STATE.remove(deps.storage);
    }

    Ok(Response::new()
        .add_attribute("action", "issue_refunds_and_burn_tokens")
        .add_messages(refund_msgs)
        .add_messages(burn_msgs))
}

fn transfer_tokens_and_send_funds(
    deps: DepsMut,
    env: Env,
    limit: Option<u32>,
) -> Result<Response, ContractError> {
    let mut state = STATE.load(deps.storage)?;
    let mut resp = Response::new();
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    require(limit > 0, ContractError::LimitMustNotBeZero {})?;
    // Send the funds if they haven't been sent yet and if all of the tokens have been transferred.
    if state.amount_transferred == state.amount_sold {
        if state.amount_to_send > Uint128::zero() {
            let msg = state.recipient.generate_msg_native(
                deps.api,
                &deps.querier,
                ADOContract::default().get_mission_contract(deps.storage)?,
                vec![Coin {
                    denom: state.price.denom.clone(),
                    amount: state.amount_to_send,
                }],
            )?;
            state.amount_to_send = Uint128::zero();
            STATE.save(deps.storage, &state)?;

            resp = resp.add_submessage(msg);
        }
        // Once all purchased tokens have been transferred, begin burning `limit` number of tokens
        // that were not purchased.
        let burn_msgs = get_burn_messages(
            deps.storage,
            &deps.querier,
            deps.api,
            env.contract.address.to_string(),
            limit,
        )?;

        if burn_msgs.is_empty() {
            // When burn messages are empty, we have finished the sale, which is represented by
            // having no State.
            STATE.remove(deps.storage);
        } else {
            resp = resp.add_messages(burn_msgs);
        }

        // If we are here then there are no purchases to process so we can exit.
        return Ok(resp.add_attribute("action", "transfer_tokens_and_send_funds"));
    }
    let mut purchases: Vec<Purchase> = PURCHASES
        .range(deps.storage, None, None, Order::Ascending)
        .flatten()
        // Flatten Vec<Vec<Purchase>> into Vec<Purchase>.
        .flat_map(|(_v, p)| p)
        // Take one extra in order to compare what the next purchaser would be to check if some
        // purchases will be left over.
        .take(limit + 1)
        .collect();

    let config = CONFIG.load(deps.storage)?;
    let mut rate_messages: Vec<SubMsg> = vec![];
    let mut transfer_msgs: Vec<CosmosMsg> = vec![];

    let last_purchaser = if purchases.len() == 1 {
        purchases[0].purchaser.clone()
    } else {
        purchases[purchases.len() - 2].purchaser.clone()
    };
    // This subtraction is no problem as we will always have at least one purchase.
    let subsequent_purchase = &purchases[purchases.len() - 1];
    // If this is false, then there are some purchases that we will need to leave for the next
    // round. Otherwise, we are able to process all of the purchases for the last purchaser and we
    // can remove their entry from the map entirely.
    let remove_last_purchaser = last_purchaser != subsequent_purchase.purchaser;

    let mut number_of_last_purchases_removed = 0;
    // If we took an extra element, we remove it. Otherwise limit + 1 was more than was necessary
    // so we need to remove all of the purchases from the map.
    if limit + 1 == purchases.len() {
        // This is an O(1) operation from looking at the source code.
        purchases.pop();
    }
    for purchase in purchases.into_iter() {
        let purchaser = purchase.purchaser;
        let should_remove = purchaser != last_purchaser || remove_last_purchaser;
        if should_remove && PURCHASES.has(deps.storage, &purchaser) {
            PURCHASES.remove(deps.storage, &purchaser);
        } else if purchaser == last_purchaser {
            // Keep track of the number of purchases removed from the last purchaser to remove them
            // at the end, if not all of them were removed.
            number_of_last_purchases_removed += 1;
        }
        rate_messages.extend(purchase.msgs);
        transfer_msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.token_address.get_address(
                deps.api,
                &deps.querier,
                ADOContract::default().get_mission_contract(deps.storage)?,
            )?,
            msg: encode_binary(&Cw721ExecuteMsg::TransferNft {
                recipient: purchaser,
                token_id: purchase.token_id,
            })?,
            funds: vec![],
        }));

        state.amount_transferred += Uint128::from(1u128);
    }
    // If the last purchaser wasn't removed, remove the subset of purchases that were processed.
    if PURCHASES.has(deps.storage, &last_purchaser) {
        let last_purchases = PURCHASES.load(deps.storage, &last_purchaser)?;
        PURCHASES.save(
            deps.storage,
            &last_purchaser,
            &last_purchases[number_of_last_purchases_removed..].to_vec(),
        )?;
    }
    STATE.save(deps.storage, &state)?;
    Ok(resp
        .add_attribute("action", "transfer_tokens_and_send_funds")
        .add_messages(transfer_msgs)
        .add_submessages(merge_sub_msgs(rate_messages)))
}

/// Processes a vector of purchases for the SAME user by merging all funds into a single BankMsg.
/// The given purchaser is then removed from `PURCHASES`.
///
/// ## Arguments
/// * `storage`  - Mutable reference to Storage
/// * `purchase` - Vector of purchases for the same user to issue a refund message for.
/// * `price`    - The price of a token
///
/// Returns an `Option<CosmosMsg>` which is `None` when the amount to refund is zero.
fn process_refund(
    storage: &mut dyn Storage,
    purchases: &[Purchase],
    price: &Coin,
) -> Option<CosmosMsg> {
    let purchaser = purchases[0].purchaser.clone();
    // Remove each entry as they get processed.
    PURCHASES.remove(storage, &purchaser);
    // Reduce a user's purchases into one message. While the tax paid on each item should
    // be the same, it is not guaranteed given that the rates module is mutable during the
    // sale.
    let amount = purchases
        .iter()
        // This represents the total amount of funds they sent for each purchase.
        .map(|p| p.tax_amount + price.amount)
        // Adds up all of the purchases.
        .reduce(|accum, item| accum + item)
        .unwrap_or_else(Uint128::zero);

    if amount > Uint128::zero() {
        Some(CosmosMsg::Bank(BankMsg::Send {
            to_address: purchaser,
            amount: vec![Coin {
                denom: price.denom.clone(),
                amount,
            }],
        }))
    } else {
        None
    }
}

fn get_burn_messages(
    storage: &mut dyn Storage,
    querier: &QuerierWrapper,
    api: &dyn Api,
    address: String,
    limit: usize,
) -> Result<Vec<CosmosMsg>, ContractError> {
    let config = CONFIG.load(storage)?;
    let token_address = config.token_address.get_address(
        api,
        querier,
        ADOContract::default().get_mission_contract(storage)?,
    )?;
    let tokens_to_burn = query_tokens(querier, token_address.clone(), address, limit)?;

    tokens_to_burn
        .into_iter()
        .map(|token_id| {
            // Any token that is burnable has been added to this map, and so must be removed.
            AVAILABLE_TOKENS.remove(storage, &token_id);
            Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: token_address.clone(),
                funds: vec![],
                msg: encode_binary(&Cw721ExecuteMsg::Burn { token_id })?,
            }))
        })
        .collect()
}

fn query_owner_of(
    querier: &QuerierWrapper,
    token_address: String,
    token_id: String,
) -> Result<String, ContractError> {
    let res: OwnerOfResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: token_address,
        msg: encode_binary(&Cw721QueryMsg::OwnerOf {
            token_id,
            include_expired: None,
        })?,
    }))?;
    Ok(res.owner)
}

fn query_tokens(
    querier: &QuerierWrapper,
    token_address: String,
    owner: String,
    limit: usize,
) -> Result<Vec<String>, ContractError> {
    let res: TokensResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: token_address,
        msg: encode_binary(&Cw721QueryMsg::Tokens {
            owner,
            start_after: None,
            limit: Some(limit as u32),
        })?,
    }))?;
    Ok(res.tokens)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrQuery(msg) => ADOContract::default().query(deps, env, msg, query),
        QueryMsg::State {} => encode_binary(&query_state(deps)?),
        QueryMsg::Config {} => encode_binary(&query_config(deps)?),
    }
}

fn query_state(deps: Deps) -> Result<State, ContractError> {
    Ok(STATE.load(deps.storage)?)
}

fn query_config(deps: Deps) -> Result<Config, ContractError> {
    Ok(CONFIG.load(deps.storage)?)
}
