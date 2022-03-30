use cosmwasm_bignumber::{Decimal256, Uint256};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, coins, from_binary, to_binary, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, Reply, Response, SubMsg, Uint128, WasmMsg,
};

use crate::{
    primitive_keys::{
        ADDRESSES_TO_CACHE, ANCHOR_ANC, ANCHOR_AUST, ANCHOR_BLUNA, ANCHOR_BLUNA_CUSTODY,
        ANCHOR_BLUNA_HUB, ANCHOR_GOV, ANCHOR_MARKET, ANCHOR_ORACLE, ANCHOR_OVERSEER,
    },
    querier::{query_borrower_info, query_collaterals},
    state::{Position, POSITION, PREV_AUST_BALANCE, PREV_UUSD_BALANCE, RECIPIENT_ADDR},
};
use ado_base::ADOContract;
use anchor_token::gov::{
    Cw20HookMsg as GovCw20HookMsg, ExecuteMsg as GovExecuteMsg, QueryMsg as GovQueryMsg,
    StakerResponse,
};
use andromeda_protocol::anchor::{
    BLunaHubCw20HookMsg, BLunaHubExecuteMsg, BLunaHubQueryMsg, Cw20HookMsg, ExecuteMsg,
    InstantiateMsg, MigrateMsg, PositionResponse, QueryMsg, WithdrawableUnbondedResponse,
};
use common::{
    ado_base::{
        recipient::Recipient, AndromedaMsg, AndromedaQuery, InstantiateMsg as BaseInstantiateMsg,
    },
    encode_binary,
    error::ContractError,
    parse_message, require,
    withdraw::Withdrawal,
};
use cw2::{get_contract_version, set_contract_version};
use cw20::Cw20ReceiveMsg;
use cw20::{Cw20Coin, Cw20ExecuteMsg};
use cw_asset::AssetInfo;
use moneymarket::{
    custody::{Cw20HookMsg as CustodyCw20HookMsg, ExecuteMsg as CustodyExecuteMsg},
    market::{Cw20HookMsg as MarketCw20HookMsg, ExecuteMsg as MarketExecuteMsg},
    overseer::ExecuteMsg as OverseerExecuteMsg,
    querier::query_price,
};
use std::cmp;

const UUSD_DENOM: &str = "uusd";
pub const DEPOSIT_ID: u64 = 1;
pub const WITHDRAW_ID: u64 = 2;

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
    PREV_AUST_BALANCE.save(deps.storage, &Uint128::zero())?;
    PREV_UUSD_BALANCE.save(deps.storage, &Uint128::zero())?;

    let contract = ADOContract::default();

    let resp = contract.instantiate(
        deps.storage,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "anchor".to_string(),
            operators: None,
            modules: None,
            primitive_contract: Some(msg.primitive_contract),
        },
    )?;

    for address in ADDRESSES_TO_CACHE {
        contract.cache_address(deps.storage, &deps.querier, address)?;
    }
    let anchor_anc = contract.get_cached_address(deps.storage, ANCHOR_ANC)?;
    contract.add_withdrawable_token(
        deps.storage,
        &anchor_anc,
        &AssetInfo::Cw20(deps.api.addr_validate(&anchor_anc)?),
    )?;

    Ok(resp)
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::AndrReceive(msg) => execute_andr_receive(deps, env, info, msg),
        ExecuteMsg::DepositCollateral {} => execute_deposit_collateral(deps, env, info),
        ExecuteMsg::DepositCollateralToAnchor { collateral_addr } => {
            require(
                info.sender == env.contract.address,
                ContractError::Unauthorized {},
            )?;
            let collateral = AssetInfo::cw20(deps.api.addr_validate(&collateral_addr)?);
            let amount = collateral.query_balance(&deps.querier, env.contract.address.clone())?;
            execute_deposit_collateral_to_anchor(
                deps,
                env,
                info.sender.to_string(),
                collateral_addr,
                amount,
            )
        }
        ExecuteMsg::Borrow {
            desired_ltv_ratio,
            recipient,
        } => execute_borrow(deps, env, info, desired_ltv_ratio, recipient),
        ExecuteMsg::ClaimAncRewards { auto_stake } => {
            execute_claim_anc(deps, env, info, auto_stake)
        }
        ExecuteMsg::StakeAnc { amount } => execute_stake_anc(deps, env, info, amount),
        ExecuteMsg::UnstakeAnc { amount } => execute_unstake_anc(deps, env, info, amount),
        ExecuteMsg::RepayLoan {} => execute_repay_loan(deps, env, info),
        ExecuteMsg::WithdrawCollateral {
            collateral_addr,
            amount,
            unbond,
            recipient,
        } => {
            execute_withdraw_collateral(deps, env, info, collateral_addr, amount, unbond, recipient)
        }
        ExecuteMsg::WithdrawUnbonded { recipient } => {
            execute_withdraw_unbonded(deps, env, info, recipient)
        }
    }
}

pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    match from_binary(&cw20_msg.msg)? {
        Cw20HookMsg::DepositCollateral {} => execute_deposit_collateral_to_anchor(
            deps,
            env,
            cw20_msg.sender,
            info.sender.to_string(),
            cw20_msg.amount,
        ),
    }
}

fn execute_andr_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: AndromedaMsg,
) -> Result<Response, ContractError> {
    match msg {
        AndromedaMsg::Receive(data) => match data {
            None => execute_deposit(deps, env, info, None),
            Some(_) => {
                let recipient: Recipient = parse_message(&data)?;
                execute_deposit(deps, env, info, Some(recipient))
            }
        },
        AndromedaMsg::Withdraw {
            recipient,
            tokens_to_withdraw,
        } => execute_withdraw(deps, env, info, recipient, tokens_to_withdraw),
        _ => ADOContract::default().execute(deps, env, info, msg, execute),
    }
}

pub fn execute_withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: Option<Recipient>,
    tokens_to_withdraw: Option<Vec<Withdrawal>>,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();

    let aust_address = contract.get_cached_address(deps.storage, ANCHOR_AUST)?;
    match tokens_to_withdraw {
        None => return contract.execute_withdraw(deps, env, info, recipient, tokens_to_withdraw),
        Some(ref tokens) => {
            if tokens.len() != 1 {
                return contract.execute_withdraw(deps, env, info, recipient, tokens_to_withdraw);
            } else {
                let token = tokens[0].token.to_lowercase();
                if token != "aust" && token != aust_address && token != UUSD_DENOM {
                    return contract.execute_withdraw(
                        deps,
                        env,
                        info,
                        recipient,
                        tokens_to_withdraw,
                    );
                }
            }
        }
    }

    let recipient = recipient.unwrap_or_else(|| Recipient::Addr(info.sender.to_string()));

    let recipient_addr = recipient.get_addr(
        deps.api,
        &deps.querier,
        ADOContract::default().get_mission_contract(deps.storage)?,
    )?;

    let authorized = recipient_addr == info.sender
        || ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?;
    require(authorized, ContractError::Unauthorized {})?;

    require(
        matches!(recipient, Recipient::Addr(_)),
        ContractError::InvalidRecipientType {
            msg: "Only recipients of type Addr are allowed as it only specifies the owner of the position to withdraw from".to_string()
        },
    )?;

    // If we are here then there is always exactly a single token to withdraw.
    let token_to_withdraw = &tokens_to_withdraw.unwrap()[0];

    let token = token_to_withdraw.token.to_lowercase();
    if token == UUSD_DENOM {
        withdraw_uusd(deps, env, info, token_to_withdraw, recipient_addr)
    } else if token == "aust" || token == aust_address {
        withdraw_aust(deps, info, token_to_withdraw, recipient_addr)
    } else {
        Err(ContractError::InvalidTokensToWithdraw {
            msg: "Can only withdraw uusd or aUST".to_string(),
        })
    }
}

fn execute_deposit_collateral_to_anchor(
    deps: DepsMut,
    env: Env,
    sender: String,
    token_address: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    let anchor_bluna_token = contract.get_cached_address(deps.storage, ANCHOR_BLUNA)?;
    let anchor_bluna_custody = contract.get_cached_address(deps.storage, ANCHOR_BLUNA_CUSTODY)?;
    let anchor_overseer = contract.get_cached_address(deps.storage, ANCHOR_OVERSEER)?;

    require(
        contract.is_owner_or_operator(deps.storage, &sender)? || sender == env.contract.address,
        ContractError::Unauthorized {},
    )?;
    require(
        token_address == anchor_bluna_token,
        ContractError::InvalidFunds {
            msg: "Only bLuna collateral supported".to_string(),
        },
    )?;

    Ok(Response::new()
        .add_attribute("action", "deposit_collateral_to_anchor")
        // Provide collateral
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: token_address,
            funds: vec![],
            msg: encode_binary(&Cw20ExecuteMsg::Send {
                contract: anchor_bluna_custody,
                msg: encode_binary(&CustodyCw20HookMsg::DepositCollateral {})?,
                amount,
            })?,
        }))
        // Lock collateral
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: anchor_overseer,
            msg: encode_binary(&OverseerExecuteMsg::LockCollateral {
                collaterals: vec![(anchor_bluna_token, amount.into())],
            })?,
            funds: vec![],
        })))
}

fn execute_deposit_collateral(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    let anchor_bluna_token = contract.get_cached_address(deps.storage, ANCHOR_BLUNA)?;
    let anchor_bluna_hub = contract.get_cached_address(deps.storage, ANCHOR_BLUNA_HUB)?;

    require(
        contract.is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;
    require(
        info.funds.len() == 1,
        ContractError::InvalidFunds {
            msg: "Must deposit exactly 1 type of native coin.".to_string(),
        },
    )?;
    let collateral = &info.funds[0];
    require(
        collateral.denom == "uluna",
        ContractError::InvalidFunds {
            msg: "Only accept uluna as collateral".to_string(),
        },
    )?;

    Ok(Response::new()
        .add_attribute("action", "deposit_collateral")
        // Convert luna -> bluna
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: anchor_bluna_hub,
            funds: info.funds,
            msg: encode_binary(&BLunaHubExecuteMsg::Bond {})?,
        }))
        // Send collateral to Anchor
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            funds: vec![],
            msg: encode_binary(&ExecuteMsg::DepositCollateralToAnchor {
                collateral_addr: anchor_bluna_token,
            })?,
        })))
}

fn execute_withdraw_collateral(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collateral_addr: String,
    amount: Option<Uint256>,
    unbond: Option<bool>,
    recipient: Option<Recipient>,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    let anchor_bluna_token = contract.get_cached_address(deps.storage, ANCHOR_BLUNA)?;
    let anchor_bluna_custody = contract.get_cached_address(deps.storage, ANCHOR_BLUNA_CUSTODY)?;
    let anchor_bluna_hub = contract.get_cached_address(deps.storage, ANCHOR_BLUNA_HUB)?;
    let anchor_overseer = contract.get_cached_address(deps.storage, ANCHOR_OVERSEER)?;

    require(
        contract.is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;
    require(
        collateral_addr == anchor_bluna_token,
        ContractError::InvalidFunds {
            msg: "Only bluna collateral supported".to_string(),
        },
    )?;

    let amount = match amount {
        Some(amount) => amount,
        None => {
            let collaterals = query_collaterals(
                &deps.querier,
                anchor_overseer.clone(),
                env.contract.address.to_string(),
            )?
            .collaterals;

            let collateral_info = collaterals.iter().find(|c| c.0 == collateral_addr);

            require(collateral_info.is_some(), ContractError::InvalidAddress {})?;
            collateral_info.unwrap().1
        }
    };

    let final_message = if unbond.unwrap_or(false) {
        // do unbond message
        SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: collateral_addr.clone(),
            funds: vec![],
            msg: encode_binary(&Cw20ExecuteMsg::Send {
                contract: anchor_bluna_hub,
                amount: amount.into(),
                msg: encode_binary(&BLunaHubCw20HookMsg::Unbond {})?,
            })?,
        }))
    } else {
        // do withdraw message
        let recipient = recipient.unwrap_or_else(|| Recipient::Addr(info.sender.to_string()));
        recipient.generate_msg_cw20(
            deps.api,
            &deps.querier,
            contract.get_mission_contract(deps.storage)?,
            Cw20Coin {
                address: collateral_addr.clone(),
                amount: amount.into(),
            },
        )?
    };

    Ok(Response::new()
        .add_attribute("action", "withdraw_collateral")
        // Unlock collateral
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: anchor_overseer,
            msg: encode_binary(&OverseerExecuteMsg::UnlockCollateral {
                collaterals: vec![(collateral_addr, amount)],
            })?,
            funds: vec![],
        }))
        // Withdraw collateral
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: anchor_bluna_custody,
            funds: vec![],
            msg: encode_binary(&CustodyExecuteMsg::WithdrawCollateral {
                amount: Some(amount),
            })?,
        }))
        .add_submessage(final_message))
}

fn execute_borrow(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    desired_ltv_ratio: Decimal256,
    recipient: Option<Recipient>,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    let recipient = recipient.unwrap_or_else(|| Recipient::Addr(info.sender.to_string()));

    let anchor_overseer = contract.get_cached_address(deps.storage, ANCHOR_OVERSEER)?;
    let anchor_market = contract.get_cached_address(deps.storage, ANCHOR_MARKET)?;
    let anchor_oracle = contract.get_cached_address(deps.storage, ANCHOR_ORACLE)?;

    require(
        contract.is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;
    require(
        desired_ltv_ratio < Decimal256::one(),
        ContractError::InvalidLtvRatio {
            msg: "Desired LTV ratio must be less than 1".to_string(),
        },
    )?;
    let collaterals = query_collaterals(
        &deps.querier,
        anchor_overseer,
        env.contract.address.to_string(),
    )?
    .collaterals;

    let mut total_value = Uint256::zero();
    for collateral in collaterals.iter() {
        let price_res = query_price(
            deps.as_ref(),
            deps.api.addr_validate(&anchor_oracle)?,
            collateral.0.clone(),
            "uusd".to_string(),
            None,
        )?;
        total_value += price_res.rate * collateral.1;
    }

    let loan_amount = query_borrower_info(
        &deps.querier,
        anchor_market.clone(),
        env.contract.address.to_string(),
    )?
    .loan_amount;

    let current_ltv_ratio =
        Decimal256::from_uint256(loan_amount) / Decimal256::from_uint256(total_value);
    require(
        desired_ltv_ratio > current_ltv_ratio,
        ContractError::InvalidLtvRatio {
            msg: "Desired LTV ratio lower than current".to_string(),
        },
    )?;

    let borrow_amount = total_value * (desired_ltv_ratio - current_ltv_ratio);

    Ok(Response::new()
        .add_attribute("action", "borrow")
        .add_attribute("desired_ltv_ratio", desired_ltv_ratio.to_string())
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: anchor_market,
            msg: encode_binary(&MarketExecuteMsg::BorrowStable {
                borrow_amount,
                to: Some(env.contract.address.to_string()),
            })?,
            funds: vec![],
        }))
        .add_submessage(recipient.generate_msg_native(
            deps.api,
            &deps.querier,
            contract.get_mission_contract(deps.storage)?,
            coins(borrow_amount.into(), "uusd"),
        )?))
}

fn execute_repay_loan(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    let anchor_market = contract.get_cached_address(deps.storage, ANCHOR_MARKET)?;

    require(
        contract.is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;
    let borrower_info = query_borrower_info(
        &deps.querier,
        anchor_market.clone(),
        env.contract.address.to_string(),
    )?;
    let coin = info.funds.iter().find(|c| c.denom == "uusd");
    require(
        coin.is_some(),
        ContractError::InvalidFunds {
            msg: "Must send uusd".to_string(),
        },
    )?;
    let coin = coin.unwrap();
    let mut msgs: Vec<CosmosMsg> = vec![];
    let loan_amount = Uint128::from(borrower_info.loan_amount);
    if coin.amount > loan_amount {
        msgs.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: coins(coin.amount.u128() - loan_amount.u128(), coin.denom.clone()),
        }))
    }
    Ok(Response::new()
        .add_attribute("action", "repay_loan")
        .add_message(WasmMsg::Execute {
            contract_addr: anchor_market,
            msg: encode_binary(&MarketExecuteMsg::RepayStable {})?,
            funds: info.funds,
        })
        .add_messages(msgs))
}

fn execute_withdraw_unbonded(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: Option<Recipient>,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    require(
        contract.is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;
    let recipient = recipient.unwrap_or_else(|| Recipient::Addr(info.sender.to_string()));
    let anchor_bluna_hub = contract.get_cached_address(deps.storage, ANCHOR_BLUNA_HUB)?;

    let withdrawable_response: WithdrawableUnbondedResponse = deps.querier.query_wasm_smart(
        anchor_bluna_hub.clone(),
        &BLunaHubQueryMsg::WithdrawableUnbonded {
            address: env.contract.address.to_string(),
        },
    )?;
    let mission_contract = contract.get_mission_contract(deps.storage)?;
    Ok(Response::new()
        .add_message(WasmMsg::Execute {
            contract_addr: anchor_bluna_hub,
            msg: encode_binary(&BLunaHubExecuteMsg::WithdrawUnbonded {})?,
            funds: vec![],
        })
        .add_submessage(recipient.generate_msg_native(
            deps.api,
            &deps.querier,
            mission_contract,
            coins(withdrawable_response.withdrawable.u128(), "uluna"),
        )?))
}

fn execute_claim_anc(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    auto_stake: Option<bool>,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    require(
        contract.is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;
    let anchor_market = contract.get_cached_address(deps.storage, ANCHOR_MARKET)?;
    let res = Response::new()
        .add_attribute("action", "claim_anc_rewards")
        .add_message(WasmMsg::Execute {
            contract_addr: anchor_market.clone(),
            msg: encode_binary(&MarketExecuteMsg::ClaimRewards { to: None })?,
            funds: vec![],
        });
    if auto_stake.unwrap_or(false) {
        let borrower_info = query_borrower_info(
            &deps.querier,
            anchor_market,
            env.contract.address.to_string(),
        )?;
        let amount = borrower_info.pending_rewards * Uint256::one();
        let stake_resp = execute_stake_anc(deps, env, info, Some(amount.into()))?;
        Ok(res
            .add_attributes(stake_resp.attributes)
            .add_submessages(stake_resp.messages))
    } else {
        Ok(res)
    }
}

fn execute_stake_anc(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    require(
        contract.is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;
    let anchor_gov = contract.get_cached_address(deps.storage, ANCHOR_GOV)?;
    let anchor_anc = contract.get_cached_address(deps.storage, ANCHOR_ANC)?;

    let anc = AssetInfo::cw20(deps.api.addr_validate(&anchor_anc)?);
    let total_amount = anc.query_balance(&deps.querier, env.contract.address)?;

    let amount = cmp::min(total_amount, amount.unwrap_or(total_amount));
    Ok(Response::new()
        .add_attribute("action", "stake_anc")
        .add_attribute("amount", amount)
        .add_message(WasmMsg::Execute {
            contract_addr: anchor_anc,
            msg: encode_binary(&Cw20ExecuteMsg::Send {
                contract: anchor_gov,
                msg: encode_binary(&GovCw20HookMsg::StakeVotingTokens {})?,
                amount,
            })?,
            funds: vec![],
        }))
}

fn execute_unstake_anc(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    require(
        contract.is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;
    let anchor_gov = contract.get_cached_address(deps.storage, ANCHOR_GOV)?;
    let staker_response: StakerResponse = deps.querier.query_wasm_smart(
        anchor_gov.clone(),
        &GovQueryMsg::Staker {
            address: env.contract.address.to_string(),
        },
    )?;

    // If we ever support voting in polls, need to take into account
    // staker_response.locked_balance (if balance has deductions made to it).
    let amount = cmp::min(
        staker_response.balance,
        amount.unwrap_or(staker_response.balance),
    );

    Ok(Response::new()
        .add_attribute("action", "unstake_anc")
        .add_attribute("amount", amount)
        .add_message(WasmMsg::Execute {
            contract_addr: anchor_gov,
            msg: encode_binary(&GovExecuteMsg::WithdrawVotingTokens {
                amount: Some(amount),
            })?,
            funds: vec![],
        }))
}

pub fn execute_deposit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: Option<Recipient>,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    let anchor_market = contract.get_cached_address(deps.storage, ANCHOR_MARKET)?;
    let anchor_aust_token = contract.get_cached_address(deps.storage, ANCHOR_AUST)?;

    require(
        info.funds.len() == 1,
        ContractError::InvalidFunds {
            msg: "Must deposit exactly 1 type of native coin.".to_string(),
        },
    )?;

    let recipient = match recipient {
        Some(recipient) => recipient,
        None => Recipient::Addr(info.sender.to_string()),
    };

    let payment = &info.funds[0];
    require(
        payment.denom == UUSD_DENOM && payment.amount > Uint128::zero(),
        ContractError::InvalidFunds {
            msg: "Must deposit a non-zero quantity of uusd".to_string(),
        },
    )?;
    let aust = AssetInfo::cw20(deps.api.addr_validate(&anchor_aust_token)?);
    let aust_balance = aust.query_balance(&deps.querier, env.contract.address)?;
    let recipient_addr = recipient.get_addr(
        deps.api,
        &deps.querier,
        ADOContract::default().get_mission_contract(deps.storage)?,
    )?;
    PREV_AUST_BALANCE.save(deps.storage, &aust_balance)?;
    RECIPIENT_ADDR.save(deps.storage, &recipient_addr)?;
    let payment_amount = payment.amount;

    if !POSITION.has(deps.storage, &recipient_addr) {
        POSITION.save(
            deps.storage,
            &recipient_addr,
            &Position {
                recipient,
                aust_amount: Uint128::zero(),
            },
        )?;
    }

    //deposit Anchor Mint
    Ok(Response::new()
        .add_submessage(SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: anchor_market,
                msg: to_binary(&MarketExecuteMsg::DepositStable {})?,
                funds: vec![payment.clone()],
            }),
            DEPOSIT_ID,
        ))
        .add_attributes(vec![
            attr("action", "deposit"),
            attr("deposit_amount", payment_amount),
        ]))
}

// The amount to withdraw specified in `withdrawal` is denominated in aUST. So if the
// amount is say 50, that would signify exchanging 50 aUST for however much UST that produces.
fn withdraw_uusd(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    withdrawal: &Withdrawal,
    recipient_addr: String,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    let anchor_aust_token = contract.get_cached_address(deps.storage, ANCHOR_AUST)?;
    let anchor_market = contract.get_cached_address(deps.storage, ANCHOR_MARKET)?;

    let mut position = POSITION.load(deps.storage, &recipient_addr)?;

    let authorized = recipient_addr == info.sender
        || contract.is_owner_or_operator(deps.storage, info.sender.as_str())?;

    require(authorized, ContractError::Unauthorized {})?;

    let uusd = AssetInfo::native(UUSD_DENOM);
    let contract_balance = uusd.query_balance(&deps.querier, env.contract.address)?;

    PREV_UUSD_BALANCE.save(deps.storage, &contract_balance)?;
    RECIPIENT_ADDR.save(deps.storage, &recipient_addr)?;

    let amount_to_redeem = withdrawal.get_amount(position.aust_amount)?;

    position.aust_amount = position.aust_amount.checked_sub(amount_to_redeem)?;
    POSITION.save(deps.storage, &recipient_addr, &position)?;

    Ok(Response::new()
        .add_submessage(SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: anchor_aust_token,
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: anchor_market,
                    amount: amount_to_redeem,
                    msg: to_binary(&MarketCw20HookMsg::RedeemStable {})?,
                })?,
                funds: vec![],
            }),
            WITHDRAW_ID,
        ))
        .add_attributes(vec![
            attr("action", "withdraw_uusd"),
            attr("recipient_addr", recipient_addr),
        ]))
}

fn withdraw_aust(
    deps: DepsMut,
    info: MessageInfo,
    withdrawal: &Withdrawal,
    recipient_addr: String,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    let anchor_aust_token = contract.get_cached_address(deps.storage, ANCHOR_AUST)?;

    let mut position = POSITION.load(deps.storage, &recipient_addr)?;

    let authorized = recipient_addr == info.sender
        || contract.is_owner_or_operator(deps.storage, info.sender.as_str())?;

    require(authorized, ContractError::Unauthorized {})?;

    let amount = withdrawal.get_amount(position.aust_amount)?;

    position.aust_amount = position.aust_amount.checked_sub(amount)?;
    POSITION.save(deps.storage, &recipient_addr, &position)?;

    let msg = position.recipient.generate_msg_cw20(
        deps.api,
        &deps.querier,
        ADOContract::default().get_mission_contract(deps.storage)?,
        Cw20Coin {
            address: anchor_aust_token,
            amount,
        },
    )?;

    Ok(Response::new().add_submessage(msg).add_attributes(vec![
        attr("action", "withdraw_aust"),
        attr("recipient_addr", recipient_addr),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        DEPOSIT_ID => reply_update_position(deps, env),
        WITHDRAW_ID => reply_withdraw_ust(deps, env),
        _ => Err(ContractError::InvalidReplyId {}),
    }
}

fn reply_update_position(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    // stores aUST amount to position
    let contract = ADOContract::default();
    let anchor_aust_token = contract.get_cached_address(deps.storage, ANCHOR_AUST)?;

    let aust = AssetInfo::cw20(deps.api.addr_validate(&anchor_aust_token)?);
    let aust_balance = aust.query_balance(&deps.querier, env.contract.address)?;

    let prev_aust_balance = PREV_AUST_BALANCE.load(deps.storage)?;
    let new_aust_balance = aust_balance.checked_sub(prev_aust_balance)?;
    require(
        new_aust_balance > Uint128::zero(),
        ContractError::InvalidFunds {
            msg: "No aUST tokens minted".to_string(),
        },
    )?;

    let recipient_addr = RECIPIENT_ADDR.load(deps.storage)?;
    let mut position = POSITION.load(deps.storage, &recipient_addr)?;
    position.aust_amount += new_aust_balance;
    POSITION.save(deps.storage, &recipient_addr, &position)?;
    Ok(Response::new().add_attributes(vec![
        attr("action", "reply_update_position"),
        attr("recipient_addr", recipient_addr.clone()),
        attr("aust_amount", new_aust_balance.to_string()),
    ]))
}

fn reply_withdraw_ust(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let uusd = AssetInfo::native(UUSD_DENOM);
    let current_balance = uusd.query_balance(&deps.querier, env.contract.address)?;

    let prev_balance = PREV_UUSD_BALANCE.load(deps.storage)?;
    let transfer_amount = current_balance - prev_balance;

    let recipient_addr = RECIPIENT_ADDR.load(deps.storage)?;
    let recipient = POSITION.load(deps.storage, &recipient_addr)?.recipient;
    let mut msgs = vec![];
    if transfer_amount > Uint128::zero() {
        msgs.push(recipient.generate_msg_native(
            deps.api,
            &deps.querier,
            ADOContract::default().get_mission_contract(deps.storage)?,
            coins(transfer_amount.u128(), UUSD_DENOM),
        )?);
    }
    Ok(Response::new()
        .add_submessages(msgs)
        .add_attribute("action", "reply_withdraw_ust")
        .add_attribute("recipient", recipient_addr)
        .add_attribute("amount", transfer_amount))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrQuery(msg) => handle_andromeda_query(deps, env, msg),
    }
}

fn handle_andromeda_query(
    deps: Deps,
    env: Env,
    msg: AndromedaQuery,
) -> Result<Binary, ContractError> {
    match msg {
        AndromedaQuery::Get(data) => {
            let recipient: String = parse_message(&data)?;
            encode_binary(&query_position(deps, recipient)?)
        }
        _ => ADOContract::default().query(deps, env, msg, query),
    }
}

fn query_position(deps: Deps, recipient: String) -> Result<PositionResponse, ContractError> {
    let position = POSITION.load(deps.storage, &recipient)?;
    Ok(PositionResponse {
        recipient: position.recipient,
        aust_amount: position.aust_amount,
    })
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
