use crate::error::ContractError;
use cosmwasm_std::{ensure, Deps};
pub const SEND_CW20_ACTION: &str = "SEND_CW20";

pub fn validate_denom(deps: Deps, denom: String) -> Result<(), ContractError> {
    let potential_supply = deps.querier.query_supply(denom.clone())?;
    let non_empty_denom = !denom.is_empty();
    let non_zero_supply = !potential_supply.amount.is_zero();
    ensure!(
        non_empty_denom && non_zero_supply,
        ContractError::InvalidAsset { asset: denom }
    );

    Ok(())
}
