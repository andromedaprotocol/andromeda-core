use andromeda_finance::cross_chain_swap::{OsmosisSlippage, OsmosisSwapMsg};
use andromeda_std::{
    ado_contract::ADOContract, amp::OSMOSIS_ROUTER_KEY, common::context::ExecuteContext,
    error::ContractError, os::aos_querier::AOSQuerier,
};
use cosmwasm_std::{
    from_binary, wasm_execute, Coin, Decimal, Reply, StdError, SubMsg, SubMsgResponse, SubMsgResult,
};
use serde::de::DeserializeOwned;

// Adapted from: https://github.com/osmosis-labs/osmosis/blob/main/cosmwasm/contracts/crosschain-swaps/src/utils.rs#LL8C1-L23C2
// Parses a swap reply to the correct message type
pub(crate) fn parse_swap_reply<T: DeserializeOwned>(msg: Reply) -> Result<T, ContractError> {
    let SubMsgResult::Ok(SubMsgResponse { data: Some(b), .. }) = msg.result else {
        return Err(ContractError::Std(StdError::generic_err(
            "failed to parse swaprouter response",
        )))
    };

    let parsed = cw_utils::parse_execute_response_data(&b).map_err(|_e| {
        ContractError::Std(StdError::generic_err("failed to parse swaprouter response"))
    })?;
    let swap_response: T = from_binary(&parsed.data.unwrap_or_default())?;
    Ok(swap_response)
}

pub(crate) fn execute_swap_osmo(
    ctx: ExecuteContext,
    input_coin: Coin,
    to_denom: String,
    slippage_percentage: Decimal,
    window_seconds: Option<u64>,
) -> Result<SubMsg, ContractError> {
    let msg = OsmosisSwapMsg::Swap {
        input_coin: input_coin.clone(),
        output_denom: to_denom,
        slippage: OsmosisSlippage::Twap {
            window_seconds,
            slippage_percentage,
        },
    };

    let address = AOSQuerier::kernel_address_getter(
        &ctx.deps.querier,
        &ADOContract::default().get_kernel_address(ctx.deps.as_ref().storage)?,
        OSMOSIS_ROUTER_KEY,
    )?;

    let msg = wasm_execute(address, &msg, vec![input_coin])?;
    let sub_msg = SubMsg::new(msg);

    Ok(sub_msg)
}
