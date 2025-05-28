use andromeda_std::andr_execute_fn;

use andromeda_std::{
    ado_base::{InstantiateMsg as BaseInstantiateMsg, MigrateMsg},
    ado_contract::ADOContract,
    amp::{AndrAddr, Recipient},
    common::{context::ExecuteContext, denom::Asset, encode_binary},
    error::ContractError,
};

#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    attr, entry_point, from_json, wasm_execute, Binary, Decimal, Deps, DepsMut, Env, MessageInfo,
    Reply, Response, StdError, Uint128,
};

use cw2::set_contract_version;
use cw20::Cw20ReceiveMsg;
use cw_utils::one_coin;

use crate::state::LP_PAIR_ADDRESS;
use crate::{
    astroport::{
        execute_swap_astroport_msg, handle_astroport_swap_reply,
        query_simulate_astro_swap_operation, ASTROPORT_MSG_CREATE_PAIR_AND_PROVIDE_LIQUIDITY_ID,
        ASTROPORT_MSG_CREATE_PAIR_ID, ASTROPORT_MSG_FORWARD_ID, ASTROPORT_MSG_PROVIDE_LIQUIDITY_ID,
        ASTROPORT_MSG_SWAP_ID,
    },
    state::{
        AstroportFactoryExecuteMsg, ForwardReplyState, LiquidityProvisionState, FACTORY,
        FORWARD_REPLY_STATE, LIQUIDITY_PROVISION_STATE, PAIR_ADDRESS, SWAP_ROUTER,
    },
};

use andromeda_socket::astroport::{
    AssetEntry, AssetInfo, Cw20HookMsg, ExecuteMsg, InstantiateMsg, LpPairAddressResponse,
    PairAddressResponse, PairExecuteMsg, PairType, QueryMsg, SimulateSwapOperationResponse,
    SwapOperation, WithdrawLiquidityInner, WithdrawLiquidityMsg,
};

const CONTRACT_NAME: &str = "crates.io:andromeda-socket-astroport";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let inst_resp = ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        &deps.querier,
        info.clone(),
        BaseInstantiateMsg {
            ado_type: CONTRACT_NAME.to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            kernel_address: msg.kernel_address.clone(),
            owner: msg.owner,
        },
    )?;

    let swap_router = msg
        .swap_router
        .unwrap_or(AndrAddr::from_string("/lib/astroport/router"));
    swap_router.get_raw_address(&deps.as_ref())?;
    SWAP_ROUTER.save(deps.storage, &swap_router)?;

    let factory_addr =
        AndrAddr::from_string("neutron1jj0scx400pswhpjes589aujlqagxgcztw04srynmhf0f6zplzn2qqmhwj7");

    FACTORY.save(deps.storage, &factory_addr)?;

    Ok(inst_resp
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[andr_execute_fn]
pub fn execute(ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => handle_receive_cw20(ctx, msg),
        ExecuteMsg::SwapAndForward {
            to_asset,
            recipient,
            max_spread,
            minimum_receive,
            operations,
        } => execute_swap_and_forward(
            ctx,
            to_asset,
            recipient,
            max_spread,
            minimum_receive,
            operations,
        ),
        ExecuteMsg::UpdateSwapRouter { swap_router } => {
            execute_update_swap_router(ctx, swap_router)
        }
        ExecuteMsg::CreatePair {
            pair_type,
            asset_infos,
            init_params,
        } => create_factory_pair(ctx, pair_type, asset_infos, init_params),
        ExecuteMsg::ProvideLiquidity {
            assets,
            slippage_tolerance,
            auto_stake,
            receiver,
        } => provide_liquidity(ctx, assets, slippage_tolerance, auto_stake, receiver),
        ExecuteMsg::CreatePairAndProvideLiquidity {
            pair_type,
            asset_infos,
            init_params,
            assets,
            slippage_tolerance,
            auto_stake,
            receiver,
        } => create_pair_and_provide_liquidity(
            ctx,
            pair_type,
            asset_infos,
            init_params,
            assets,
            slippage_tolerance,
            auto_stake,
            receiver,
        ),
        ExecuteMsg::WithdrawLiquidity {
            pair_address,
            sender,
        } => withdraw_liquidity(ctx, pair_address, sender),
        _ => ADOContract::default().execute(ctx, msg),
    }
}

fn handle_receive_cw20(
    ctx: ExecuteContext,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let ExecuteContext { ref info, .. } = ctx;

    let amount = cw20_msg.amount;
    let sender = cw20_msg.sender;
    let from_addr = AndrAddr::from_string(info.sender.clone());
    let from_asset = Asset::Cw20Token(from_addr.clone());

    match from_json(&cw20_msg.msg)? {
        Cw20HookMsg::SwapAndForward {
            to_asset,
            recipient,
            max_spread,
            minimum_receive,
            operations,
        } => {
            let recipient = match recipient {
                None => Recipient::new(sender.clone(), None),
                Some(recipient) => recipient,
            };
            recipient.validate(&ctx.deps.as_ref())?;

            swap_and_forward_cw20(
                ctx,
                from_asset,
                amount,
                to_asset,
                recipient,
                AndrAddr::from_string(sender),
                max_spread,
                minimum_receive,
                operations,
            )
        }
        Cw20HookMsg::ProvideLiquidity {
            other_asset,
            slippage_tolerance,
            auto_stake,
            receiver,
        } => {
            let cw20_asset = AssetEntry {
                info: AssetInfo::Token {
                    contract_addr: from_addr.get_raw_address(&ctx.deps.as_ref())?,
                },
                amount,
            };

            let assets = vec![cw20_asset, other_asset];

            provide_liquidity(ctx, assets, slippage_tolerance, auto_stake, receiver)
        }
        Cw20HookMsg::CreatePairAndProvideLiquidity {
            pair_type,
            asset_infos,
            init_params,
            other_asset,
            slippage_tolerance,
            auto_stake,
            receiver,
        } => {
            let cw20_asset = AssetEntry {
                info: AssetInfo::Token {
                    contract_addr: from_addr.get_raw_address(&ctx.deps.as_ref())?,
                },
                amount,
            };

            let assets = vec![cw20_asset, other_asset];

            create_pair_and_provide_liquidity(
                ctx,
                pair_type,
                asset_infos,
                init_params,
                assets,
                slippage_tolerance,
                auto_stake,
                receiver,
            )
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_swap_and_forward(
    ctx: ExecuteContext,
    to_asset: Asset,
    recipient: Option<Recipient>,
    max_spread: Option<Decimal>,
    minimum_receive: Option<Uint128>,
    operations: Option<Vec<SwapOperation>>,
) -> Result<Response, ContractError> {
    let fund = one_coin(&ctx.info).map_err(|_| ContractError::InvalidAsset {
        asset: "Invalid or missing coin".to_string(),
    })?;

    let from_asset = Asset::NativeToken(fund.denom);
    let sender = AndrAddr::from_string(&ctx.info.sender);
    let recipient = match recipient {
        None => Recipient::new(sender.clone(), None),
        Some(recipient) => recipient,
    };
    recipient.validate(&ctx.deps.as_ref())?;

    let swap_msg = execute_swap_astroport_msg(
        ctx,
        from_asset.clone(),
        fund.amount,
        to_asset.clone(),
        recipient.clone(),
        sender,
        max_spread,
        minimum_receive,
        operations,
    )?;

    Ok(Response::default()
        .add_submessage(swap_msg)
        .add_attributes(vec![
            attr("from_asset", from_asset.to_string()),
            attr("from_amount", fund.amount),
            attr("to_asset", to_asset.to_string()),
            attr("recipient", recipient.get_addr()),
        ]))
}

#[allow(clippy::too_many_arguments)]
fn swap_and_forward_cw20(
    ctx: ExecuteContext,
    from_asset: Asset,
    from_amount: Uint128,
    to_asset: Asset,
    recipient: Recipient,
    refund_addr: AndrAddr,
    max_spread: Option<Decimal>,
    minimum_receive: Option<Uint128>,
    operations: Option<Vec<SwapOperation>>,
) -> Result<Response, ContractError> {
    let swap_msg = execute_swap_astroport_msg(
        ctx,
        from_asset.clone(),
        from_amount,
        to_asset.clone(),
        recipient.clone(),
        refund_addr,
        max_spread,
        minimum_receive,
        operations,
    )?;

    Ok(Response::default()
        .add_submessage(swap_msg)
        .add_attributes(vec![
            attr("from_asset", from_asset.to_string()),
            attr("from_amount", from_amount),
            attr("to_asset", to_asset.to_string()),
            attr("recipient", recipient.get_addr()),
        ]))
}

fn execute_update_swap_router(
    ctx: ExecuteContext,
    swap_router: AndrAddr,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, info, .. } = ctx;

    // Verify sender has owner permissions
    ADOContract::default().is_contract_owner(deps.storage, info.sender.as_str())?;

    swap_router.get_raw_address(&deps.as_ref())?;
    let previous_swap_router = SWAP_ROUTER.load(deps.storage)?;

    SWAP_ROUTER.save(deps.storage, &swap_router)?;
    Ok(Response::new().add_attributes(vec![
        attr("action", "update-swap-router"),
        attr("previous_swap_router", previous_swap_router),
        attr("swap_router", swap_router),
    ]))
}

fn create_factory_pair(
    ctx: ExecuteContext,
    pair_type: PairType,
    asset_infos: Vec<AssetInfo>,
    init_parameters: Option<Binary>,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, .. } = ctx;

    let factory_addr = FACTORY.load(deps.storage)?;
    let factory_addr_raw = factory_addr.get_raw_address(&deps.as_ref())?;

    let create_factory_pair_msg = AstroportFactoryExecuteMsg::CreatePair {
        pair_type: pair_type.clone(),
        asset_infos: asset_infos.clone(),
        init_params: init_parameters,
    };

    let wasm_msg = wasm_execute(factory_addr_raw, &create_factory_pair_msg, vec![])?;

    // Return response with the wasm message as a submessage with a reply ID
    // so we can extract the LP pool address from the response
    Ok(Response::new()
        .add_submessage(cosmwasm_std::SubMsg::reply_always(
            wasm_msg,
            ASTROPORT_MSG_CREATE_PAIR_ID,
        ))
        .add_attributes(vec![
            attr("action", "create_factory_pair"),
            attr("pair_type", format!("{:?}", pair_type.clone())),
            attr("asset_infos", format!("{:?}", asset_infos.clone())),
        ]))
}

fn provide_liquidity(
    ctx: ExecuteContext,
    assets: Vec<andromeda_socket::astroport::AssetEntry>,
    slippage_tolerance: Option<Decimal>,
    auto_stake: Option<bool>,
    receiver: Option<String>,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, .. } = ctx;

    // Load the pair address from state
    let pair_addr = PAIR_ADDRESS.load(deps.storage)?;
    let pair_addr_raw = pair_addr.get_raw_address(&deps.as_ref())?;

    // Build the provide liquidity message
    let provide_liquidity_msg = PairExecuteMsg::ProvideLiquidity {
        assets: assets.clone(),
        slippage_tolerance,
        auto_stake,
        receiver,
    };

    // Handle both native coins and CW20 token allowances
    // NOTE: For CW20 tokens received via hooks, this socket contract owns the tokens
    // and needs to give allowance to the pair contract to spend them (as per Astroport docs)
    let mut response = Response::new();
    let mut native_coins = vec![];

    for asset in &assets {
        match &asset.info {
            AssetInfo::NativeToken { denom } => {
                native_coins.push(cosmwasm_std::Coin {
                    denom: denom.clone(),
                    amount: asset.amount,
                });
            }
            AssetInfo::Token { contract_addr } => {
                // Set allowance for the pair contract to spend CW20 tokens owned by this socket
                // This is required by Astroport: "increase your token allowance for the pool before providing liquidity"
                let allowance_msg = cw20::Cw20ExecuteMsg::IncreaseAllowance {
                    spender: pair_addr_raw.to_string(),
                    amount: asset.amount,
                    expires: None,
                };
                let allowance_wasm_msg = wasm_execute(contract_addr, &allowance_msg, vec![])?;
                response = response.add_message(allowance_wasm_msg);
            }
        }
    }

    // Send the provide liquidity message to the pair (native coins attached, CW20s via allowance)
    let provide_wasm_msg = wasm_execute(pair_addr_raw, &provide_liquidity_msg, native_coins)?;
    response = response.add_message(provide_wasm_msg);

    Ok(response.add_attributes(vec![
        attr("action", "provide_liquidity"),
        attr("pair_address", pair_addr.to_string()),
        attr("assets", format!("{:?}", assets)),
    ]))
}

#[allow(clippy::too_many_arguments)]
fn create_pair_and_provide_liquidity(
    ctx: ExecuteContext,
    pair_type: PairType,
    asset_infos: Vec<AssetInfo>,
    init_parameters: Option<Binary>,
    assets: Vec<AssetEntry>,
    slippage_tolerance: Option<Decimal>,
    auto_stake: Option<bool>,
    receiver: Option<String>,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, .. } = ctx;

    let factory_addr = FACTORY.load(deps.storage)?;
    let factory_addr_raw = factory_addr.get_raw_address(&deps.as_ref())?;

    // Store the liquidity provision parameters for use in the reply handler
    let liquidity_state = LiquidityProvisionState {
        assets: assets.clone(),
        slippage_tolerance,
        auto_stake,
        receiver,
    };
    LIQUIDITY_PROVISION_STATE.save(deps.storage, &liquidity_state)?;

    let create_factory_pair_msg = AstroportFactoryExecuteMsg::CreatePair {
        pair_type: pair_type.clone(),
        asset_infos: asset_infos.clone(),
        init_params: init_parameters,
    };

    let wasm_msg = wasm_execute(factory_addr_raw, &create_factory_pair_msg, vec![])?;

    // Return response with the wasm message as a submessage with a specific reply ID
    // so we can extract the LP pool address and then provide liquidity
    Ok(Response::new()
        .add_submessage(cosmwasm_std::SubMsg::reply_always(
            wasm_msg,
            ASTROPORT_MSG_CREATE_PAIR_AND_PROVIDE_LIQUIDITY_ID,
        ))
        .add_attributes(vec![
            attr("action", "create_pair_and_provide_liquidity"),
            attr("pair_type", format!("{:?}", pair_type.clone())),
            attr("asset_infos", format!("{:?}", asset_infos.clone())),
            attr("assets", format!("{:?}", assets)),
        ]))
}

fn withdraw_liquidity(
    ctx: ExecuteContext,
    pair_address: AndrAddr,
    sender: String,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, .. } = ctx;

    let pair_addr_raw = pair_address.get_raw_address(&deps.as_ref())?;

    let withdraw_msg = cosmwasm_std::to_json_binary(&WithdrawLiquidityMsg {
        withdraw_liquidity: WithdrawLiquidityInner {},
    })?;

    let wasm_msg = wasm_execute(&pair_addr_raw, &withdraw_msg, vec![])?;

    Ok(Response::new().add_message(wasm_msg).add_attributes(vec![
        attr("action", "withdraw_liquidity"),
        attr("pair_address", pair_addr_raw.to_string()),
        attr("sender", sender),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::SimulateSwapOperation {
            offer_amount,
            operations,
        } => encode_binary(&query_simulate_swap_operation(
            deps,
            offer_amount,
            operations,
        )?),
        QueryMsg::PairAddress {} => encode_binary(&query_pair_address(deps)?),
        QueryMsg::LpPairAddress {} => encode_binary(&query_lp_pair_address(deps)?),
    }
}

fn query_simulate_swap_operation(
    deps: Deps,
    offer_amount: Uint128,
    swap_operation: Vec<SwapOperation>,
) -> Result<SimulateSwapOperationResponse, ContractError> {
    query_simulate_astro_swap_operation(deps, offer_amount, swap_operation)
}

fn query_pair_address(deps: Deps) -> Result<PairAddressResponse, ContractError> {
    let pair_address = PAIR_ADDRESS.may_load(deps.storage)?;
    Ok(PairAddressResponse {
        pair_address: pair_address.map(|addr| addr.to_string()),
    })
}

fn query_lp_pair_address(deps: Deps) -> Result<LpPairAddressResponse, ContractError> {
    let pair_address = LP_PAIR_ADDRESS.may_load(deps.storage)?;
    Ok(LpPairAddressResponse {
        lp_pair_address: pair_address,
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    ADOContract::default().migrate(deps, env, CONTRACT_NAME, CONTRACT_VERSION)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        ASTROPORT_MSG_SWAP_ID => {
            let state: ForwardReplyState = FORWARD_REPLY_STATE.load(deps.storage)?;
            FORWARD_REPLY_STATE.remove(deps.storage);

            if msg.result.is_err() {
                Err(ContractError::Std(StdError::generic_err(format!(
                    "Astroport swap failed with error: {:?}",
                    msg.result.unwrap_err()
                ))))
            } else {
                handle_astroport_swap_reply(deps, env, msg, state)
            }
        }
        ASTROPORT_MSG_FORWARD_ID => {
            if msg.result.is_err() {
                return Err(ContractError::Std(StdError::generic_err(format!(
                    "Astroport msg forwarding failed with error: {:?}",
                    msg.result.unwrap_err()
                ))));
            }
            Ok(Response::default()
                .add_attributes(vec![attr("action", "message_forwarded_success")]))
        }
        ASTROPORT_MSG_CREATE_PAIR_ID => {
            if msg.result.is_err() {
                return Err(ContractError::Std(StdError::generic_err(format!(
                    "Astroport create pair failed with error: {:?}",
                    msg.result.unwrap_err()
                ))));
            }

            // Extract the pair address from the response
            let response = msg.result.unwrap();

            // Look for the pair contract address in the events
            let pair_address = response
                .events
                .iter()
                .find(|event| event.ty == "instantiate")
                .and_then(|event| {
                    event
                        .attributes
                        .iter()
                        .find(|attr| attr.key == "_contract_address")
                        .map(|attr| attr.value.clone())
                })
                .ok_or_else(|| {
                    ContractError::Std(StdError::generic_err(
                        "Could not find pair contract address in response".to_string(),
                    ))
                })?;

            // Store the pair address
            let pair_addr = AndrAddr::from_string(pair_address.clone());
            LP_PAIR_ADDRESS.save(deps.storage, &pair_addr)?;

            Ok(Response::default().add_attributes(vec![
                attr("action", "create_pair_success"),
                attr("pair_address", pair_address),
            ]))
        }
        ASTROPORT_MSG_CREATE_PAIR_AND_PROVIDE_LIQUIDITY_ID => {
            if msg.result.is_err() {
                return Err(ContractError::Std(StdError::generic_err(format!(
                    "Astroport create pair and provide liquidity failed with error: {:?}",
                    msg.result.unwrap_err()
                ))));
            }

            // Extract the pair address from the response
            let response = msg.result.unwrap();

            // Look for the pair contract address in the events
            let pair_address = response
                .events
                .iter()
                .find(|event| event.ty == "instantiate")
                .and_then(|event| {
                    event
                        .attributes
                        .iter()
                        .find(|attr| attr.key == "_contract_address")
                        .map(|attr| attr.value.clone())
                })
                .ok_or_else(|| {
                    ContractError::Std(StdError::generic_err(
                        "Could not find pair contract address in response".to_string(),
                    ))
                })?;

            let pair_addr = AndrAddr::from_string(pair_address.clone());
            LP_PAIR_ADDRESS.save(deps.storage, &pair_addr)?;

            // Load the liquidity provision parameters
            let liquidity_state = LIQUIDITY_PROVISION_STATE.load(deps.storage)?;
            LIQUIDITY_PROVISION_STATE.remove(deps.storage);

            // Build the provide liquidity message
            let provide_liquidity_msg = PairExecuteMsg::ProvideLiquidity {
                assets: liquidity_state.assets.clone(),
                slippage_tolerance: liquidity_state.slippage_tolerance,
                auto_stake: liquidity_state.auto_stake,
                receiver: liquidity_state.receiver,
            };

            // Handle both native coins and CW20 token allowances
            let mut response_msgs = vec![];
            let mut native_coins = vec![];

            for asset in &liquidity_state.assets {
                match &asset.info {
                    AssetInfo::NativeToken { denom } => {
                        native_coins.push(cosmwasm_std::Coin {
                            denom: denom.clone(),
                            amount: asset.amount,
                        });
                    }
                    AssetInfo::Token { contract_addr } => {
                        // Set allowance for the pair contract to spend CW20 tokens
                        let allowance_msg = cw20::Cw20ExecuteMsg::IncreaseAllowance {
                            spender: pair_address.clone(),
                            amount: asset.amount,
                            expires: None,
                        };
                        let allowance_wasm_msg =
                            wasm_execute(contract_addr, &allowance_msg, vec![])?;
                        response_msgs.push(allowance_wasm_msg);
                    }
                }
            }

            // Add the provide liquidity message
            let provide_wasm_msg =
                wasm_execute(pair_address.clone(), &provide_liquidity_msg, native_coins)?;
            response_msgs.push(provide_wasm_msg);

            let mut response = Response::new();
            for msg in response_msgs {
                response = response.add_message(msg);
            }

            Ok(response.add_attributes(vec![
                attr("action", "create_pair_and_provide_liquidity_success"),
                attr("pair_address", pair_address),
                attr("liquidity_assets", format!("{:?}", liquidity_state.assets)),
            ]))
        }
        ASTROPORT_MSG_PROVIDE_LIQUIDITY_ID => {
            if msg.result.is_err() {
                return Err(ContractError::Std(StdError::generic_err(format!(
                    "Astroport provide liquidity failed with error: {:?}",
                    msg.result.unwrap_err()
                ))));
            }

            Ok(Response::default()
                .add_attributes(vec![attr("action", "provide_liquidity_success")]))
        }
        _ => Err(ContractError::Std(StdError::generic_err(
            "Invalid Reply ID".to_string(),
        ))),
    }
}
