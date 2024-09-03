use std::fmt::{Display, Formatter, Result as StdResult};

use crate::{ado_contract::ADOContract, amp::AndrAddr, error::ContractError};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    coin, ensure, to_json_binary, wasm_execute, BankMsg, Deps, DepsMut, Env, QueryRequest, SubMsg,
    Uint128, WasmQuery,
};
use cw20::{Cw20ExecuteMsg, Cw20QueryMsg, TokenInfoResponse};
pub const SEND_CW20_ACTION: &str = "SEND_CW20";
pub const SEND_NFT_ACTION: &str = "SEND_NFT";

#[cw_serde]
pub enum Asset {
    Cw20Token(AndrAddr),
    NativeToken(String),
}

impl Display for Asset {
    fn fmt(&self, f: &mut Formatter) -> StdResult {
        match self {
            Asset::NativeToken(addr) => f.write_str(&format!("native:{addr}")),
            Asset::Cw20Token(addr) => f.write_str(&format!("cw20:{addr}")),
        }
    }
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
                let token_info_query: TokenInfoResponse =
                    deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                        contract_addr: cw20_token.to_string(),
                        msg: to_json_binary(&Cw20QueryMsg::TokenInfo {})?,
                    }))?;
                ensure!(
                    !token_info_query.total_supply.is_zero(),
                    ContractError::InvalidZeroAmount {}
                );
                let valid_cw20_auction = ADOContract::default()
                    .is_permissioned(deps, env, SEND_CW20_ACTION, cw20_token.clone())
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
    pub fn transfer(
        &self,
        deps: &Deps,
        to_address: impl Into<String>,
        amount: Uint128,
    ) -> Result<SubMsg, ContractError> {
        let to_address: String = to_address.into();

        Ok(match self {
            Asset::NativeToken(denom) => SubMsg::new(BankMsg::Send {
                to_address,
                amount: vec![coin(amount.u128(), denom)],
            }),
            Asset::Cw20Token(denom) => {
                let denom = denom.get_raw_address(deps)?;
                let transfer_msg = Cw20ExecuteMsg::Transfer {
                    recipient: to_address,
                    amount,
                };
                let wasm_msg = wasm_execute(denom, &transfer_msg, vec![])?;
                SubMsg::new(wasm_msg)
            }
        })
    }

    pub fn burn(&self, deps: &Deps, amount: Uint128) -> Result<SubMsg, ContractError> {
        Ok(match self {
            Asset::NativeToken(denom) => SubMsg::new(BankMsg::Burn {
                amount: vec![coin(amount.u128(), denom)],
            }),
            Asset::Cw20Token(denom) => {
                let denom = denom.get_raw_address(deps)?;
                let burn_msg = Cw20ExecuteMsg::Burn { amount };
                let wasm_msg = wasm_execute(denom, &burn_msg, vec![])?;
                SubMsg::new(wasm_msg)
            }
        })
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
