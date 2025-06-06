use andromeda_std::{
    ado_contract::ADOContract,
    amp::{
        messages::{AMPMsg, AMPPkt},
        AndrAddr, Recipient,
    },
    common::{context::ExecuteContext, denom::Asset},
    error::ContractError,
};
use cosmwasm_std::{
    attr, coin, ensure, to_json_binary, wasm_execute, Coin, Decimal, Deps, DepsMut, Env, Reply,
    Response, StdError, SubMsg, Uint128, WasmMsg,
};
use cw20::{BalanceResponse, Cw20ExecuteMsg, Cw20QueryMsg};

use crate::state::{ForwardReplyState, FORWARD_REPLY_STATE, PREV_BALANCE, SWAP_ROUTER};

use andromeda_socket::astroport::{
    AssetInfo, Cw20HookMsgAstroport, ExecuteMsgAstroport, QueryMsgAstroport,
    SimulateSwapOperationResponse, SwapOperation, SwapOperationAstroport,
};

pub const ASTROPORT_MSG_SWAP_ID: u64 = 1;
pub const ASTROPORT_MSG_FORWARD_ID: u64 = 2;
pub const ASTROPORT_MSG_CREATE_PAIR_ID: u64 = 3;
pub const ASTROPORT_MSG_CREATE_PAIR_AND_PROVIDE_LIQUIDITY_ID: u64 = 4;
pub const ASTROPORT_MSG_PROVIDE_LIQUIDITY_ID: u64 = 5;
pub const ASTROPORT_MSG_WITHDRAW_LIQUIDITY_ID: u64 = 6;

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

#[derive(Clone, Debug, PartialEq)]
pub struct AstroportSwapResponse {
    pub spread_amount: Uint128, // remaining Asset that is not consumed by the swap operation
    pub return_amount: Uint128, // amount of token_out swapped from astroport
}

pub fn generate_asset_info_from_asset(
    deps: &Deps,
    asset: Asset,
) -> Result<AssetInfo, ContractError> {
    match asset {
        Asset::Cw20Token(andr_addr) => {
            let contract_addr = andr_addr.get_raw_address(deps)?;
            Ok(AssetInfo::Token { contract_addr })
        }
        Asset::NativeToken(denom) => Ok(AssetInfo::NativeToken { denom }),
    }
}

pub fn handle_astroport_swap_reply(
    deps: DepsMut,
    env: Env,
    msg: Reply,
    state: ForwardReplyState,
) -> Result<Response, ContractError> {
    let balance = query_balance(&deps.as_ref(), &env, &state.to_asset)?;
    let prev_balance = PREV_BALANCE.load(deps.storage)?;
    let return_amount = balance.checked_sub(prev_balance)?;
    PREV_BALANCE.remove(deps.storage);

    if return_amount.is_zero() {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Incomplete data in Astroport swap response: {:?}",
            msg
        ))));
    }

    let mut resp = Response::default();

    let transfer_msg = match &state.to_asset {
        Asset::NativeToken(denom) => {
            let funds = vec![Coin {
                denom: denom.to_string(),
                amount: return_amount,
            }];

            let mut pkt = if let Some(amp_ctx) = state.amp_ctx.clone() {
                AMPPkt::new(amp_ctx.get_origin(), amp_ctx.get_previous_sender(), vec![])
            } else {
                AMPPkt::new(
                    env.contract.address.clone(),
                    env.contract.address.clone(),
                    vec![],
                )
            };

            let Recipient { address, msg, .. } = &state.recipient;
            let msg = AMPMsg::new(
                address.clone(),
                msg.clone().unwrap_or_default(),
                Some(funds.clone()),
            );

            pkt = pkt.add_message(msg);
            let kernel_address = ADOContract::default().get_kernel_address(deps.storage)?;
            pkt.to_sub_msg(kernel_address, Some(funds), ASTROPORT_MSG_FORWARD_ID)?
        }
        Asset::Cw20Token(andr_addr) => {
            let Recipient { address, msg, .. } = &state.recipient;
            let transfer_msg = match msg {
                Some(msg) => Cw20ExecuteMsg::Send {
                    contract: address.get_raw_address(&deps.as_ref())?.to_string(),
                    amount: return_amount,
                    msg: msg.clone(),
                },
                None => Cw20ExecuteMsg::Transfer {
                    recipient: address.get_raw_address(&deps.as_ref())?.to_string(),
                    amount: return_amount,
                },
            };
            let wasm_msg = wasm_execute(
                andr_addr.get_raw_address(&deps.as_ref())?,
                &transfer_msg,
                vec![],
            )?;
            SubMsg::new(wasm_msg)
        }
    };
    let kernel_address = ADOContract::default().get_kernel_address(deps.storage)?;
    resp = resp.add_submessage(transfer_msg).add_attributes(vec![
        attr("action", "swap_and_forward"),
        attr("dex", "astroport"),
        attr("to_denom", state.to_asset.to_string()),
        attr("to_amount", return_amount),
        attr("recipient", state.recipient.get_addr()),
        attr("kernel_address", kernel_address),
    ]);
    Ok(resp)
}

pub(crate) fn query_balance(
    deps: &Deps,
    env: &Env,
    asset: &Asset,
) -> Result<Uint128, ContractError> {
    let balance = match &asset {
        Asset::Cw20Token(andr_addr) => {
            let contract_addr = andr_addr.get_raw_address(deps)?;
            let res: BalanceResponse = deps.querier.query_wasm_smart(
                contract_addr,
                &Cw20QueryMsg::Balance {
                    address: env.contract.address.to_string(),
                },
            )?;
            res.balance
        }
        Asset::NativeToken(denom) => {
            deps.querier
                .query_balance(env.contract.address.to_string(), denom)?
                .amount
        }
    };
    Ok(balance)
}

pub fn query_simulate_astro_swap_operation(
    deps: Deps,
    offer_amount: Uint128,
    operations: Vec<SwapOperation>,
) -> Result<SimulateSwapOperationResponse, ContractError> {
    let operations: Vec<SwapOperationAstroport> = operations
        .iter()
        .map(|oper| {
            let astro_operation = SwapOperationAstroport::AstroSwap {
                offer_asset_info: generate_asset_info_from_asset(
                    &deps,
                    oper.offer_asset_info.clone(),
                )?,
                ask_asset_info: generate_asset_info_from_asset(&deps, oper.ask_asset_info.clone())?,
            };
            Ok(astro_operation)
        })
        .collect::<Result<Vec<SwapOperationAstroport>, ContractError>>()?;
    let query_msg = QueryMsgAstroport::SimulateSwapOperations {
        offer_amount,
        operations,
    };

    let swap_router = SWAP_ROUTER.load(deps.storage)?.get_raw_address(&deps)?;

    deps.querier
        .query_wasm_smart(swap_router, &query_msg)
        .map_err(ContractError::Std)
}
