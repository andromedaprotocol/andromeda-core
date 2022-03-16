use crate::state::{Config, Purchase, State, CONFIG, PURCHASES, STATE, UNAVAILABLE_TOKENS};
use ado_base::ADOContract;
use andromeda_protocol::{
    crowdfund::{ExecuteMsg, InstantiateMsg, QueryMsg},
    cw721::{ExecuteMsg as Cw721ExecuteMsg, QueryMsg as Cw721QueryMsg},
    rates::get_tax_amount,
};
use common::{
    ado_base::{recipient::Recipient, InstantiateMsg as BaseInstantiateMsg},
    encode_binary,
    error::ContractError,
    primitive::PRIMITVE_CONTRACT,
    require, Funds,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    has_coins, to_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order,
    QuerierWrapper, QueryRequest, Response, StdResult, SubMsg, Uint128, WasmMsg, WasmQuery,
};
use cw0::Expiration;
use cw721::{OwnerOfResponse, TokensResponse};

const DEFAULT_LIMIT: u32 = 50;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    // TODO: change instantiate of ADOContract to not take deps directly to allow instantiating
    // first.
    contract.owner.save(deps.storage, &info.sender)?;
    contract
        .ado_type
        .save(deps.storage, &"crowdfund".to_string())?;
    let module_msgs = contract.register_modules(
        info.sender.as_str(),
        &deps.querier,
        deps.storage,
        deps.api,
        msg.modules,
    )?;
    PRIMITVE_CONTRACT.save(deps.storage, &msg.primitive_address)?;
    CONFIG.save(
        deps.storage,
        &Config {
            token_address: deps.api.addr_validate(&msg.token_address)?,
        },
    )?;
    let resp = contract.instantiate(
        deps,
        info,
        BaseInstantiateMsg {
            ado_type: "crowdfund".to_string(),
            operators: None,
        },
    )?;
    Ok(resp.add_submessages(module_msgs))
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
        ExecuteMsg::EndSale { limit } => execute_end_sale(deps, env, limit),
    }
}

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
    let contract = ADOContract::default();
    require(
        contract.is_contract_owner(deps.storage, info.sender.as_str())?,
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
            recipient,
        },
    )?;

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
    // If the token is in this map, it has been purchased and is therefore unavailable.
    let token_available = !UNAVAILABLE_TOKENS.has(deps.storage, &token_id);

    let config = CONFIG.load(deps.storage)?;
    let token_owner_res = query_owner_of(
        &deps.querier,
        config.token_address.to_string(),
        token_id.clone(),
    );
    require(
        token_owner_res.is_ok() && token_owner_res.unwrap() == env.contract.address,
        ContractError::TokenNotForSale {},
    )?;
    require(token_available, ContractError::TokenAlreadyPurchased {})?;

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
    let payment: &Coin = &info.funds[0];
    let (msgs, _events, remainder) = ADOContract::default().on_funds_transfer(
        deps.storage,
        deps.querier,
        sender.clone(),
        Funds::Native(state.price.clone()),
        encode_binary(&"")?,
    )?;
    let remaining_amount = remainder.try_get_coin()?;

    state.amount_to_send += remaining_amount.amount;

    let tax_amount = get_tax_amount(&msgs, state.price.amount - remaining_amount.amount);
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

    UNAVAILABLE_TOKENS.save(deps.storage, &token_id, &false)?;

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

fn execute_end_sale(
    deps: DepsMut,
    env: Env,
    limit: Option<u32>,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    require(
        state.expiration.is_expired(&env.block),
        ContractError::SaleNotEnded {},
    )?;
    if state.amount_sold < state.min_tokens_sold {
        issue_refunds_and_burn_tokens(deps, env, limit)
    } else {
        transfer_tokens_and_send_funds(deps, limit)
    }
}

fn issue_refunds_and_burn_tokens(
    deps: DepsMut,
    env: Env,
    limit: Option<u32>,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    let limit = limit.unwrap_or(DEFAULT_LIMIT) as usize;
    let mut refund_msgs: Vec<CosmosMsg> = vec![];
    // Issue refunds for `limit` number of users.
    let purchases: Vec<Vec<Purchase>> = PURCHASES
        .range(deps.storage, None, None, Order::Ascending)
        .take(limit)
        .flatten()
        .map(|(_v, p)| p)
        .collect();
    for purchase_vec in purchases.iter() {
        let purchaser = purchase_vec[0].purchaser.clone();
        // Remove each entry as they get processed.
        PURCHASES.remove(deps.storage, &purchaser);
        // Reduce a user's purchases into one message. While the tax paid on each item should
        // be the same, it is not guaranteed given that the rates module is mutable during the
        // sale.
        let amount = purchase_vec
            .iter()
            // This represents the total amount of funds they sent for each purchase.
            .map(|p| p.tax_amount + state.price.amount)
            // Adds up all of the purchases.
            .reduce(|accum, item| accum + item)
            .unwrap_or_else(Uint128::zero);

        if amount > Uint128::zero() {
            refund_msgs.push(CosmosMsg::Bank(BankMsg::Send {
                to_address: purchaser,
                amount: vec![Coin {
                    denom: state.price.denom.clone(),
                    amount,
                }],
            }));
        }
    }

    // Burn `limit` number of tokens
    let config = CONFIG.load(deps.storage)?;
    let tokens_to_burn = query_tokens(
        &deps.querier,
        config.token_address.to_string(),
        env.contract.address.to_string(),
        limit,
    )?;

    let burn_msgs: Result<Vec<CosmosMsg>, ContractError> = tokens_to_burn
        .into_iter()
        .map(|token_id| {
            Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: config.token_address.to_string(),
                funds: vec![],
                msg: encode_binary(&Cw721ExecuteMsg::Burn { token_id })?,
            }))
        })
        .collect();

    Ok(Response::new()
        .add_attribute("action", "issue_refunds_and_burn_tokens")
        .add_messages(refund_msgs)
        .add_messages(burn_msgs?))
}

fn transfer_tokens_and_send_funds(
    deps: DepsMut,
    limit: Option<u32>,
) -> Result<Response, ContractError> {
    let mut state = STATE.load(deps.storage)?;
    let mut resp = Response::new();
    // Send the funds if they haven't been sent yet.
    if state.amount_to_send > Uint128::zero() {
        let msg = state.recipient.generate_msg_native(
            deps.api,
            vec![Coin {
                denom: state.price.denom.clone(),
                amount: state.amount_to_send,
            }],
        )?;
        state.amount_to_send = Uint128::zero();
        STATE.save(deps.storage, &state)?;

        resp = resp.add_submessage(msg);
    }
    let limit = limit.unwrap_or(DEFAULT_LIMIT) as usize;
    let purchases: Vec<Purchase> = PURCHASES
        .range(deps.storage, None, None, Order::Ascending)
        .flatten()
        .map(|(_v, p)| p)
        .flatten()
        .collect();

    let config = CONFIG.load(deps.storage)?;
    let mut rate_messages: Vec<SubMsg> = vec![];
    let mut transfer_msgs: Vec<CosmosMsg> = vec![];
    let purchases_slice = &purchases[0..limit];
    let remove_all = limit >= purchases_slice.len()
        || purchases[limit].purchaser != purchases_slice[limit - 1].purchaser;
    let last_purchaser = &purchases[limit].purchaser;
    for purchase in purchases_slice.iter() {
        let purchaser = &purchase.purchaser;
        if purchaser != last_purchaser && !remove_all && PURCHASES.has(deps.storage, purchaser) {
            PURCHASES.remove(deps.storage, &purchase.purchaser);
        }
        rate_messages.extend(purchase.msgs.clone());
        transfer_msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.token_address.to_string(),
            msg: encode_binary(&Cw721ExecuteMsg::TransferNft {
                recipient: purchase.purchaser.to_owned(),
                token_id: purchase.token_id.to_owned(),
            })?,
            funds: vec![],
        }));
    }
    Ok(resp
        .add_messages(transfer_msgs)
        .add_submessages(rate_messages))
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
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    Ok(to_binary(&"")?)
}

#[cfg(test)]
mod tests {}
