use crate::{ado_contract::ADOContract, amp::AndrAddr, error::ContractError};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{ensure, Deps, DepsMut, Env};
pub const SEND_CW20_ACTION: &str = "SEND_CW20";

#[cw_serde]
pub enum Asset {
    Cw20Token(AndrAddr),
    NativeToken(String),
}
impl Asset {
    pub fn get_verified_asset(
        &self,
        deps: DepsMut,
        env: Env,
    ) -> Result<(String, bool), ContractError> {
        match self {
            Asset::Cw20Token(cw20_token) => {
                let cw20_token = cw20_token.get_raw_address(&deps.as_ref())?;
                let valid_cw20_auction = ADOContract::default()
                    .is_permissioned(deps.storage, env, SEND_CW20_ACTION, cw20_token.clone())
                    .is_ok();
                ensure!(
                    valid_cw20_auction,
                    ContractError::InvalidFunds {
                        msg: format!("Non-permissioned CW20 asset '{}' set as denom.", cw20_token)
                    }
                );
                Ok((cw20_token.to_string(), true))
            }
            Asset::NativeToken(native) => {
                validate_denom(deps.as_ref(), native.clone())?;
                Ok((native.to_string(), false))
            }
        }
    }
}

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
