use cosmwasm_std::{
    entry_point, from_binary, Addr, Api, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env,
    MessageInfo, QuerierWrapper, Response, SubMsg, Uint128, WasmMsg,
};

use crate::{
    primitive_keys::{
        ADDRESSES_TO_CACHE, ASTROPORT_ASTRO, ASTROPORT_FACTORY, ASTROPORT_ROUTER, ASTROPORT_XASTRO,
    },
    querier::{query_pair_given_address, query_pair_share},
    staking::{
        execute_claim_lp_staking_rewards, execute_stake_astro, execute_stake_lp,
        execute_unstake_astro, execute_unstake_lp,
    },
};
use ado_base::state::ADOContract;
use andromeda_protocol::{
    astroport::{Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    swapper::{SwapperCw20HookMsg, SwapperMsg},
};
use astroport::{
    asset::AssetInfo as AstroportAssetInfo,
    pair::{Cw20HookMsg as PairCw20HookMsg, ExecuteMsg as AstroportPairExecuteMsg},
    querier::query_pair_info,
    router::{
        Cw20HookMsg as AstroportRouterCw20HookMsg, ExecuteMsg as AstroportRouterExecuteMsg,
        SwapOperation,
    },
};
use common::{
    ado_base::{recipient::Recipient, InstantiateMsg as BaseInstantiateMsg},
    encode_binary,
    error::ContractError,
    require,
};

use cw2::{get_contract_version, set_contract_version};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use cw_asset::{Asset, AssetInfo, AssetUnchecked};
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
    let contract = ADOContract::default();
    let resp = contract.instantiate(
        deps.storage,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "astroport".to_string(),
            operators: None,
            modules: None,
            primitive_contract: Some(msg.primitive_contract),
        },
    )?;
    for address in ADDRESSES_TO_CACHE {
        contract.cache_address(deps.storage, &deps.querier, address)?;
    }
    // Astro token is obtained from staking LP tokens.
    let astroport_astro = contract.get_cached_address(deps.storage, ASTROPORT_ASTRO)?;
    contract.add_withdrawable_token(
        deps.storage,
        &astroport_astro,
        &AssetInfo::Cw20(deps.api.addr_validate(&astroport_astro)?),
    )?;
    // xAstro is obtained from staking Astro.
    let astroport_xastro = contract.get_cached_address(deps.storage, ASTROPORT_XASTRO)?;
    contract.add_withdrawable_token(
        deps.storage,
        &astroport_xastro,
        &AssetInfo::Cw20(deps.api.addr_validate(&astroport_xastro)?),
    )?;
    Ok(resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    let astroport_factory = contract.get_cached_address(deps.storage, ASTROPORT_FACTORY)?;
    match msg {
        ExecuteMsg::AndrReceive(msg) => contract.execute(deps, env, info, msg, execute),
        ExecuteMsg::Swapper(msg) => handle_swapper_msg(deps, info, msg),
        ExecuteMsg::Receive(msg) => receive_cw20(deps, info, msg),
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
        ExecuteMsg::StakeLp {
            lp_token_contract,
            amount,
        } => execute_stake_lp(deps, env, info, lp_token_contract, amount),
        ExecuteMsg::UnstakeLp {
            lp_token_contract,
            amount,
        } => execute_unstake_lp(deps, env, info, lp_token_contract, amount),
        ExecuteMsg::ClaimLpStakingRewards {
            lp_token_contract,
            auto_stake,
        } => execute_claim_lp_staking_rewards(deps, env, info, lp_token_contract, auto_stake),
        ExecuteMsg::StakeAstro { amount } => execute_stake_astro(deps, env, info, amount),
        ExecuteMsg::UnstakeAstro { amount } => execute_unstake_astro(deps, env, info, amount),
        ExecuteMsg::AstroportFactoryExecuteMsg(msg) => {
            execute_astroport_msg(info.funds, astroport_factory, encode_binary(&msg)?)
        }
    }
}

fn execute_provide_liquidity(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    assets: [AssetUnchecked; 2],
    slippage_tolerance: Option<Decimal>,
    auto_stake: Option<bool>,
) -> Result<Response, ContractError> {
    let sender = info.sender.as_str();
    let contract = ADOContract::default();
    let astroport_factory = contract.get_cached_address(deps.storage, ASTROPORT_FACTORY)?;
    require(
        contract.is_owner_or_operator(deps.storage, sender)?,
        ContractError::Unauthorized {},
    )?;

    let assets = [
        assets[0].check(deps.api, None)?,
        assets[1].check(deps.api, None)?,
    ];

    let pair = query_pair_info(
        &deps.querier,
        deps.api.addr_validate(&astroport_factory)?,
        &[assets[0].info.clone().into(), assets[1].info.clone().into()],
    )?;

    let pooled_assets = pair.query_pools(&deps.querier, pair.contract_addr.clone())?;
    let mission_contract = contract.get_mission_contract(deps.storage)?;
    let (assets, overflow_messages) = verify_asset_ratio(
        deps.api,
        &deps.querier,
        mission_contract,
        pooled_assets.map(|a| a.into()),
        assets,
        Recipient::Addr(info.sender.to_string()),
    )?;

    // In the case where we want to witdraw the LP token.
    contract.add_withdrawable_token(
        deps.storage,
        &pair.liquidity_token.to_string(),
        &AssetInfo::Cw20(pair.liquidity_token),
    )?;
    let mut initial_transfer_messages: Vec<SubMsg> = vec![];
    for asset in assets.iter() {
        if let AssetInfo::Cw20(contract_addr) = &asset.info {
            initial_transfer_messages.push(SubMsg::new(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                // User needs to allow this contract to transfer tokens.
                msg: encode_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: info.sender.to_string(),
                    recipient: env.contract.address.to_string(),
                    // The excess is already deducted from this amount.
                    amount: asset.amount,
                })?,
                funds: vec![],
            }));
            initial_transfer_messages.push(SubMsg::new(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                // Allow the pair address to transfer from here.
                msg: encode_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                    spender: pair.contract_addr.to_string(),
                    // The excess is already deducted from this amount.
                    amount: asset.amount,
                    expires: None,
                })?,
                funds: vec![],
            }));
        }
    }

    let native_funds = get_native_funds_from_assets(&assets);
    Ok(Response::new()
        .add_submessages(initial_transfer_messages)
        .add_submessages(overflow_messages)
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: pair.contract_addr.to_string(),
            msg: encode_binary(&AstroportPairExecuteMsg::ProvideLiquidity {
                assets: assets.map(|a| a.into()),
                slippage_tolerance,
                auto_stake,
                // Not strictly neccessary but to be explicit.
                receiver: Some(env.contract.address.to_string()),
            })?,
            funds: native_funds,
        }))
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
    let sender = info.sender.as_str();
    let contract = ADOContract::default();
    require(
        contract.is_owner_or_operator(deps.storage, sender)?,
        ContractError::Unauthorized {},
    )?;
    let recipient = recipient.unwrap_or_else(|| Recipient::Addr(sender.to_owned()));
    let pair_info = query_pair_given_address(&deps.querier, pair_address)?;
    let lp_token_asset_info = AssetInfo::cw20(pair_info.liquidity_token.clone());
    let total_amount = lp_token_asset_info.query_balance(&deps.querier, env.contract.address)?;
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
    let mission_contract = contract.get_mission_contract(deps.storage)?;
    for asset in share.into_iter() {
        withdraw_messages.push(recipient.generate_msg_from_asset(
            deps.api,
            &deps.querier,
            mission_contract.clone(),
            asset,
        )?);
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

pub fn receive_cw20(
    deps: DepsMut,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    require(
        !cw20_msg.amount.is_zero(),
        ContractError::InvalidFunds {
            msg: "Amount must be non-zero".to_string(),
        },
    )?;

    let token_address = info.sender;
    match from_binary(&cw20_msg.msg)? {
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
            AssetInfo::Cw20(token_addr),
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
    let contract = ADOContract::default();
    let astroport_factory = contract.get_cached_address(deps.storage, ASTROPORT_FACTORY)?;
    let astroport_router = contract.get_cached_address(deps.storage, ASTROPORT_ROUTER)?;

    let operations: Vec<SwapOperation> = get_swap_operations(
        &deps.querier,
        offer_asset_info,
        ask_asset_info,
        deps.api.addr_validate(&astroport_factory)?,
    )?;
    let swap_msg = AstroportRouterExecuteMsg::ExecuteSwapOperations {
        operations,
        minimum_receive: None,
        to: Some(info.sender.clone()),
    };

    execute_astroport_msg(info.funds, astroport_router, encode_binary(&swap_msg)?)
}

fn execute_swap_cw20(
    deps: DepsMut,
    sender: String,
    token_addr: String,
    amount: Uint128,
    offer_asset_info: AssetInfo,
    ask_asset_info: AssetInfo,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    let astroport_factory = contract.get_cached_address(deps.storage, ASTROPORT_FACTORY)?;
    let astroport_router = contract.get_cached_address(deps.storage, ASTROPORT_ROUTER)?;

    let operations: Vec<SwapOperation> = get_swap_operations(
        &deps.querier,
        offer_asset_info,
        ask_asset_info,
        deps.api.addr_validate(&astroport_factory)?,
    )?;
    let swap_msg = AstroportRouterCw20HookMsg::ExecuteSwapOperations {
        operations,
        minimum_receive: None,
        to: Some(sender),
    };

    execute_astroport_cw20_msg(
        token_addr,
        amount,
        astroport_router,
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
    } else if let [AssetInfo::Native(offer_denom), AssetInfo::Native(ask_denom)] =
        [offer_asset_info.clone(), ask_asset_info.clone()]
    {
        vec![SwapOperation::NativeSwap {
            offer_denom,
            ask_denom,
        }]
    } else {
        let first_swap = if let AssetInfo::Native(denom) = offer_asset_info {
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
        QueryMsg::AndrQuery(msg) => ADOContract::default().query(deps, env, msg, query),
    }
}

/// Removes excess assets to avoid over/under contributing to the LP. This can't be done perfectly
/// due to precision.
///
/// ## Arguments
/// * `api` - The api
/// * `pooled_assets` - The assets the make up the pool
/// * `sent_assets` - The two assets sent to the pool
/// * `overflow_recipient` - The recipient of any excess tokens
///
/// Returns the deducted assets and vector of sub messages for sending back excess, or
/// ContractError.
fn verify_asset_ratio(
    api: &dyn Api,
    querier: &QuerierWrapper,
    mission_contract: Option<Addr>,
    pooled_assets: [Asset; 2],
    sent_assets: [Asset; 2],
    overflow_recipient: Recipient,
) -> Result<([Asset; 2], Vec<SubMsg>), ContractError> {
    if pooled_assets[0].amount.is_zero() && pooled_assets[1].amount.is_zero() {
        return Ok((sent_assets, vec![]));
    }

    // Ensure that the indices of the deposited assets and pooled assets line up.
    let pooled_amounts: [Uint128; 2] = [
        pooled_assets
            .iter()
            .find(|a| a.info == sent_assets[0].info)
            .map(|a| a.amount)
            .expect("Wrong asset info is given"),
        pooled_assets
            .iter()
            .find(|a| a.info == sent_assets[1].info)
            .map(|a| a.amount)
            .expect("Wrong asset info is given"),
    ];

    // This is done twice to improve precision as often when one of the token balances gets changed
    // we get a better approximation for the second. Tests have shown that further iterations do
    // not make a difference due to precision limitiations.
    let modified_assets = modify_ratio(
        &pooled_amounts,
        &modify_ratio(&pooled_amounts, &sent_assets)?,
    )?;
    let mut messages: Vec<SubMsg> = vec![];
    for i in 0..2 {
        let delta_asset = Asset {
            amount: sent_assets[i].amount - modified_assets[i].amount,
            info: modified_assets[i].info.clone(),
        };
        // Messages for cw20 tokens not needed since the deducted amounts will be transfered in.
        // Alternatively we could have pulled in the original amount and sent back the excess, but
        // that requires an extra message to be sent for no reason.
        if delta_asset.amount > Uint128::zero() && matches!(delta_asset.info, AssetInfo::Native(_))
        {
            messages.push(overflow_recipient.generate_msg_from_asset(
                api,
                querier,
                mission_contract.clone(),
                delta_asset,
            )?);
        }
    }

    Ok((modified_assets, messages))
}

fn modify_ratio(
    pooled_amounts: &[Uint128; 2],
    assets: &[Asset; 2],
) -> Result<[Asset; 2], ContractError> {
    let required_second_amount = assets[0]
        .amount
        .multiply_ratio(pooled_amounts[1], pooled_amounts[0]);

    let required_first_amount = assets[1]
        .amount
        .multiply_ratio(pooled_amounts[0], pooled_amounts[1]);

    Ok([
        Asset {
            info: assets[0].info.clone(),
            amount: cmp::min(assets[0].amount, required_first_amount),
        },
        Asset {
            info: assets[1].info.clone(),
            amount: cmp::min(assets[1].amount, required_second_amount),
        },
    ])
}

fn get_native_funds_from_assets(assets: &[Asset; 2]) -> Vec<Coin> {
    let mut coins: Vec<Coin> = vec![];
    for asset in assets.iter() {
        if let AssetInfo::Native(denom) = &asset.info {
            coins.push(Coin {
                denom: denom.clone(),
                amount: asset.amount,
            });
        }
    }
    coins
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{coins, testing::mock_dependencies, BankMsg};

    fn get_assets_second_native(asset1_amount: u128, asset2_amount: u128) -> [Asset; 2] {
        [
            Asset {
                info: AssetInfo::Cw20(Addr::unchecked("token1".to_owned())),
                amount: asset1_amount.into(),
            },
            Asset {
                info: AssetInfo::native("uusd"),
                amount: asset2_amount.into(),
            },
        ]
    }

    fn get_assets(asset1_amount: u128, asset2_amount: u128) -> [Asset; 2] {
        [
            Asset {
                info: AssetInfo::Cw20(Addr::unchecked("token1")),
                amount: asset1_amount.into(),
            },
            Asset {
                info: AssetInfo::Cw20(Addr::unchecked("token2")),
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
            &deps.as_ref().querier,
            None,
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
            &deps.as_ref().querier,
            None,
            pooled_assets,
            deposited_assets,
            Recipient::Addr("sender".to_string()),
        )
        .unwrap();

        assert_eq!(get_assets(10, 20), assets);
        assert!(msgs.is_empty());
    }

    #[test]
    fn test_verify_asset_ratio_order_agnostic() {
        // Here token2 is first and token1 is second.
        let pooled_assets = [
            Asset {
                info: AssetInfo::Cw20(Addr::unchecked("token2")),
                amount: 200u128.into(),
            },
            Asset {
                info: AssetInfo::Cw20(Addr::unchecked("token1")),
                amount: 100u128.into(),
            },
        ];
        // Here token1 is first and token2 is second.
        let deposited_assets = get_assets(18, 20);
        let deps = mock_dependencies(&[]);

        let (assets, msgs) = verify_asset_ratio(
            deps.as_ref().api,
            &deps.as_ref().querier,
            None,
            pooled_assets,
            deposited_assets,
            Recipient::Addr("sender".to_string()),
        )
        .unwrap();

        assert_eq!(get_assets(10, 20), assets);
        assert!(msgs.is_empty());
    }

    #[test]
    fn test_verify_asset_ratio_too_much_asset2() {
        let pooled_assets = get_assets(100, 200);
        let deposited_assets = get_assets(10, 40);
        let deps = mock_dependencies(&[]);

        let (assets, msgs) = verify_asset_ratio(
            deps.as_ref().api,
            &deps.as_ref().querier,
            None,
            pooled_assets,
            deposited_assets,
            Recipient::Addr("sender".to_string()),
        )
        .unwrap();

        assert_eq!(get_assets(10, 20), assets);
        assert!(msgs.is_empty());
    }

    #[test]
    fn test_verify_asset_ratio_too_much_both() {
        let pooled_assets = get_assets(100, 200);
        let deposited_assets = get_assets(18, 21);
        let deps = mock_dependencies(&[]);

        let (assets, msgs) = verify_asset_ratio(
            deps.as_ref().api,
            &deps.as_ref().querier,
            None,
            pooled_assets,
            deposited_assets,
            Recipient::Addr("sender".to_string()),
        )
        .unwrap();

        assert_eq!(get_assets(10, 20), assets);
        assert!(msgs.is_empty());
    }

    #[test]
    fn test_verify_asset_ratio_rounding() {
        let pooled_assets = get_assets(30, 80);
        let deposited_assets = get_assets(56, 93);
        let deps = mock_dependencies(&[]);

        let (assets, msgs) = verify_asset_ratio(
            deps.as_ref().api,
            &deps.as_ref().querier,
            None,
            pooled_assets,
            deposited_assets,
            Recipient::Addr("sender".to_string()),
        )
        .unwrap();

        assert_eq!(get_assets(34, 90), assets);
        assert!(msgs.is_empty());
    }

    #[test]
    fn test_verify_asset_second_native() {
        let pooled_assets = get_assets_second_native(10, 20);
        let deposited_assets = get_assets_second_native(18, 33);
        let deps = mock_dependencies(&[]);

        let (assets, msgs) = verify_asset_ratio(
            deps.as_ref().api,
            &deps.as_ref().querier,
            None,
            pooled_assets,
            deposited_assets,
            Recipient::Addr("sender".to_string()),
        )
        .unwrap();

        assert_eq!(get_assets_second_native(16, 32), assets);
        assert_eq!(
            vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "sender".to_string(),
                amount: coins(1, "uusd")
            }))],
            msgs
        );
    }
}
