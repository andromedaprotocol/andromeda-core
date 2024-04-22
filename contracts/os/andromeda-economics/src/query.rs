use andromeda_std::{amp::AndrAddr, error::ContractError};
use cosmwasm_std::{Deps, Uint128};

use crate::state::BALANCES;

pub fn balance(deps: Deps, address: AndrAddr, asset: String) -> Result<Uint128, ContractError> {
    let addr = address.get_raw_address(&deps)?;
    let balance = BALANCES
        .load(deps.storage, (addr, asset))
        .unwrap_or_default();
    Ok(balance)
}
