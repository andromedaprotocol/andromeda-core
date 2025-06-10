use crate::astroport::generate_asset_info_from_asset;
use crate::state::{LP_PAIR_ADDRESS, PAIR_ADDRESS, SWAP_ROUTER};
use andromeda_socket::astroport::{
    LpPairAddressResponse, PairAddressResponse, QueryMsgAstroport, SimulateSwapOperationResponse,
    SwapOperation, SwapOperationAstroport,
};
use andromeda_std::error::ContractError;
#[cfg(not(feature = "library"))]
use cosmwasm_std::{Deps, Uint128};

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

pub fn query_pair_address(deps: Deps) -> Result<PairAddressResponse, ContractError> {
    let pair_address = PAIR_ADDRESS.may_load(deps.storage)?;
    Ok(PairAddressResponse {
        pair_address: pair_address.map(|addr| addr.to_string()),
    })
}

pub fn query_lp_pair_address(deps: Deps) -> Result<LpPairAddressResponse, ContractError> {
    let pair_address = LP_PAIR_ADDRESS.may_load(deps.storage)?;
    Ok(LpPairAddressResponse {
        lp_pair_address: pair_address,
    })
}
