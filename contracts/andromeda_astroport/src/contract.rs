use crate::state::{Config, CONFIG};
use andromeda_protocol::{
    astroport::{ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    communication::{encode_binary, parse_message, AndromedaMsg, AndromedaQuery, Recipient},
    error::ContractError,
    operators::{execute_update_operators, is_operator, query_is_operator, query_operators},
    ownership::{execute_update_owner, is_contract_owner, query_contract_owner, CONTRACT_OWNER},
    require,
    swapper::{query_token_balance, AssetInfo, SwapperCw20HookMsg, SwapperMsg},
    withdraw::{add_withdrawable_token, execute_withdraw},
};
use astroport::{
    asset::{Asset, AssetInfo as AstroportAssetInfo, PairInfo},
    pair::{
        Cw20HookMsg as PairCw20HookMsg, ExecuteMsg as AstroportPairExecuteMsg,
        QueryMsg as PairQueryMsg,
    },
    querier::query_pair_info,
    router::{
        Cw20HookMsg as AstroportRouterCw20HookMsg, ExecuteMsg as AstroportRouterExecuteMsg,
        SwapOperation,
    },
};
use cosmwasm_std::{
    attr, entry_point, from_binary, Addr, Api, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut,
    Env, MessageInfo, QuerierWrapper, QueryRequest, Response, StdResult, SubMsg, Uint128, WasmMsg,
    WasmQuery,
};
use cw2::{get_contract_version, set_contract_version};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use std::cmp;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda_astroport";
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
        astroport_factory_contract: deps.api.addr_validate(&msg.astroport_factory_contract)?,
        astroport_router_contract: deps.api.addr_validate(&msg.astroport_router_contract)?,
        astroport_staking_contract: deps.api.addr_validate(&msg.astroport_staking_contract)?,
    };
    // Astro token is obtained from staking LP tokens.
    add_withdrawable_token(
        deps.storage,
        &msg.astroport_token_contract,
        &AssetInfo::Token {
            contract_addr: deps.api.addr_validate(&msg.astroport_token_contract)?,
        },
    )?;
    CONFIG.save(deps.storage, &config)?;
    CONTRACT_OWNER.save(deps.storage, &info.sender)?;
    Ok(Response::new().add_attributes(vec![
        attr("action", "instantiate"),
        attr("type", "astroport"),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    match msg {
        ExecuteMsg::ProvideLiquidity {
            assets,
            slippage_tolerance,
            auto_stake,
        } => execute_provide_liquidity(deps, env, info, assets, slippage_tolerance, auto_stake),
        ExecuteMsg::WithdrawLiquidity {
            pair_address,
            amount,
            recipient,
        } => execute_withdraw_liquidity(deps, env, info, pair_address, amount, recipient),
        ExecuteMsg::AndrReceive(msg) => execute_andr_receive(deps, env, info, msg),
        ExecuteMsg::Swapper(msg) => handle_swapper_msg(deps, info, msg),
        ExecuteMsg::Receive(msg) => receive_cw20(deps, info, msg),
        ExecuteMsg::AstroportFactoryExecuteMsg(msg) => execute_astroport_msg(
            info.funds,
            config.astroport_factory_contract.to_string(),
            encode_binary(&msg)?,
        ),
        ExecuteMsg::AstroportRouterExecuteMsg(msg) => execute_astroport_msg(
            info.funds,
            config.astroport_router_contract.to_string(),
            encode_binary(&msg)?,
        ),
        ExecuteMsg::AstroportStakingExecuteMsg(msg) => execute_astroport_msg(
            info.funds,
            config.astroport_staking_contract.to_string(),
            encode_binary(&msg)?,
        ),
        ExecuteMsg::UpdateConfig {
            astroport_factory_contract,
            astroport_router_contract,
            astroport_staking_contract,
        } => execute_update_config(
            deps,
            info,
            astroport_factory_contract,
            astroport_router_contract,
            astroport_staking_contract,
        ),
    }
}

fn execute_provide_liquidity(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    assets: [Asset; 2],
    slippage_tolerance: Option<Decimal>,
    auto_stake: Option<bool>,
) -> Result<Response, ContractError> {
    let sender = info.sender.as_str();
    require(
        is_contract_owner(deps.storage, sender)? || is_operator(deps.storage, sender)?,
        ContractError::Unauthorized {},
    )?;
    let config = CONFIG.load(deps.storage)?;
    let pair = query_pair_info(
        &deps.querier,
        config.astroport_factory_contract,
        &[assets[0].info.clone(), assets[1].info.clone()],
    )?;

    let pooled_assets = pair.query_pools(&deps.querier, pair.contract_addr.clone())?;
    let (assets, sub_messages) = verify_asset_ratio(
        deps.api,
        pooled_assets,
        assets,
        Recipient::Addr(info.sender.to_string()),
    )?;

    // In the case where we want to witdraw the LP token.
    add_withdrawable_token(
        deps.storage,
        &pair.liquidity_token.to_string(),
        &AssetInfo::Token {
            contract_addr: pair.liquidity_token,
        },
    )?;
    let mut messages: Vec<CosmosMsg> = vec![];
    for asset in assets.iter() {
        if let AstroportAssetInfo::Token { contract_addr } = &asset.info {
            messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                // User needs to allow this contract to transfer tokens.
                msg: encode_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: info.sender.to_string(),
                    recipient: env.contract.address.to_string(),
                    amount: asset.amount,
                })?,
                funds: vec![],
            }));
            messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                // Allow the pair address to transfer from here.
                msg: encode_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                    spender: pair.contract_addr.to_string(),
                    amount: asset.amount,
                    expires: None,
                })?,
                funds: vec![],
            }));
        }
    }

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: pair.contract_addr.to_string(),
        msg: encode_binary(&AstroportPairExecuteMsg::ProvideLiquidity {
            assets,
            slippage_tolerance,
            auto_stake,
            // Not strictly neccessary but to be explicit.
            receiver: Some(env.contract.address.to_string()),
        })?,
        funds: info.funds,
    }));
    Ok(Response::new()
        .add_submessages(sub_messages)
        .add_messages(messages)
        .add_attribute("action", "provide_liquidity"))
}

fn execute_withdraw_liquidity(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    pair_address: String,
    amount: Option<Uint128>,
    recipient: Option<Recipient>,
) -> Result<Response, ContractError> {
    let recipient = recipient.unwrap_or_else(|| Recipient::Addr(info.sender.to_string()));
    let pair_info = query_pair_given_address(&deps.querier, pair_address)?;
    let total_amount = query_token_balance(
        &deps.querier,
        pair_info.liquidity_token.clone(),
        env.contract.address,
    )?;
    let withdraw_amount = match amount {
        None => total_amount,
        Some(amount) => cmp::min(amount, total_amount),
    };

    // This represents how many underlying tokens will be withdrawn.
    let share = query_pair_share(
        &deps.querier,
        pair_info.contract_addr.to_string(),
        withdraw_amount,
    )?;
    let mut withdraw_messages: Vec<SubMsg> = vec![];
    for asset in share.into_iter() {
        withdraw_messages.push(recipient.generate_msg_from_asset(deps.api, asset)?);
    }

    Ok(Response::new()
        .add_submessage(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: pair_info.liquidity_token.to_string(),
            msg: encode_binary(&Cw20ExecuteMsg::Send {
                contract: pair_info.contract_addr.to_string(),
                amount: withdraw_amount,
                msg: encode_binary(&PairCw20HookMsg::WithdrawLiquidity {})?,
            })?,
            funds: info.funds,
        })))
        .add_submessages(withdraw_messages)
        .add_attribute("action", "withdraw_liquidity"))
}

fn query_pair_given_address(
    querier: &QuerierWrapper,
    pair_address: String,
) -> Result<PairInfo, ContractError> {
    query_pair_contract(querier, pair_address, PairQueryMsg::Pair {})
}

fn query_pair_share(
    querier: &QuerierWrapper,
    pair_address: String,
    amount: Uint128,
) -> Result<Vec<Asset>, ContractError> {
    query_pair_contract(querier, pair_address, PairQueryMsg::Share { amount })
}

fn query_pair_contract<T: serde::de::DeserializeOwned>(
    querier: &QuerierWrapper,
    pair_address: String,
    msg: PairQueryMsg,
) -> Result<T, ContractError> {
    Ok(querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: pair_address,
        msg: encode_binary(&msg)?,
    }))?)
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
        AndromedaMsg::Withdraw {
            recipient,
            tokens_to_withdraw,
        } => execute_withdraw(deps.as_ref(), env, info, recipient, tokens_to_withdraw),
    }
}

pub fn receive_cw20(
    deps: DepsMut,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let token_address = info.sender;
    match from_binary(&cw20_msg.msg)? {
        Cw20HookMsg::AstroportRouterCw20HookMsg(msg) => execute_astroport_cw20_msg(
            token_address.to_string(),
            cw20_msg.amount,
            config.astroport_router_contract.to_string(),
            encode_binary(&msg)?,
        ),
        Cw20HookMsg::AstroportStakingCw20HookMsg(msg) => execute_astroport_cw20_msg(
            token_address.to_string(),
            cw20_msg.amount,
            config.astroport_staking_contract.to_string(),
            encode_binary(&msg)?,
        ),
        Cw20HookMsg::Swapper(msg) => {
            handle_swapper_msg_cw20(deps, cw20_msg.sender, msg, token_address, cw20_msg.amount)
        }
    }
}

fn handle_swapper_msg(
    deps: DepsMut,
    info: MessageInfo,
    msg: SwapperMsg,
) -> Result<Response, ContractError> {
    match msg {
        SwapperMsg::Swap {
            offer_asset_info,
            ask_asset_info,
        } => execute_swap(deps, info, offer_asset_info, ask_asset_info),
    }
}

fn handle_swapper_msg_cw20(
    deps: DepsMut,
    sender: String,
    msg: SwapperCw20HookMsg,
    token_addr: Addr,
    amount: Uint128,
) -> Result<Response, ContractError> {
    match msg {
        SwapperCw20HookMsg::Swap { ask_asset_info } => execute_swap_cw20(
            deps,
            sender,
            token_addr.to_string(),
            amount,
            AssetInfo::Token {
                contract_addr: token_addr,
            },
            ask_asset_info,
        ),
    }
}

fn execute_swap(
    deps: DepsMut,
    info: MessageInfo,
    offer_asset_info: AssetInfo,
    ask_asset_info: AssetInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let operations: Vec<SwapOperation> = get_swap_operations(
        &deps.querier,
        offer_asset_info,
        ask_asset_info,
        config.astroport_factory_contract,
    )?;
    let swap_msg = AstroportRouterExecuteMsg::ExecuteSwapOperations {
        operations,
        minimum_receive: None,
        to: Some(info.sender.clone()),
    };

    execute_astroport_msg(
        info.funds,
        config.astroport_router_contract.to_string(),
        encode_binary(&swap_msg)?,
    )
}

fn execute_swap_cw20(
    deps: DepsMut,
    sender: String,
    token_addr: String,
    amount: Uint128,
    offer_asset_info: AssetInfo,
    ask_asset_info: AssetInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let operations: Vec<SwapOperation> = get_swap_operations(
        &deps.querier,
        offer_asset_info,
        ask_asset_info,
        config.astroport_factory_contract,
    )?;
    let swap_msg = AstroportRouterCw20HookMsg::ExecuteSwapOperations {
        operations,
        minimum_receive: None,
        to: Some(sender),
    };

    execute_astroport_cw20_msg(
        token_addr,
        amount,
        config.astroport_router_contract.to_string(),
        encode_binary(&swap_msg)?,
    )
}

fn get_swap_operations(
    querier: &QuerierWrapper,
    offer_asset_info: AssetInfo,
    ask_asset_info: AssetInfo,
    factory_address: Addr,
) -> Result<Vec<SwapOperation>, ContractError> {
    let existing_pair = query_pair_info(
        querier,
        factory_address,
        &[
            offer_asset_info.clone().into(),
            ask_asset_info.clone().into(),
        ],
    );
    Ok(if existing_pair.is_ok() {
        vec![SwapOperation::AstroSwap {
            offer_asset_info: offer_asset_info.into(),
            ask_asset_info: ask_asset_info.into(),
        }]
    } else if let [AssetInfo::NativeToken { denom: offer_denom }, AssetInfo::NativeToken { denom: ask_denom }] =
        [offer_asset_info.clone(), ask_asset_info.clone()]
    {
        vec![SwapOperation::NativeSwap {
            offer_denom,
            ask_denom,
        }]
    } else {
        let first_swap = if let AssetInfo::NativeToken { denom } = offer_asset_info {
            SwapOperation::NativeSwap {
                offer_denom: denom,
                ask_denom: "uusd".to_string(),
            }
        } else {
            SwapOperation::AstroSwap {
                offer_asset_info: offer_asset_info.into(),
                ask_asset_info: AstroportAssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
            }
        };
        vec![
            first_swap,
            SwapOperation::AstroSwap {
                offer_asset_info: AstroportAssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                ask_asset_info: ask_asset_info.into(),
            },
        ]
    })
}

pub fn execute_astroport_cw20_msg(
    token_addr: String,
    amount: Uint128,
    contract_addr: String,
    msg_binary: Binary,
) -> Result<Response, ContractError> {
    let msg = Cw20ExecuteMsg::Send {
        contract: contract_addr,
        amount,
        msg: msg_binary,
    };
    execute_astroport_msg(vec![], token_addr, encode_binary(&msg)?)
}

pub fn execute_astroport_msg(
    funds: Vec<Coin>,
    contract_addr: String,
    msg_binary: Binary,
) -> Result<Response, ContractError> {
    require(
        funds.is_empty() || funds.len() == 1,
        ContractError::InvalidFunds {
            msg: "Astroport expects no funds or a single type of fund to be deposited.".to_string(),
        },
    )?;

    let execute_msg = WasmMsg::Execute {
        contract_addr,
        funds,
        msg: msg_binary,
    };

    Ok(Response::new().add_message(CosmosMsg::Wasm(execute_msg)))
}

pub fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    astroport_factory_contract: Option<String>,
    astroport_router_contract: Option<String>,
    astroport_staking_contract: Option<String>,
) -> Result<Response, ContractError> {
    require(
        is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;
    let mut config = CONFIG.load(deps.storage)?;
    if let Some(astroport_factory_contract) = astroport_factory_contract {
        config.astroport_factory_contract = deps.api.addr_validate(&astroport_factory_contract)?;
    }
    if let Some(astroport_router_contract) = astroport_router_contract {
        config.astroport_router_contract = deps.api.addr_validate(&astroport_router_contract)?;
    }
    if let Some(astroport_staking_contract) = astroport_staking_contract {
        config.astroport_staking_contract = deps.api.addr_validate(&astroport_staking_contract)?;
    }
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attribute("action", "update_config"))
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

#[cfg_attr(not(feature = "library"), entry_point)]
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

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        astroport_factory_contract: config.astroport_factory_contract.to_string(),
        astroport_router_contract: config.astroport_router_contract.to_string(),
        astroport_staking_contract: config.astroport_staking_contract.to_string(),
    })
}

/// Removes excess assets to avoid over/under contributing to the LP. This can't be done perfectly
/// due to precision.
///
/// ## Arguments
/// * `api` - The api
/// * `pooled_assets` - The assets the make up the pool
/// * `assets` - The two assets sent to the pool
/// * `overflow_recipient` - The recipient of any excess tokens
///
/// Returns the deducted assets and vector of sub messages for sending back excess, or
/// ContractError.
fn verify_asset_ratio(
    api: &dyn Api,
    pooled_assets: [Asset; 2],
    assets: [Asset; 2],
    overflow_recipient: Recipient,
) -> Result<([Asset; 2], Vec<SubMsg>), ContractError> {
    // Do it twice for improved precision. From testing it doesn't appear like doing more than
    // two iterations has any improvement.
    let modified_assets = modify_ratio(&pooled_assets, &modify_ratio(&pooled_assets, &assets)?)?;
    let mut messages: Vec<SubMsg> = vec![];
    for i in 0..2 {
        let delta_asset = Asset {
            amount: assets[i].amount - modified_assets[i].amount,
            info: modified_assets[i].info.clone(),
        };
        if delta_asset.amount > Uint128::zero() {
            messages.push(overflow_recipient.generate_msg_from_asset(api, delta_asset)?);
        }
    }

    println!("{:?}", modified_assets);
    Ok((modified_assets, messages))
}

fn modify_ratio(
    pooled_assets: &[Asset; 2],
    assets: &[Asset; 2],
) -> Result<[Asset; 2], ContractError> {
    let required_second_amount = assets[0]
        .amount
        .multiply_ratio(pooled_assets[1].amount, pooled_assets[0].amount);

    let required_first_amount = assets[1]
        .amount
        .multiply_ratio(pooled_assets[0].amount, pooled_assets[1].amount);

    Ok([
        Asset {
            info: assets[0].info.clone(),
            amount: std::cmp::min(assets[0].amount, required_first_amount),
        },
        Asset {
            info: assets[1].info.clone(),
            amount: std::cmp::min(assets[1].amount, required_second_amount),
        },
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::mock_dependencies;

    fn get_assets(asset1_amount: u128, asset2_amount: u128) -> [Asset; 2] {
        [
            Asset {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("token1".to_owned()),
                }
                .into(),
                amount: asset1_amount.into(),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("token2".to_owned()),
                }
                .into(),
                amount: asset2_amount.into(),
            },
        ]
    }

    #[test]
    fn test_verify_asset_ratio_exact_ratio() {
        let pooled_assets = get_assets(100, 200);
        let deposited_assets = get_assets(10, 20);
        let deps = mock_dependencies(&[]);

        let (assets, msgs) = verify_asset_ratio(
            deps.as_ref().api,
            pooled_assets,
            deposited_assets.clone(),
            Recipient::Addr("sender".to_string()),
        )
        .unwrap();

        assert_eq!(deposited_assets, assets);
        assert!(msgs.is_empty());
    }

    #[test]
    fn test_verify_asset_ratio_too_asset1() {
        let pooled_assets = get_assets(100, 200);
        let deposited_assets = get_assets(18, 20);
        let deps = mock_dependencies(&[]);

        let (assets, msgs) = verify_asset_ratio(
            deps.as_ref().api,
            pooled_assets,
            deposited_assets.clone(),
            Recipient::Addr("sender".to_string()),
        )
        .unwrap();

        assert_eq!(get_assets(10, 20), assets);
        assert_eq!(
            vec![SubMsg::new(WasmMsg::Execute {
                contract_addr: "token1".to_owned(),
                msg: encode_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: "sender".to_string(),
                    amount: 8u128.into(),
                })
                .unwrap(),
                funds: vec![],
            })],
            msgs
        );
    }

    #[test]
    fn test_verify_asset_ratio_too_much_asset2() {
        let pooled_assets = get_assets(100, 200);
        let deposited_assets = get_assets(10, 40);
        let deps = mock_dependencies(&[]);

        let (assets, msgs) = verify_asset_ratio(
            deps.as_ref().api,
            pooled_assets,
            deposited_assets.clone(),
            Recipient::Addr("sender".to_string()),
        )
        .unwrap();

        assert_eq!(get_assets(10, 20), assets);
        assert_eq!(
            vec![SubMsg::new(WasmMsg::Execute {
                contract_addr: "token2".to_owned(),
                msg: encode_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: "sender".to_string(),
                    amount: 20u128.into(),
                })
                .unwrap(),
                funds: vec![],
            })],
            msgs
        );
    }

    #[test]
    fn test_verify_asset_ratio_too_much_both() {
        let pooled_assets = get_assets(100, 200);
        let deposited_assets = get_assets(18, 21);
        let deps = mock_dependencies(&[]);

        let (assets, msgs) = verify_asset_ratio(
            deps.as_ref().api,
            pooled_assets,
            deposited_assets.clone(),
            Recipient::Addr("sender".to_string()),
        )
        .unwrap();

        assert_eq!(get_assets(10, 20), assets);
        assert_eq!(
            vec![
                SubMsg::new(WasmMsg::Execute {
                    contract_addr: "token1".to_owned(),
                    msg: encode_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: "sender".to_string(),
                        amount: 8u128.into(),
                    })
                    .unwrap(),
                    funds: vec![],
                }),
                SubMsg::new(WasmMsg::Execute {
                    contract_addr: "token2".to_owned(),
                    msg: encode_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: "sender".to_string(),
                        amount: 1u128.into(),
                    })
                    .unwrap(),
                    funds: vec![],
                })
            ],
            msgs
        );
    }

    #[test]
    fn test_verify_asset_ratio_rounding() {
        let pooled_assets = get_assets(30, 80);
        let deposited_assets = get_assets(56, 91);
        let deps = mock_dependencies(&[]);

        let (assets, msgs) = verify_asset_ratio(
            deps.as_ref().api,
            pooled_assets,
            deposited_assets.clone(),
            Recipient::Addr("sender".to_string()),
        )
        .unwrap();

        assert_eq!(get_assets(34, 90), assets);
        assert_eq!(
            vec![
                SubMsg::new(WasmMsg::Execute {
                    contract_addr: "token1".to_owned(),
                    msg: encode_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: "sender".to_string(),
                        amount: 22u128.into(),
                    })
                    .unwrap(),
                    funds: vec![],
                }),
                SubMsg::new(WasmMsg::Execute {
                    contract_addr: "token2".to_owned(),
                    msg: encode_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: "sender".to_string(),
                        amount: 1u128.into(),
                    })
                    .unwrap(),
                    funds: vec![],
                }),
            ],
            msgs
        );
    }
}
