use andromeda_std::{
    ado_contract::ADOContract,
    amp::{AndrAddr, Recipient},
    common::{context::ExecuteContext, denom::Asset},
    error::ContractError,
};
use cosmwasm_std::{coin, ensure, to_json_binary, wasm_execute, Decimal, SubMsg, Uint128, WasmMsg};
use cw20::Cw20ExecuteMsg;

use crate::{
    astroport::{
        build_liquidity_messages, generate_asset_info_from_asset, query_balance,
        ASTROPORT_MSG_CREATE_PAIR_AND_PROVIDE_LIQUIDITY_ID, ASTROPORT_MSG_CREATE_PAIR_ID,
        ASTROPORT_MSG_SWAP_ID, ASTROPORT_MSG_WITHDRAW_LIQUIDITY_ID,
    },
    state::{
        AstroportFactoryExecuteMsg, ForwardReplyState, LiquidityProvisionState, FACTORY,
        FORWARD_REPLY_STATE, LIQUIDITY_PROVISION_STATE, LP_PAIR_ADDRESS, PAIR_ADDRESS,
        PREV_BALANCE, SWAP_ROUTER, WITHDRAWAL_STATE,
    },
};

use andromeda_socket::astroport::{
    AssetEntry, AssetInfo, Cw20HookMsgAstroport, ExecuteMsgAstroport, PairExecuteMsg, PairType,
    SwapOperation, SwapOperationAstroport,
};
use cosmwasm_std::CosmosMsg;
#[cfg(not(feature = "library"))]
use cosmwasm_std::{attr, Binary, Response};

use cw_utils::one_coin;

#[allow(clippy::too_many_arguments)]
pub(crate) fn execute_swap_astroport_msg(
    ctx: ExecuteContext,
    from_asset: Asset,
    from_amount: Uint128,
    to_asset: Asset,
    recipient: Recipient,  // receiver where the swapped token goes to
    refund_addr: AndrAddr, // refund address
    max_spread: Option<Decimal>,
    minimum_receive: Option<Uint128>,
    operations: Option<Vec<SwapOperation>>,
) -> Result<SubMsg, ContractError> {
    let ExecuteContext { deps, env, .. } = ctx;

    // Prepare offer and ask asset
    ensure!(from_asset != to_asset, ContractError::DuplicateTokens {});
    let from_denom = match from_asset.clone() {
        Asset::NativeToken(denom) => denom,
        Asset::Cw20Token(andr_addr) => andr_addr.get_raw_address(&deps.as_ref())?.to_string(),
    };

    // Prepare swap operations
    let operations: Vec<SwapOperationAstroport> = operations
        .unwrap_or(vec![SwapOperation {
            offer_asset_info: from_asset.clone(),
            ask_asset_info: to_asset.clone(),
        }])
        .iter()
        .map(|oper| {
            let astro_operation = SwapOperationAstroport::AstroSwap {
                offer_asset_info: generate_asset_info_from_asset(
                    &deps.as_ref(),
                    oper.offer_asset_info.clone(),
                )?,
                ask_asset_info: generate_asset_info_from_asset(
                    &deps.as_ref(),
                    oper.ask_asset_info.clone(),
                )?,
            };
            Ok(astro_operation)
        })
        .collect::<Result<Vec<SwapOperationAstroport>, ContractError>>()?;
    ensure!(
        FORWARD_REPLY_STATE
            .may_load(deps.as_ref().storage)?
            .is_none(),
        ContractError::Unauthorized {}
    );

    let amp_ctx = if let Some(pkt) = ctx.amp_ctx.clone() {
        Some(pkt.ctx)
    } else {
        None
    };

    let prev_balance = query_balance(&deps.as_ref(), &env, &to_asset)?;
    FORWARD_REPLY_STATE.save(
        deps.storage,
        &ForwardReplyState {
            recipient,
            refund_addr,
            amp_ctx,
            from_asset: from_asset.clone(),
            to_asset: to_asset.clone(),
        },
    )?;
    PREV_BALANCE.save(deps.storage, &prev_balance)?;

    let swap_router = SWAP_ROUTER
        .load(deps.storage)?
        .get_raw_address(&deps.as_ref())?;
    // Build swap msg
    let msg = match from_asset {
        Asset::NativeToken(_) => {
            let astro_swap_msg = ExecuteMsgAstroport::ExecuteSwapOperations {
                operations,
                to: None,
                max_spread,
                minimum_receive,
            };
            WasmMsg::Execute {
                contract_addr: swap_router.to_string(),
                msg: to_json_binary(&astro_swap_msg)?,
                funds: vec![coin(from_amount.u128(), from_denom)],
            }
        }
        Asset::Cw20Token(cw20_contract) => {
            let astro_swap_hook_msg = Cw20HookMsgAstroport::ExecuteSwapOperations {
                operations,
                to: None,
                max_spread,
                minimum_receive,
            };

            let send_msg = Cw20ExecuteMsg::Send {
                contract: swap_router.to_string(),
                amount: from_amount,
                msg: to_json_binary(&astro_swap_hook_msg)?,
            };

            wasm_execute(
                cw20_contract.get_raw_address(&deps.as_ref())?,
                &send_msg,
                vec![],
            )?
        }
    };

    Ok(SubMsg::reply_always(msg, ASTROPORT_MSG_SWAP_ID))
}

#[allow(clippy::too_many_arguments)]
pub fn execute_swap_and_forward(
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

pub fn execute_update_swap_router(
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

pub fn create_factory_pair(
    ctx: ExecuteContext,
    pair_type: PairType,
    asset_infos: Vec<AssetInfo>,
    init_parameters: Option<Binary>,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, .. } = ctx;

    let factory_addr = FACTORY.load(deps.storage)?;

    let create_factory_pair_msg = AstroportFactoryExecuteMsg::CreatePair {
        pair_type: pair_type.clone(),
        asset_infos: asset_infos.clone(),
        init_params: init_parameters,
    };

    let wasm_msg = wasm_execute(factory_addr, &create_factory_pair_msg, vec![])?;

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

pub fn provide_liquidity(
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

    let provide_liquidity_msg = PairExecuteMsg::ProvideLiquidity {
        assets: assets.clone(),
        slippage_tolerance,
        auto_stake,
        receiver,
    };

    let response = Response::new().add_messages(build_liquidity_messages(
        &assets,
        pair_addr_raw.clone(),
        provide_liquidity_msg,
    )?);

    Ok(response.add_attributes(vec![
        attr("action", "provide_liquidity"),
        attr("pair_address", pair_addr_raw),
        attr("assets", format!("{:?}", assets)),
    ]))
}

#[allow(clippy::too_many_arguments)]
pub fn create_pair_and_provide_liquidity(
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

    let factory_addr: String = FACTORY.load(deps.storage)?;

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

    let wasm_msg = wasm_execute(factory_addr, &create_factory_pair_msg, vec![])?;

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

pub fn withdraw_liquidity(ctx: ExecuteContext) -> Result<Response, ContractError> {
    let ExecuteContext { deps, info, .. } = ctx;
    let lp_pair_address = LP_PAIR_ADDRESS.load(deps.storage)?;
    let lp_pair_address_raw = lp_pair_address.get_raw_address(&deps.as_ref())?;
    let funds = info.funds.first().unwrap();

    // Save withdrawal state to track the original sender
    WITHDRAWAL_STATE.save(deps.storage, &info.sender.to_string())?;

    let msg = AstroportFactoryExecuteMsg::WithdrawLiquidity {};

    let result = wasm_execute(lp_pair_address_raw, &msg, vec![funds.clone()])?;

    let sub_message =
        SubMsg::reply_always(CosmosMsg::Wasm(result), ASTROPORT_MSG_WITHDRAW_LIQUIDITY_ID);

    Ok(Response::new()
        .add_attributes(vec![
            attr("action", "withdraw_liquidity"),
            attr("pair_address", lp_pair_address.to_string()),
            attr("sender", info.sender.clone()),
        ])
        .add_submessage(sub_message))
}

#[allow(clippy::too_many_arguments)]
pub fn swap_and_forward_cw20(
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
