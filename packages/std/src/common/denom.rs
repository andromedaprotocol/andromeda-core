use crate::{ado_contract::ADOContract, error::ContractError};
use cosmwasm_std::{ensure, DepsMut, Env};
pub const SEND_CW20_ACTION: &str = "SEND_CW20";

pub fn validate_denom(deps: DepsMut, env: Env, denom: String) -> Result<(), ContractError> {
    let potential_supply = deps.querier.query_supply(denom.clone())?;
    let non_empty_denom = !denom.is_empty();
    let non_zero_supply = !potential_supply.amount.is_zero();
    let valid_cw20 = ADOContract::default()
        .is_permissioned_strict(deps.storage, env, SEND_CW20_ACTION, denom.clone())
        .is_ok();
    ensure!(
        (non_empty_denom && non_zero_supply) || valid_cw20,
        ContractError::InvalidAsset { asset: denom }
    );

    Ok(())
}
