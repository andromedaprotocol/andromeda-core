use crate::state::{
    Config, Position, CONFIG, KEY_POSITION_IDX, POSITION, PREV_AUST_BALANCE, TEMP_BALANCE,
};
use andromeda_protocol::{
    anchor::{ExecuteMsg, InstantiateMsg, QueryMsg},
    communication::{encode_binary, parse_message, AndromedaMsg, AndromedaQuery, Recipient},
    error::ContractError,
    operators::{execute_update_operators, is_operator, query_is_operator, query_operators},
    ownership::{execute_update_owner, is_contract_owner, query_contract_owner, CONTRACT_OWNER},
    require,
};
use cosmwasm_std::{
    attr, coins, entry_point, to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply,
    Response, StdError, StdResult, SubMsg, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;

use terraswap::querier::{query_balance, query_token_balance};

use andromeda_protocol::anchor::{AnchorMarketMsg, ConfigResponse, YourselfMsg};

const UUSD_DENOM: &str = "uusd";

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let config = Config {
        anchor_market: deps.api.addr_canonicalize(&msg.anchor_market)?,
        aust_token: deps.api.addr_canonicalize(&msg.aust_token)?,
    };
    CONFIG.save(deps.storage, &config)?;
    KEY_POSITION_IDX.save(deps.storage, &Uint128::from(1u128))?;
    PREV_AUST_BALANCE.save(deps.storage, &Uint128::zero())?;
    TEMP_BALANCE.save(deps.storage, &Uint128::zero())?;
    CONTRACT_OWNER.save(deps.storage, &info.sender)?;
    Ok(Response::new().add_attributes(vec![attr("action", "instantiate"), attr("type", "anchor")]))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AndrReceive(msg) => execute_andr_receive(deps, env, info, msg),
        ExecuteMsg::Deposit { recipient } => execute_deposit(deps, env, info, recipient),
        ExecuteMsg::Withdraw { position_idx } => withdraw(deps, env, info, position_idx),
        ExecuteMsg::Yourself { yourself_msg } => {
            require(
                info.sender == env.contract.address,
                ContractError::Unauthorized {},
            )?;
            match yourself_msg {
                YourselfMsg::TransferUst { receiver } => transfer_ust(deps, env, receiver),
            }
        }
    }
}

fn execute_andr_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: AndromedaMsg,
) -> Result<Response, ContractError> {
    match msg {
        AndromedaMsg::Receive(data) => {
            let received: ExecuteMsg = parse_message(data)?;
            match received {
                ExecuteMsg::AndrReceive(..) => Err(ContractError::NestedAndromedaMsg {}),
                _ => execute(deps, env, info, received),
            }
        }
        AndromedaMsg::UpdateOwner { address } => execute_update_owner(deps, info, address),
        AndromedaMsg::UpdateOperators { operators } => {
            execute_update_operators(deps, info, operators)
        }
        AndromedaMsg::Withdraw { .. } => Err(ContractError::UnsupportedOperation {}),
    }
}

pub fn execute_deposit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: Option<Recipient>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let depositor = match recipient {
        Some(recipient) => recipient,
        None => Recipient::Addr(info.sender.to_string()),
    };

    require(
        info.funds.len() <= 1usize,
        ContractError::MoreThanOneCoin {},
    )?;

    let payment = info
        .funds
        .iter()
        .find(|x| x.denom == UUSD_DENOM && x.amount > Uint128::zero())
        .ok_or_else(|| {
            StdError::generic_err(format!("No {} assets are provided to deposit", UUSD_DENOM))
        })?;
    //create position
    let position_idx = KEY_POSITION_IDX.load(deps.storage)?;
    let aust_balance = query_token_balance(
        &deps.querier,
        deps.api.addr_humanize(&config.aust_token)?,
        env.contract.address,
    )?;
    PREV_AUST_BALANCE.save(deps.storage, &aust_balance)?;
    let payment_amount = payment.amount;

    POSITION.save(
        deps.storage,
        &position_idx.u128().to_be_bytes(),
        &Position {
            idx: Default::default(),
            owner: depositor,
            deposit_amount: payment_amount,
            aust_amount: Uint128::zero(),
        },
    )?;

    //deposit Anchor Mint
    Ok(Response::new()
        .add_submessage(SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: deps.api.addr_humanize(&config.anchor_market)?.to_string(),
                msg: to_binary(&AnchorMarketMsg::DepositStable {})?,
                funds: vec![payment.clone()],
            }),
            1u64,
        ))
        .add_attributes(vec![
            attr("action", "deposit"),
            attr("deposit_amount", payment_amount),
        ]))
}

pub fn withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    position_idx: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let position = POSITION.load(deps.storage, &position_idx.u128().to_be_bytes())?;

    require(
        is_operator(deps.storage, info.sender.as_str())?
            || is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;

    let contract_balance = query_balance(
        &deps.querier,
        env.contract.address.clone(),
        UUSD_DENOM.to_owned(),
    )?;
    TEMP_BALANCE.save(deps.storage, &contract_balance)?;

    POSITION.remove(deps.storage, &position_idx.u128().to_be_bytes());

    Ok(Response::new()
        .add_messages(vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: deps.api.addr_humanize(&config.aust_token)?.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: deps.api.addr_humanize(&config.anchor_market)?.to_string(),
                    amount: position.aust_amount,
                    msg: to_binary(&AnchorMarketMsg::RedeemStable {})?,
                })?,
                funds: vec![],
            }),
            //send UST
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                msg: to_binary(&ExecuteMsg::Yourself {
                    yourself_msg: YourselfMsg::TransferUst {
                        receiver: position.owner,
                    },
                })?,
                funds: vec![],
            }),
        ])
        .add_attributes(vec![
            attr("action", "withdraw"),
            attr("position_idx", position_idx.to_string()),
        ]))
}

pub fn transfer_ust(
    deps: DepsMut,
    env: Env,
    receiver: Recipient,
) -> Result<Response, ContractError> {
    let current_balance =
        query_balance(&deps.querier, env.contract.address, UUSD_DENOM.to_owned())?;
    let prev_balance = TEMP_BALANCE.load(deps.storage)?;
    let transfer_amount = current_balance - prev_balance;
    let mut msgs = vec![];
    if transfer_amount > Uint128::zero() {
        msgs.push(
            receiver
                .generate_msg_native(&deps.as_ref(), coins(transfer_amount.u128(), UUSD_DENOM))?,
        );
    }
    Ok(Response::new().add_submessages(msgs).add_attributes(vec![
        attr("action", "withdraw"),
        attr("receiver", receiver.get_addr()),
        attr("amount", transfer_amount.to_string()),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> StdResult<Response> {
    match msg.id {
        1u64 => {
            // stores aUST amount to position
            let config = CONFIG.load(deps.storage)?;
            let aust_balance = query_token_balance(
                &deps.querier,
                deps.api.addr_humanize(&config.aust_token)?,
                env.contract.address,
            )?;

            let prev_aust_balance = PREV_AUST_BALANCE.load(deps.storage)?;
            let new_aust_balance = aust_balance.checked_sub(prev_aust_balance)?;
            if new_aust_balance <= Uint128::from(1u128) {
                return Err(StdError::generic_err("no minted aUST token"));
            }
            let position_idx = KEY_POSITION_IDX.load(deps.storage)?;
            let mut position = POSITION.load(deps.storage, &position_idx.u128().to_be_bytes())?;
            position.aust_amount = new_aust_balance;
            POSITION.save(deps.storage, &position_idx.u128().to_be_bytes(), &position)?;
            KEY_POSITION_IDX.save(deps.storage, &(position_idx + Uint128::from(1u128)))?;
            Ok(Response::new().add_attributes(vec![
                attr("action", "reply"),
                attr("position_idx", position_idx.clone().to_string()),
                attr("aust_amount", new_aust_balance.to_string()),
            ]))
        }
        _ => Err(StdError::generic_err("invalid reply id")),
    }
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrQuery(msg) => handle_andromeda_query(deps, env, msg),
        QueryMsg::Config {} => encode_binary(&query_config(deps)?),
    }
}

fn handle_andromeda_query(
    deps: Deps,
    env: Env,
    msg: AndromedaQuery,
) -> Result<Binary, ContractError> {
    match msg {
        AndromedaQuery::Get(data) => {
            let received: QueryMsg = parse_message(data)?;
            match received {
                QueryMsg::AndrQuery(..) => Err(ContractError::NestedAndromedaMsg {}),
                _ => query(deps, env, received),
            }
        }
        AndromedaQuery::Owner {} => encode_binary(&query_contract_owner(deps)?),
        AndromedaQuery::Operators {} => encode_binary(&query_operators(deps)?),
        AndromedaQuery::IsOperator { address } => {
            encode_binary(&query_is_operator(deps, &address)?)
        }
    }
}

fn query_config(deps: Deps) -> Result<ConfigResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    Ok(ConfigResponse {
        anchor_market: deps.api.addr_humanize(&config.anchor_market)?.to_string(),
        aust_token: deps.api.addr_humanize(&config.aust_token)?.to_string(),
    })
}
