#[cfg(not(feature = "library"))]
use crate::state::{Config, CONFIG};
use andromeda_modules::rates::{
    calculate_fee, ExecuteMsg, InstantiateMsg, MigrateMsg, PaymentAttribute, PaymentsResponse,
    QueryMsg, RateInfo,
};
use andromeda_std::{
    ado_base::{
        hooks::{AndromedaHook, OnFundsTransferResponse},
        InstantiateMsg as BaseInstantiateMsg,
    },
    ado_contract::ADOContract,
    common::{context::ExecuteContext, deduct_funds, encode_binary, Funds},
    error::{from_semver, ContractError},
};

use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, coin, ensure, Binary, Coin, Deps, DepsMut, Env, Event, MessageInfo, Response, SubMsg,
};
use cw2::{get_contract_version, set_contract_version};
use cw20::Cw20Coin;
use cw_utils::nonpayable;
use semver::Version;
// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-rates";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let config = Config { rates: msg.rates };
    CONFIG.save(deps.storage, &config)?;

    let inst_resp = ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        &deps.querier,
        info,
        BaseInstantiateMsg {
            ado_type: CONTRACT_NAME.to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            kernel_address: msg.kernel_address,
            owner: msg.owner,
        },
    )?;

    Ok(inst_resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let ctx = ExecuteContext::new(deps, info, env);

    match msg {
        ExecuteMsg::AMPReceive(pkt) => {
            ADOContract::default().execute_amp_receive(ctx, pkt, handle_execute)
        }
        _ => handle_execute(ctx, msg),
    }
}

pub fn handle_execute(ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateRates { rates } => execute_update_rates(ctx, rates),
        _ => ADOContract::default().execute(ctx, msg),
    }
}

fn execute_update_rates(
    ctx: ExecuteContext,
    rates: Vec<RateInfo>,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, info, .. } = ctx;
    nonpayable(&info)?;

    ensure!(
        ADOContract::default().is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );
    let mut config = CONFIG.load(deps.storage)?;
    config.rates = rates;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attributes(vec![attr("action", "update_rates")]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // New version
    let version: Version = CONTRACT_VERSION.parse().map_err(from_semver)?;

    // Old version
    let stored = get_contract_version(deps.storage)?;
    let storage_version: Version = stored.version.parse().map_err(from_semver)?;

    ensure!(
        stored.contract == CONTRACT_NAME,
        ContractError::CannotMigrate {
            previous_contract: stored.contract,
        }
    );

    // New version has to be newer/greater than the old version
    ensure!(
        storage_version < version,
        ContractError::CannotMigrate {
            previous_contract: stored.version,
        }
    );

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrHook(msg) => handle_andromeda_hook(deps, msg),
        QueryMsg::Payments {} => encode_binary(&query_payments(deps)?),
        _ => ADOContract::default().query(deps, env, msg),
    }
}

fn handle_andromeda_hook(deps: Deps, msg: AndromedaHook) -> Result<Binary, ContractError> {
    match msg {
        AndromedaHook::OnFundsTransfer { amount, .. } => {
            encode_binary(&query_deducted_funds(deps, amount)?)
        }
        _ => Ok(encode_binary(&None::<Response>)?),
    }
}

fn query_payments(deps: Deps) -> Result<PaymentsResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let rates = config.rates;

    Ok(PaymentsResponse { payments: rates })
}

//NOTE Currently set as pub for testing
pub fn query_deducted_funds(
    deps: Deps,
    funds: Funds,
) -> Result<OnFundsTransferResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut msgs: Vec<SubMsg> = vec![];
    let mut events: Vec<Event> = vec![];
    let (coin, is_native): (Coin, bool) = match funds {
        Funds::Native(coin) => (coin, true),
        Funds::Cw20(cw20_coin) => (coin(cw20_coin.amount.u128(), cw20_coin.address), false),
    };
    let mut leftover_funds = vec![coin.clone()];
    for rate_info in config.rates.iter() {
        let event_name = if rate_info.is_additive {
            "tax"
        } else {
            "royalty"
        };
        let mut event = Event::new(event_name);
        if let Some(desc) = &rate_info.description {
            event = event.add_attribute("description", desc);
        }
        let rate = rate_info.rate.validate(&deps.querier)?;
        let fee = calculate_fee(rate, &coin)?;
        for receiver in rate_info.recipients.iter() {
            if !rate_info.is_additive {
                deduct_funds(&mut leftover_funds, &fee)?;
                event = event.add_attribute("deducted", fee.to_string());
            }
            event = event.add_attribute(
                "payment",
                PaymentAttribute {
                    receiver: receiver.get_addr(),
                    amount: fee.clone(),
                }
                .to_string(),
            );
            let msg = if is_native {
                receiver.generate_direct_msg(&deps, vec![fee.clone()])?
            } else {
                receiver.generate_msg_cw20(
                    &deps,
                    Cw20Coin {
                        amount: fee.amount,
                        address: fee.denom.to_string(),
                    },
                )?
            };
            msgs.push(msg);
        }
        events.push(event);
    }
    Ok(OnFundsTransferResponse {
        msgs,
        leftover_funds: if is_native {
            Funds::Native(leftover_funds[0].clone())
        } else {
            Funds::Cw20(Cw20Coin {
                amount: leftover_funds[0].amount,
                address: coin.denom,
            })
        },
        events,
    })
}
