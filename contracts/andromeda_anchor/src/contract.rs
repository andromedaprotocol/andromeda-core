use crate::state::{
    Config, Position, CONFIG, KEY_POSITION_IDX, POSITION, PREV_AUST_BALANCE, TEMP_BALANCE,
};
use andromeda_protocol::{
    anchor::{ExecuteMsg, InstantiateMsg, QueryMsg},
    error::ContractError,
    ownership::{execute_update_owner, query_contract_owner, CONTRACT_OWNER},
    require,
};
use cosmwasm_std::{
    attr, coin, entry_point, to_binary, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, Reply, Response, StdError, StdResult, SubMsg, Uint128, WasmMsg,
};
use cw2::{get_contract_version, set_contract_version};
use cw20::Cw20ExecuteMsg;

use terraswap::querier::{query_balance, query_token_balance};

use andromeda_protocol::anchor::{AnchorMarketMsg, ConfigResponse, MigrateMsg, YourselfMsg};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-anchor";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let config = Config {
        anchor_mint: deps.api.addr_canonicalize(&msg.anchor_mint)?,
        anchor_token: deps.api.addr_canonicalize(&msg.anchor_token)?,
        stable_denom: msg.stable_denom,
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
        ExecuteMsg::Deposit {} => execute_deposit(deps, env, info),
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
        ExecuteMsg::UpdateOwner { address } => execute_update_owner(deps, info, address),
    }
}
pub fn execute_deposit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let coin_denom = config.stable_denom.clone();
    let depositor = info.sender.clone();

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
    //create position
    let position_idx = KEY_POSITION_IDX.load(deps.storage)?;
    let aust_balance = query_token_balance(
        &deps.querier,
        deps.api.addr_humanize(&config.anchor_token)?,
        env.contract.address,
    )?;
    PREV_AUST_BALANCE.save(deps.storage, &aust_balance)?;
    let payment_amount = payment.amount;

    POSITION.save(
        deps.storage,
        &position_idx.u128().to_be_bytes(),
        &Position {
            idx: Default::default(),
            owner: deps.api.addr_canonicalize(depositor.as_str())?,
            deposit_amount: payment_amount,
            aust_amount: Uint128::zero(),
        },
    )?;

    //deposit Anchor Mint
    Ok(Response::new()
        .add_submessage(SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: deps.api.addr_humanize(&config.anchor_mint)?.to_string(),
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
    let position_owner = deps.api.addr_humanize(&position.owner)?;

    require(
        position_owner == info.sender,
        ContractError::Unauthorized {},
    )?;

    let contract_balance = query_balance(
        &deps.querier,
        env.contract.address.clone(),
        config.stable_denom.clone(),
    )?;
    TEMP_BALANCE.save(deps.storage, &contract_balance)?;

    POSITION.remove(deps.storage, &position_idx.u128().to_be_bytes());

    Ok(Response::new()
        .add_messages(vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: deps.api.addr_humanize(&config.anchor_token)?.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: deps.api.addr_humanize(&config.anchor_mint)?.to_string(),
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
                        receiver: deps.api.addr_humanize(&position.owner)?.to_string(),
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

pub fn transfer_ust(deps: DepsMut, env: Env, receiver: String) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let current_balance = query_balance(
        &deps.querier,
        env.contract.address,
        config.stable_denom.clone(),
    )?;
    let prev_balance = TEMP_BALANCE.load(deps.storage)?;
    let transfer_amount = current_balance - prev_balance;
    let mut msg = vec![];
    if transfer_amount > Uint128::zero() {
        msg.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: receiver.to_string(),
            amount: vec![coin(transfer_amount.u128(), config.stable_denom)],
        }));
    }
    Ok(Response::new().add_messages(msg).add_attributes(vec![
        attr("action", "withdraw"),
        attr("receiver", receiver),
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
                deps.api.addr_humanize(&config.anchor_token)?,
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
        anchor_mint: deps.api.addr_humanize(&config.anchor_mint)?.to_string(),
        anchor_token: deps.api.addr_humanize(&config.anchor_token)?.to_string(),
        stable_denom: config.stable_denom,
    })
}
