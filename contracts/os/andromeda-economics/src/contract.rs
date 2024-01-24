use crate::state::BALANCES;
use andromeda_std::ado_base::{AndromedaQuery, InstantiateMsg as BaseInstantiateMsg};
use andromeda_std::ado_contract::ADOContract;
use andromeda_std::amp::AndrAddr;

use andromeda_std::error::{from_semver, ContractError};
use andromeda_std::os::aos_querier::AOSQuerier;
use andromeda_std::os::economics::{Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
#[allow(unused_imports)]
use cosmwasm_std::{
    attr, coin, ensure, entry_point, from_json, to_json_binary, Addr, BankMsg, Binary, CosmosMsg,
    Deps, DepsMut, Empty, Env, MessageInfo, Response, Storage, SubMsg, Uint128, WasmMsg,
};
use cw2::{get_contract_version, set_contract_version};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use semver::Version;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-economics";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "economics".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            operators: None,
            kernel_address: msg.kernel_address,
            owner: msg.owner,
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
        ExecuteMsg::Deposit { address } => execute_deposit_native(deps, info, address),
        ExecuteMsg::PayFee { payee, action } => execute_pay_fee(deps, env, info, payee, action),
        ExecuteMsg::Withdraw { amount, asset } => {
            execute_withdraw_native(deps, info, amount, asset)
        }
        ExecuteMsg::Receive(cw20msg) => cw20_receive(deps, env, info, cw20msg),
        ExecuteMsg::WithdrawCW20 { amount, asset } => {
            execute_withdraw_cw20(deps, info, amount, asset)
        }
    }
}

pub fn cw20_receive(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let sender = deps.api.addr_validate(&msg.sender)?;
    let amount = msg.amount;

    match from_json::<Cw20HookMsg>(&msg.msg)? {
        Cw20HookMsg::Deposit { address } => {
            execute_cw20_deposit(deps, info, sender, amount, address)
        }
    }
}

pub fn execute_cw20_deposit(
    deps: DepsMut,
    info: MessageInfo,
    sender: Addr,
    amount: Uint128,
    address: Option<AndrAddr>,
) -> Result<Response, ContractError> {
    ensure!(
        amount > Uint128::zero(),
        ContractError::InvalidFunds {
            msg: "Cannot send 0 amount to deposit".to_string()
        }
    );
    let token_address = info.sender;
    let resp = Response::default().add_attributes(vec![
        attr("action", "receive"),
        attr("sender", sender.to_string()),
        attr("amount", amount.to_string()),
        attr("token_address", token_address.to_string()),
    ]);

    let sender = if let Some(address) = address {
        address.get_raw_address(&deps.as_ref())?
    } else {
        sender
    };

    let balance = BALANCES
        .load(deps.storage, (sender.clone(), token_address.to_string()))
        .unwrap_or_default();

    BALANCES.save(
        deps.storage,
        (sender, token_address.to_string()),
        &(balance + amount),
    )?;

    Ok(resp)
}

pub fn execute_deposit_native(
    deps: DepsMut,
    info: MessageInfo,
    address: Option<AndrAddr>,
) -> Result<Response, ContractError> {
    ensure!(!info.funds.is_empty(), ContractError::CoinNotFound {});

    let addr = address
        .unwrap_or(AndrAddr::from_string(info.sender.to_string()))
        .get_raw_address(&deps.as_ref())?;

    let mut resp = Response::default().add_attributes(vec![
        attr("action", "deposit"),
        attr("depositee", info.sender.to_string()),
        attr("recipient", addr.to_string()),
    ]);

    for funds in info.funds {
        let balance = BALANCES
            .load(
                deps.as_ref().storage,
                (addr.clone(), funds.denom.to_string()),
            )
            .unwrap_or_default();

        BALANCES.save(
            deps.storage,
            (addr.clone(), funds.denom.to_string()),
            &(balance + funds.amount),
        )?;

        resp = resp.add_attribute(
            "deposited_funds",
            format!("{}{}", funds.amount, funds.denom),
        );
    }

    Ok(resp)
}

pub(crate) fn spend_balance(
    storage: &mut dyn Storage,
    addr: &Addr,
    asset: String,
    amount: Uint128,
) -> Result<Uint128, ContractError> {
    let balance = BALANCES
        .load(storage, (addr.clone(), asset.to_string()))
        .unwrap_or_default();

    let remainder = if amount > balance {
        amount - balance
    } else {
        Uint128::zero()
    };
    let post_balance = if balance > amount {
        balance - amount
    } else {
        Uint128::zero()
    };

    BALANCES.save(storage, (addr.clone(), asset), &post_balance)?;

    Ok(remainder)
}

/// Charges a fee depending on the sending ADO and the action being performed.
/// Sender must be an ADO contract else this will error.
///
/// Fees are charged in the following order:
/// 1. ADO
/// 2. App
/// 3. Payee
fn execute_pay_fee(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    payee: Addr,
    action: String,
) -> Result<Response, ContractError> {
    let mut resp = Response::default();

    resp.attributes = vec![
        attr("action", action.clone()),
        attr("sender", info.sender.to_string()),
        attr("payee", payee.to_string()),
    ];

    let contract_info = deps.querier.query_wasm_contract_info(info.sender.clone());
    if let Ok(contract_info) = contract_info {
        let code_id = contract_info.code_id;
        let adodb_addr = ADOContract::default().get_adodb_address(deps.storage, &deps.querier)?;
        let ado_type = AOSQuerier::ado_type_getter(&deps.querier, &adodb_addr, code_id)?;
        if ado_type.is_none() {
            // Not an ADO
            return Ok(resp);
        }

        let ado_type = ado_type.unwrap();
        let fee = AOSQuerier::action_fee_getter(
            &deps.querier,
            &adodb_addr,
            ado_type.as_str(),
            action.as_str(),
        )?;

        match fee {
            // No fee
            None => Ok(resp),
            Some(fee) => {
                let asset_string = fee.asset.to_string();
                let asset = asset_string.split(':').last().unwrap();

                // Charge ADO first
                let mut remainder =
                    spend_balance(deps.storage, &info.sender, asset.to_string(), fee.amount)?;

                // Next charge the app
                if remainder > Uint128::zero() {
                    let app_contract = deps.querier.query_wasm_smart::<Option<Addr>>(
                        &info.sender,
                        &AndromedaQuery::AppContract {},
                    )?;
                    remainder = if let Some(app_contract) = app_contract {
                        spend_balance(deps.storage, &app_contract, asset.to_string(), remainder)?
                    } else {
                        remainder
                    };
                }

                // Next charge the payee
                if remainder > Uint128::zero() {
                    remainder = spend_balance(deps.storage, &payee, asset.to_string(), remainder)?;
                }

                // If balance remaining then not enough funds to pay fee
                ensure!(
                    remainder == Uint128::zero(),
                    ContractError::InsufficientFunds {}
                );

                let recipient = if let Some(receiver) = fee.receiver {
                    receiver
                } else {
                    let publisher = AOSQuerier::ado_publisher_getter(
                        &deps.querier,
                        &adodb_addr,
                        ado_type.as_str(),
                    )?;
                    deps.api.addr_validate(&publisher)?
                };

                let receiver_balance = BALANCES
                    .load(
                        deps.as_ref().storage,
                        (recipient.clone(), asset.to_string()),
                    )
                    .unwrap_or_default();
                BALANCES.save(
                    deps.storage,
                    (recipient.clone(), asset.to_string()),
                    &(receiver_balance + fee.amount),
                )?;

                resp = resp
                    .add_attribute("paid_fee", format!("{}{}", fee.amount, fee.asset))
                    .add_attribute("fee_recipient", recipient.to_string());
                Ok(resp)
            }
        }
    } else {
        // Not a contract
        Err(ContractError::InvalidSender {})
    }
}

fn execute_withdraw_native(
    deps: DepsMut,
    info: MessageInfo,
    amount: Option<Uint128>,
    asset: String,
) -> Result<Response, ContractError> {
    let mut resp = Response::default();

    let balance = BALANCES
        .load(deps.storage, (info.sender.clone(), asset.to_string()))
        .unwrap_or_default();

    let amount = if let Some(amount) = amount {
        amount
    } else {
        balance
    };

    ensure!(
        balance >= amount && !balance.is_zero(),
        ContractError::InsufficientFunds {}
    );

    spend_balance(deps.storage, &info.sender, asset.clone(), amount)?;

    let bank_msg = BankMsg::Send {
        to_address: info.sender.clone().into(),
        amount: vec![coin(amount.u128(), asset)],
    };
    let cosmos_msg: CosmosMsg<Empty> = CosmosMsg::Bank(bank_msg);

    resp.attributes = vec![
        attr("action", "withdraw"),
        attr("sender", info.sender.to_string()),
        attr("amount", amount),
    ];

    resp = resp.add_message(cosmos_msg);

    Ok(resp)
}

pub(crate) fn cw20_withdraw_msg(
    amount: Uint128,
    asset: impl Into<String>,
    recipient: impl Into<String>,
) -> SubMsg {
    let exec_msg = Cw20ExecuteMsg::Transfer {
        recipient: recipient.into(),
        amount,
    };

    SubMsg::reply_on_error(
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: asset.into(),
            msg: to_json_binary(&exec_msg).unwrap(),
            funds: vec![],
        }),
        999,
    )
}

fn execute_withdraw_cw20(
    deps: DepsMut,
    info: MessageInfo,
    amount: Option<Uint128>,
    asset: String,
) -> Result<Response, ContractError> {
    let mut resp = Response::default();

    let balance = BALANCES
        .load(deps.storage, (info.sender.clone(), asset.to_string()))
        .unwrap_or_default();

    let amount = if let Some(amount) = amount {
        amount
    } else {
        balance
    };

    ensure!(
        balance >= amount && !balance.is_zero(),
        ContractError::InsufficientFunds {}
    );

    spend_balance(deps.storage, &info.sender, asset.clone(), amount)?;

    let msg = cw20_withdraw_msg(amount, asset, info.sender.clone());

    resp.attributes = vec![
        attr("action", "withdraw"),
        attr("sender", info.sender.to_string()),
        attr("amount", amount),
    ];

    resp = resp.add_submessage(msg);

    Ok(resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // New version
    let version: Version = CONTRACT_VERSION.parse().map_err(from_semver)?;

    // Old version
    let stored = get_contract_version(deps.storage)?;
    let storage_version: Version = stored.version.parse().map_err(from_semver)?;

    let contract = ADOContract::default();

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

    // Update the ADOContract's version
    contract.execute_update_version(deps)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::Balance { address, asset } => {
            Ok(to_json_binary(&query_balance(_deps, address, asset)?)?)
        }
    }
}

fn query_balance(deps: Deps, address: AndrAddr, asset: String) -> Result<Uint128, ContractError> {
    let addr = address.get_raw_address(&deps)?;
    let balance = BALANCES
        .load(deps.storage, (addr, asset))
        .unwrap_or_default();
    Ok(balance)
}
