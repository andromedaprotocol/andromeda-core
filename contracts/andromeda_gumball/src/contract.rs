use andromeda_protocol::{
    error::ContractError,
    gumball::{Config, ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    require,
};
use cosmwasm_std::{
    attr, coin, entry_point, to_binary, Addr, BalanceResponse, BankMsg, BankQuery, Binary,
    CosmosMsg, Deps, DepsMut, Env, MessageInfo, QuerierWrapper, QueryRequest, Response, StdError,
    StdResult, Uint128, WasmMsg, WasmQuery,
};
use cw2::{get_contract_version, set_contract_version};

use crate::state::CONFIG;
use andromeda_protocol::ownership::{
    execute_update_owner, is_contract_owner, query_contract_owner, CONTRACT_OWNER,
};
use cw721::{Cw721ExecuteMsg, Cw721QueryMsg, TokensResponse};

const MAX_LIMIT: u32 = 30;
// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda_gumball";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CONTRACT_OWNER.save(deps.storage, &info.sender)?;
    let config = Config {
        token_addr: deps.api.addr_canonicalize(&msg.token_addr)?,
        stable_denom: msg.stable_denom.clone(),
        price: msg.price.clone(),
    };
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attributes(vec![
        attr("action", "instantiate"),
        attr("token_addr", msg.token_addr),
        attr("stable_denom", msg.stable_denom),
        attr("price", msg.price),
    ]))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Claim {} => execute_claim(deps, env, info),
        ExecuteMsg::WithdrawFunds {} => execute_withdraw_funds(deps, env, info),
        ExecuteMsg::UpdateOwner { address } => execute_update_owner(deps, info, address),
    }
}

fn execute_claim(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let coin_denom = config.stable_denom.clone();

    require(
        info.funds.len() <= 1usize,
        ContractError::MoreThanOneCoin {},
    )?;

    let payment = info
        .funds
        .iter()
        .find(|x| x.denom == coin_denom && x.amount > Uint128::zero())
        .ok_or_else(|| {
            StdError::generic_err(format!("No {} assets are provided to deposit", coin_denom))
        })?;

    require(
        payment.amount >= config.price,
        ContractError::InsufficientFunds {},
    )?;

    //get randomed token_id
    let mut all_token_ids = vec![];
    let token_addr = deps.api.addr_humanize(&config.token_addr)?;
    let mut start_after = "".to_string();

    loop {
        let tokens_res = query_tokens(
            deps.querier,
            token_addr.clone(),
            info.sender.clone(),
            Some(start_after.clone()),
        )?;
        let len = tokens_res.tokens.len();
        for token_id in tokens_res.tokens {
            all_token_ids.push(token_id.clone());
            start_after = token_id;
        }

        if len < MAX_LIMIT as usize {
            break;
        }
    }

    let all_token_len = all_token_ids.len() as u64;
    require(all_token_len > 0, ContractError::InsufficientTokens {})?;
    let selected_id = env.block.time.seconds() % all_token_len;

    let token_id = all_token_ids[selected_id as usize].clone();

    //transfer NFT
    let msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: deps.api.addr_humanize(&config.token_addr)?.to_string(),
        msg: to_binary(&Cw721ExecuteMsg::TransferNft {
            recipient: info.sender.to_string(),
            token_id: token_id.clone(),
        })?,
        funds: vec![],
    });

    Ok(Response::new().add_message(msg).add_attributes(vec![
        attr("action", "claim"),
        attr("receiver", info.sender.to_string()),
        attr("token_id", token_id),
    ]))
}

fn execute_withdraw_funds(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    require(
        is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;

    let amount = query_balance(
        &deps.querier,
        env.contract.address,
        config.stable_denom.clone(),
    )?;
    require(!amount.is_zero(), ContractError::InsufficientFunds {})?;
    let msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![coin(amount.u128(), config.stable_denom)],
    });

    Ok(Response::new().add_message(msg).add_attributes(vec![
        attr("action", "withdraw_funds"),
        attr("receiver", info.sender.to_string()),
        attr("amount", amount),
    ]))
}

fn query_tokens(
    querier: QuerierWrapper,
    contract_addr: Addr,
    account_addr: Addr,
    start_after: Option<String>,
) -> StdResult<TokensResponse> {
    // load price form the oracle
    let tokens_res: TokensResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: contract_addr.to_string(),
        msg: to_binary(&Cw721QueryMsg::Tokens {
            owner: account_addr.to_string(),
            start_after,
            limit: Some(MAX_LIMIT),
        })?,
    }))?;

    Ok(tokens_res)
}

fn query_balance(
    querier: &QuerierWrapper,
    account_addr: Addr,
    denom: String,
) -> StdResult<Uint128> {
    // load price form the oracle
    let balance: BalanceResponse = querier.query(&QueryRequest::Bank(BankQuery::Balance {
        address: account_addr.to_string(),
        denom,
    }))?;
    Ok(balance.amount.amount)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let version = get_contract_version(deps.storage)?;
    if version.contract != CONTRACT_NAME {
        return Err(ContractError::CannotMigrate {
            previous_contract: version.contract,
        });
    }
    Ok(Response::default())
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::ContractOwner {} => to_binary(&query_contract_owner(deps)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;

    Ok(ConfigResponse {
        token_addr: deps.api.addr_humanize(&config.token_addr)?.to_string(),
        stable_denom: config.stable_denom,
        price: config.price,
    })
}
