use crate::state::{
    read_sale_infos, sale_infos, SaleInfo, TokenSaleState, NEXT_SALE_ID, TOKEN_SALE_STATE,
};
use ado_base::state::ADOContract;
use andromeda_non_fungible_tokens::marketplace::{
    Cw721HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, SaleIdsResponse,
    SaleStateResponse, Status,
};
use common::{
    ado_base::{hooks::AndromedaHook, AndromedaMsg, InstantiateMsg as BaseInstantiateMsg},
    encode_binary,
    error::ContractError,
    rates::get_tax_amount,
    Funds,
};
use cosmwasm_std::{
    attr, ensure, entry_point, from_binary, has_coins, Api, BankMsg, Binary, Coin, CosmosMsg, Deps,
    DepsMut, Env, MessageInfo, QuerierWrapper, QueryRequest, Response, StdError, Storage, SubMsg,
    Uint128, WasmMsg, WasmQuery,
};
use cw2::{get_contract_version, set_contract_version};
use cw721::{Cw721ExecuteMsg, Cw721QueryMsg, Cw721ReceiveMsg, OwnerOfResponse};
use cw_utils::nonpayable;
use semver::Version;

const CONTRACT_NAME: &str = "crates.io:andromeda-marketplace";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    NEXT_SALE_ID.save(deps.storage, &Uint128::from(1u128))?;
    ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "marketplace".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            operators: None,
            modules: msg.modules,
            primitive_contract: None,
            kernel_address: msg.kernel_address,
        },
    )
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();

    // Do this before the hooks get fired off to ensure that there are no errors from the app
    // address not being fully setup yet.
    if let ExecuteMsg::AndrReceive(andr_msg) = msg.clone() {
        if let AndromedaMsg::UpdateAppContract { address } = andr_msg {
            return contract.execute_update_app_contract(deps, info, address, None);
        } else if let AndromedaMsg::UpdateOwner { address } = andr_msg {
            return contract.execute_update_owner(deps, info, address);
        }
    }

    contract.module_hook::<Response>(
        deps.storage,
        deps.api,
        deps.querier,
        AndromedaHook::OnExecute {
            sender: info.sender.to_string(),
            payload: encode_binary(&msg)?,
        },
    )?;

    match msg {
        ExecuteMsg::AndrReceive(msg) => {
            ADOContract::default().execute(deps, env, info, msg, execute)
        }
        ExecuteMsg::ReceiveNft(msg) => handle_receive_cw721(deps, env, info, msg),
        ExecuteMsg::UpdateSale {
            token_id,
            token_address,
            coin_denom,
            price,
        } => execute_update_sale(deps, env, info, token_id, token_address, price, coin_denom),
        ExecuteMsg::Buy {
            token_id,
            token_address,
        } => execute_buy(deps, env, info, token_id, token_address),
        ExecuteMsg::CancelSale {
            token_id,
            token_address,
        } => execute_cancel(deps, env, info, token_id, token_address),
    }
}

fn handle_receive_cw721(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: Cw721ReceiveMsg,
) -> Result<Response, ContractError> {
    match from_binary(&msg.msg)? {
        Cw721HookMsg::StartSale { price, coin_denom } => execute_start_sale(
            deps,
            env,
            msg.sender,
            msg.token_id,
            info.sender.to_string(),
            price,
            coin_denom,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_start_sale(
    deps: DepsMut,
    _env: Env,
    sender: String,
    token_id: String,
    token_address: String,
    price: Uint128,
    coin_denom: String,
) -> Result<Response, ContractError> {
    // Price can't be zero
    ensure!(price > Uint128::zero(), ContractError::InvalidZeroAmount {});

    let sale_id = get_and_increment_next_sale_id(deps.storage, &token_id, &token_address)?;

    TOKEN_SALE_STATE.save(
        deps.storage,
        sale_id.u128(),
        &TokenSaleState {
            coin_denom: coin_denom.clone(),
            sale_id,
            owner: sender,
            token_id: token_id.clone(),
            token_address: token_address.clone(),
            price,
            status: Status::Open,
        },
    )?;
    Ok(Response::new().add_attributes(vec![
        attr("action", "start_sale"),
        attr("status", "Open"),
        attr("coin_denom", coin_denom),
        attr("price", price),
        attr("sale_id", sale_id.to_string()),
        attr("token_id", token_id),
        attr("token_address", token_address),
    ]))
}

#[allow(clippy::too_many_arguments)]
fn execute_update_sale(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    token_id: String,
    token_address: String,
    price: Uint128,
    coin_denom: String,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let mut token_sale_state =
        get_existing_token_sale_state(deps.storage, &token_id, &token_address)?;
    // Only token owner is authorized to update the sale

    ensure!(
        info.sender == token_sale_state.owner,
        ContractError::Unauthorized {}
    );

    // New price can't be zero
    ensure!(price > Uint128::zero(), ContractError::InvalidZeroAmount {});

    token_sale_state.price = price;
    token_sale_state.coin_denom = coin_denom.clone();
    TOKEN_SALE_STATE.save(
        deps.storage,
        token_sale_state.sale_id.u128(),
        &token_sale_state,
    )?;
    Ok(Response::new().add_attributes(vec![
        attr("action", "update_sale"),
        attr("coin_denom", coin_denom),
        attr("price", price),
        attr("sale_id", token_sale_state.sale_id.to_string()),
        attr("token_id", token_id),
        attr("token_address", token_address),
    ]))
}

fn execute_buy(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
    token_address: String,
) -> Result<Response, ContractError> {
    let mut token_sale_state =
        get_existing_token_sale_state(deps.storage, &token_id, &token_address)?;

    // Sale needs to be open
    ensure!(
        token_sale_state.status == Status::Open,
        ContractError::SaleNotOpen {}
    );

    // The owner can't buy his own NFT
    ensure!(
        token_sale_state.owner != info.sender,
        ContractError::TokenOwnerCannotBuy {}
    );

    // Only one coin can be sent
    ensure!(
        info.funds.len() == 1,
        ContractError::InvalidFunds {
            msg: "Sales ensure! exactly one coin to be sent.".to_string(),
        }
    );

    let token_owner = query_owner_of(
        deps.querier,
        token_sale_state.token_address.clone(),
        token_id.clone(),
    )?
    .owner;
    ensure!(
        // If this is false then the token is no longer held by the contract so the token has been
        // claimed.
        token_owner == env.contract.address,
        ContractError::SaleAlreadyConducted {}
    );

    let coin_denom = token_sale_state.coin_denom.clone();
    let payment: &Coin = &info.funds[0];

    // Make sure funds are equal to the price and in the correct denomination
    ensure!(
        payment.denom == coin_denom,
        ContractError::InvalidFunds {
            msg: format!("No {coin_denom} assets are provided to sale"),
        }
    );
    ensure!(
        payment.amount >= token_sale_state.price,
        ContractError::InsufficientFunds {}
    );

    let key = token_sale_state.sale_id.u128();

    // Change sale status from Open to Executed
    token_sale_state.status = Status::Executed;

    TOKEN_SALE_STATE.save(deps.storage, key, &token_sale_state)?;

    // Calculate the funds to be received after tax
    let after_tax_payment = purchase_token(
        deps.storage,
        deps.api,
        &deps.querier,
        &info,
        token_sale_state.clone(),
    )?;

    Ok(Response::new()
        .add_submessages(after_tax_payment.1)
        // Send funds to the original owner.
        .add_message(CosmosMsg::Bank(BankMsg::Send {
            to_address: token_sale_state.owner,
            amount: vec![after_tax_payment.0],
        }))
        // Send NFT to buyer.
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: token_sale_state.token_address.clone(),
            msg: encode_binary(&Cw721ExecuteMsg::TransferNft {
                recipient: info.sender.to_string(),
                token_id: token_id.clone(),
            })?,
            funds: vec![],
        }))
        .add_attribute("action", "buy")
        .add_attribute("token_id", token_id)
        .add_attribute("token_contract", token_sale_state.token_address)
        .add_attribute("recipient", info.sender.to_string())
        .add_attribute("sale_id", token_sale_state.sale_id))
}

fn execute_cancel(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    token_id: String,
    token_address: String,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let mut token_sale_state =
        get_existing_token_sale_state(deps.storage, &token_id, &token_address)?;

    ensure!(
        info.sender == token_sale_state.owner,
        ContractError::Unauthorized {}
    );

    // Sale needs to be open to be cancelled
    ensure!(
        token_sale_state.status == Status::Open,
        ContractError::SaleNotOpen {}
    );

    let messages: Vec<CosmosMsg> = vec![CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: token_sale_state.token_address.clone(),
        msg: encode_binary(&Cw721ExecuteMsg::TransferNft {
            recipient: info.sender.to_string(),
            token_id,
        })?,
        funds: vec![],
    })];

    token_sale_state.status = Status::Cancelled;
    TOKEN_SALE_STATE.save(
        deps.storage,
        token_sale_state.sale_id.u128(),
        &token_sale_state,
    )?;

    Ok(Response::new().add_messages(messages))
}

fn purchase_token(
    storage: &mut dyn Storage,
    api: &dyn Api,
    querier: &QuerierWrapper,
    info: &MessageInfo,
    state: TokenSaleState,
) -> Result<(Coin, Vec<SubMsg>), ContractError> {
    let total_cost = Coin::new(state.price.u128(), state.coin_denom.clone());

    let mut total_tax_amount = Uint128::zero();

    let (msgs, _events, remainder) = ADOContract::default().on_funds_transfer(
        storage,
        api,
        querier,
        info.sender.to_string(),
        Funds::Native(total_cost),
        encode_binary(&"")?,
    )?;

    let remaining_amount = remainder.try_get_coin()?;

    let tax_amount = get_tax_amount(&msgs, state.price, remaining_amount.amount);

    // Calculate total tax
    total_tax_amount += tax_amount;

    let required_payment = Coin {
        denom: state.coin_denom.clone(),
        amount: state.price + total_tax_amount,
    };
    ensure!(
        has_coins(&info.funds, &required_payment),
        ContractError::InsufficientFunds {}
    );

    let after_tax_payment = Coin {
        denom: state.coin_denom,
        amount: remaining_amount.amount,
    };
    Ok((after_tax_payment, msgs))
}

fn get_existing_token_sale_state(
    storage: &dyn Storage,
    token_id: &str,
    token_address: &str,
) -> Result<TokenSaleState, ContractError> {
    let key = token_id.to_owned() + token_address;
    let latest_sale_id: Uint128 = match sale_infos().may_load(storage, &key)? {
        None => return Err(ContractError::SaleDoesNotExist {}),
        Some(sale_info) => *sale_info.last().unwrap(),
    };
    let token_sale_state = TOKEN_SALE_STATE.load(storage, latest_sale_id.u128())?;

    Ok(token_sale_state)
}

fn get_and_increment_next_sale_id(
    storage: &mut dyn Storage,
    token_id: &str,
    token_address: &str,
) -> Result<Uint128, ContractError> {
    let next_sale_id = NEXT_SALE_ID.load(storage)?;
    let incremented_next_sale_id = next_sale_id.checked_add(Uint128::from(1u128))?;
    NEXT_SALE_ID.save(storage, &incremented_next_sale_id)?;

    let key = token_id.to_owned() + token_address;

    let mut sale_info = sale_infos().load(storage, &key).unwrap_or_default();
    sale_info.push(next_sale_id);
    if sale_info.token_address.is_empty() {
        sale_info.token_address = token_address.to_owned();
        sale_info.token_id = token_id.to_owned();
    }
    sale_infos().save(storage, &key, &sale_info)?;
    Ok(next_sale_id)
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrQuery(msg) => ADOContract::default().query(deps, env, msg, query),
        QueryMsg::LatestSaleState {
            token_id,
            token_address,
        } => encode_binary(&query_latest_sale_state(deps, token_id, token_address)?),
        QueryMsg::SaleState { sale_id } => encode_binary(&query_sale_state(deps, sale_id)?),
        QueryMsg::SaleIds {
            token_id,
            token_address,
        } => encode_binary(&query_sale_ids(deps, token_id, token_address)?),
        QueryMsg::SaleInfosForAddress {
            token_address,
            start_after,
            limit,
        } => encode_binary(&query_sale_infos_for_address(
            deps,
            token_address,
            start_after,
            limit,
        )?),
    }
}

fn query_sale_ids(
    deps: Deps,
    token_id: String,
    token_address: String,
) -> Result<SaleIdsResponse, ContractError> {
    let key = token_id + &token_address;
    let sale_info = sale_infos().may_load(deps.storage, &key)?;
    if let Some(sale_info) = sale_info {
        return Ok(SaleIdsResponse {
            sale_ids: sale_info.sale_ids,
        });
    }
    Ok(SaleIdsResponse { sale_ids: vec![] })
}

pub fn query_sale_infos_for_address(
    deps: Deps,
    token_address: String,
    start_after: Option<String>,
    limit: Option<u64>,
) -> Result<Vec<SaleInfo>, ContractError> {
    read_sale_infos(deps.storage, token_address, start_after, limit)
}

fn query_latest_sale_state(
    deps: Deps,
    token_id: String,
    token_address: String,
) -> Result<SaleStateResponse, ContractError> {
    let token_sale_state_result =
        get_existing_token_sale_state(deps.storage, &token_id, &token_address);
    if let Ok(token_sale_state) = token_sale_state_result {
        return Ok(token_sale_state.into());
    }
    Err(ContractError::SaleDoesNotExist {})
}

fn query_sale_state(deps: Deps, sale_id: Uint128) -> Result<SaleStateResponse, ContractError> {
    let token_sale_state = TOKEN_SALE_STATE.load(deps.storage, sale_id.u128())?;
    Ok(token_sale_state.into())
}

fn query_owner_of(
    querier: QuerierWrapper,
    token_addr: String,
    token_id: String,
) -> Result<OwnerOfResponse, ContractError> {
    let res: OwnerOfResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: token_addr,
        msg: encode_binary(&Cw721QueryMsg::OwnerOf {
            token_id,
            include_expired: None,
        })?,
    }))?;

    Ok(res)
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

fn from_semver(err: semver::Error) -> StdError {
    StdError::generic_err(format!("Semver: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock_querier::{
        mock_dependencies_custom, MOCK_RATES_CONTRACT, MOCK_TOKEN_ADDR, MOCK_TOKEN_OWNER,
        MOCK_UNCLAIMED_TOKEN,
    };
    use crate::state::SaleInfo;
    use andromeda_non_fungible_tokens::marketplace::{Cw721HookMsg, ExecuteMsg, InstantiateMsg};

    use common::ado_base::modules::{Module, RATES};
    use common::app::AndrAddress;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coin, coins};

    fn start_sale(deps: DepsMut) {
        let hook_msg = Cw721HookMsg::StartSale {
            coin_denom: "uusd".to_string(),
            price: Uint128::new(100),
        };
        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: MOCK_TOKEN_OWNER.to_owned(),
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            msg: encode_binary(&hook_msg).unwrap(),
        });
        let env = mock_env();

        let info = mock_info(MOCK_TOKEN_ADDR, &[]);
        let _res = execute(deps, env, info, msg).unwrap();
    }

    fn assert_sale_created(deps: Deps) {
        assert_eq!(
            TokenSaleState {
                coin_denom: "uusd".to_string(),
                sale_id: 1u128.into(),
                owner: MOCK_TOKEN_OWNER.to_string(),
                token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
                token_address: MOCK_TOKEN_ADDR.to_owned(),
                status: Status::Open,
                price: Uint128::new(100)
            },
            TOKEN_SALE_STATE.load(deps.storage, 1u128).unwrap()
        );

        assert_eq!(
            SaleInfo {
                sale_ids: vec![Uint128::from(1u128)],
                token_address: MOCK_TOKEN_ADDR.to_owned(),
                token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            },
            sale_infos()
                .load(
                    deps.storage,
                    &(MOCK_UNCLAIMED_TOKEN.to_owned() + MOCK_TOKEN_ADDR)
                )
                .unwrap()
        );
    }

    #[test]
    fn test_sale_instantiate() {
        let owner = "creator";
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(owner, &[]);
        let msg = InstantiateMsg {
            modules: None,
            kernel_address: None,
        };
        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn test_execute_buy_non_existing_sale() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info(MOCK_TOKEN_OWNER, &[]);
        let msg = InstantiateMsg {
            modules: None,
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        let msg = ExecuteMsg::Buy {
            token_id: MOCK_UNCLAIMED_TOKEN.to_string(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
        };
        let info = mock_info("buyer", &coins(100, "uusd"));
        let res = execute(deps.as_mut(), env, info, msg);
        assert_eq!(ContractError::SaleDoesNotExist {}, res.unwrap_err());
    }

    #[test]
    fn execute_buy_sale_not_open_already_bought() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info(MOCK_TOKEN_OWNER, &[]);
        let msg = InstantiateMsg {
            modules: None,
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        start_sale(deps.as_mut());
        assert_sale_created(deps.as_ref());

        let msg = ExecuteMsg::Buy {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
        };

        let info = mock_info("sender", &coins(100, "uusd".to_string()));
        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let msg = ExecuteMsg::Buy {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
        };

        let info = mock_info("sender", &coins(100, "uusd".to_string()));
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(err, ContractError::SaleNotOpen {})
    }

    #[test]
    fn execute_buy_sale_not_open_cancelled() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info(MOCK_TOKEN_OWNER, &[]);
        let msg = InstantiateMsg {
            modules: None,
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        start_sale(deps.as_mut());
        assert_sale_created(deps.as_ref());

        let msg = ExecuteMsg::CancelSale {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
        };

        let info = mock_info(MOCK_TOKEN_OWNER, &[]);
        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let msg = ExecuteMsg::Buy {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
        };
        let info = mock_info("sender", &coins(100, "uusd".to_string()));
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(err, ContractError::SaleNotOpen {})
    }

    #[test]
    fn execute_buy_token_owner_cannot_buy() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            modules: None,
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        start_sale(deps.as_mut());
        assert_sale_created(deps.as_ref());

        let msg = ExecuteMsg::Buy {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
        };

        let info = mock_info(MOCK_TOKEN_OWNER, &coins(100, "uusd".to_string()));
        let res = execute(deps.as_mut(), env, info, msg);
        assert_eq!(ContractError::TokenOwnerCannotBuy {}, res.unwrap_err());
    }

    // #[test]
    // fn execute_buy_whitelist() {
    //     let mut deps = mock_dependencies_custom(&[]);
    //     let env = mock_env();
    //     let info = mock_info("owner", &[]);
    //     let msg = InstantiateMsg {
    //     let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    //     start_sale(deps.as_mut(), Some(vec![Addr::unchecked("sender")]));
    //     assert_sale_created(deps.as_ref(), Some(vec![Addr::unchecked("sender")]));

    //     let msg = ExecuteMsg::Buy {
    //         token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
    //         token_address: MOCK_TOKEN_ADDR.to_string(),
    //     };

    //     let info = mock_info("not_sender", &coins(100, "uusd".to_string()));
    //     let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
    //     assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());

    //     let info = mock_info("sender", &coins(100, "uusd".to_string()));
    //     let _res = execute(deps.as_mut(), env, info, msg).unwrap();
    // }

    #[test]
    fn execute_buy_invalid_coins_sent() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            modules: None,
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        start_sale(deps.as_mut());
        assert_sale_created(deps.as_ref());

        let error = ContractError::InvalidFunds {
            msg: "Sales ensure! exactly one coin to be sent.".to_string(),
        };
        let msg = ExecuteMsg::Buy {
            token_id: MOCK_UNCLAIMED_TOKEN.to_string(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
        };

        // No coins sent
        let info = mock_info("sender", &[]);
        let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
        assert_eq!(error, res.unwrap_err());

        // Multiple coins sent
        let info = mock_info("sender", &[coin(100, "uusd"), coin(100, "uluna")]);
        let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
        assert_eq!(error, res.unwrap_err());

        // Invalid denom sent
        let info = mock_info("sender", &[coin(100, "uluna")]);
        let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
        assert_eq!(
            ContractError::InvalidFunds {
                msg: "No uusd assets are provided to sale".to_string(),
            },
            res.unwrap_err()
        );

        // Correct denom but empty
        let info = mock_info("sender", &[coin(0, "uusd")]);
        let res = execute(deps.as_mut(), env, info, msg);
        assert_eq!(ContractError::InsufficientFunds {}, res.unwrap_err());
    }

    #[test]
    fn execute_buy_works() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            modules: None,
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        start_sale(deps.as_mut());
        assert_sale_created(deps.as_ref());

        let msg = ExecuteMsg::Buy {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
        };

        let info = mock_info("someone", &coins(100, "uusd".to_string()));
        let _res = execute(deps.as_mut(), env, info, msg).unwrap();
    }

    #[test]
    fn execute_update_sale_unauthorized() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            modules: None,
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        start_sale(deps.as_mut());
        assert_sale_created(deps.as_ref());

        let msg = ExecuteMsg::UpdateSale {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
            price: Uint128::new(11),
            coin_denom: "juno".to_string(),
        };

        let info = mock_info("someone", &[]);
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {})
    }

    #[test]
    fn execute_update_sale_invalid_price() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            modules: None,
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        start_sale(deps.as_mut());
        assert_sale_created(deps.as_ref());

        let msg = ExecuteMsg::UpdateSale {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
            price: Uint128::zero(),
            coin_denom: "juno".to_string(),
        };

        let info = mock_info("owner", &[]);
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(err, ContractError::InvalidZeroAmount {})
    }

    #[test]
    fn execute_start_sale_invalid_price() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            modules: None,
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let hook_msg = Cw721HookMsg::StartSale {
            coin_denom: "uusd".to_string(),
            price: Uint128::zero(),
        };
        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: MOCK_TOKEN_OWNER.to_owned(),
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            msg: encode_binary(&hook_msg).unwrap(),
        });
        let env = mock_env();

        let info = mock_info(MOCK_TOKEN_ADDR, &[]);
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(err, ContractError::InvalidZeroAmount {})
    }

    #[test]
    fn execute_buy_with_tax_and_royalty_insufficient_funds() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let modules = vec![Module {
            module_type: RATES.to_owned(),
            address: AndrAddress {
                identifier: MOCK_RATES_CONTRACT.to_owned(),
            },
            is_mutable: false,
        }];
        let msg = InstantiateMsg {
            modules: Some(modules),
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        start_sale(deps.as_mut());
        assert_sale_created(deps.as_ref());

        let msg = ExecuteMsg::Buy {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
        };

        let info = mock_info("someone", &coins(100, "uusd".to_string()));
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::InsufficientFunds {})
    }

    #[test]
    fn execute_buy_with_tax_and_royalty_works() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let modules = vec![Module {
            module_type: RATES.to_owned(),
            address: AndrAddress {
                identifier: MOCK_RATES_CONTRACT.to_owned(),
            },
            is_mutable: false,
        }];
        let msg = InstantiateMsg {
            modules: Some(modules),
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        start_sale(deps.as_mut());
        assert_sale_created(deps.as_ref());

        let msg = ExecuteMsg::Buy {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
        };

        let info = mock_info("someone", &coins(150, "uusd".to_string()));
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        let expected: Vec<SubMsg<_>> = vec![
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "royalty_recipient".to_string(),
                amount: vec![coin(10, "uusd")],
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "tax_recipient".to_string(),
                amount: vec![coin(50, "uusd")],
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "owner".to_string(),
                amount: vec![coin(90, "uusd")],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_TOKEN_ADDR.to_string(),
                msg: encode_binary(&Cw721ExecuteMsg::TransferNft {
                    recipient: info.sender.to_string(),
                    token_id: MOCK_UNCLAIMED_TOKEN.to_string(),
                })
                .unwrap(),
                funds: vec![],
            })),
        ];
        assert_eq!(res.messages, expected)
    }
}
