use crate::state::{
    Config, Purchase, State, AMOUNT_TO_SEND, CONFIG, PURCHASES, STATE, TOKEN_AVAILABILITY,
};
use ado_base::ADOContract;
use andromeda_protocol::{
    crowdfund::{ExecuteMsg, InstantiateMsg, QueryMsg},
    rates::get_tax_amount,
};
use common::{
    ado_base::{recipient::Recipient, InstantiateMsg as BaseInstantiateMsg},
    encode_binary,
    error::ContractError,
    require, Funds,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    has_coins, to_binary, Binary, Coin, Deps, DepsMut, Env, MessageInfo, QuerierWrapper,
    QueryRequest, Response, StdResult, Uint128, WasmQuery,
};
use cw0::Expiration;
use cw721::{Cw721QueryMsg, OwnerOfResponse};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    let module_msgs = contract.register_modules(
        info.sender.as_str(),
        &deps.querier,
        deps.storage,
        deps.api,
        msg.modules,
    )?;
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
        ExecuteMsg::EndSale {} => panic!(),
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

    let state = state.unwrap();
    require(
        !state.expiration.is_expired(&env.block),
        ContractError::NoOngoingSale {},
    )?;
    // If the token is in this map, it has been purchased and is therefore unavailable.
    let token_available = !TOKEN_AVAILABILITY.has(deps.storage, &token_id);

    let config = CONFIG.load(deps.storage)?;
    let token_owner = query_owner_of(
        &deps.querier,
        config.token_address.to_string(),
        token_id.clone(),
    )?;
    require(
        token_owner == env.contract.address,
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
        info.funds.len() == 1,
        ContractError::InvalidFunds {
            msg: "Must send exactly one type of coin".to_string(),
        },
    )?;
    let payment: &Coin = &info.funds[0];
    require(
        payment.denom == state.price.denom,
        ContractError::InvalidFunds {
            msg: "Sent denom does not match required denom".to_string(),
        },
    )?;
    let (msgs, _events, remainder) = ADOContract::default().on_funds_transfer(
        deps.storage,
        deps.querier,
        sender.clone(),
        Funds::Native(payment.to_owned()),
        encode_binary(&"")?,
    )?;
    let remaining_amount = remainder.try_get_coin()?;

    let amount_to_send = AMOUNT_TO_SEND.load(deps.storage)?;
    AMOUNT_TO_SEND.save(deps.storage, &(amount_to_send + remaining_amount.amount))?;

    let tax_amount = get_tax_amount(&msgs, state.price.amount - remaining_amount.amount);
    // require that the sender has sent enough for taxes
    require(
        has_coins(
            &info.funds,
            &Coin {
                denom: state.price.denom,
                amount: state.price.amount + tax_amount,
            },
        ),
        ContractError::InsufficientFunds {},
    )?;

    TOKEN_AVAILABILITY.save(deps.storage, &token_id, &false)?;

    let purchase = Purchase {
        token_id: token_id.clone(),
        tax_amount,
        msgs,
    };

    purchases.push(purchase);
    PURCHASES.save(deps.storage, &sender, &purchases)?;

    Ok(Response::new()
        .add_attribute("action", "purchase")
        .add_attribute("token_id", token_id))
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    Ok(to_binary(&"")?)
}

#[cfg(test)]
mod tests {}
