use cosmwasm_std::{BankMsg, Coin, CosmosMsg, DepsMut, Env, MessageInfo, StdResult};
use cw721::Expiration;

use crate::token::ExecuteMsg;

#[derive(Debug, PartialEq)]
pub struct HookResponse {
    pub msgs: Vec<CosmosMsg>,
}

impl HookResponse {
    pub fn default() -> Self {
        HookResponse { msgs: vec![] }
    }
}

pub trait MessageHooks {
    fn on_execute(
        &self,
        _deps: &DepsMut,
        _info: MessageInfo,
        _env: Env,
        _msg: ExecuteMsg,
    ) -> StdResult<HookResponse> {
        Ok(HookResponse::default())
    }
    fn on_mint(
        &self,
        _deps: &DepsMut,
        _info: MessageInfo,
        _env: Env,
        _token_id: String,
    ) -> StdResult<HookResponse> {
        Ok(HookResponse::default())
    }
    fn on_transfer(
        &self,
        _deps: &DepsMut,
        _info: MessageInfo,
        _env: Env,
        _recipient: String,
        _token_id: String,
    ) -> StdResult<HookResponse> {
        Ok(HookResponse::default())
    }
    fn on_send(
        &self,
        _deps: &DepsMut,
        _info: MessageInfo,
        _env: Env,
        _contract: String,
        _token_id: String,
    ) -> StdResult<HookResponse> {
        Ok(HookResponse::default())
    }
    fn on_approve(
        &self,
        _deps: &DepsMut,
        _info: MessageInfo,
        _env: Env,
        _spender: String,
        _token_id: String,
        _expires: Option<Expiration>,
    ) -> StdResult<HookResponse> {
        Ok(HookResponse::default())
    }
    fn on_revoke(
        &self,
        _deps: &DepsMut,
        _info: MessageInfo,
        _env: Env,
        _sender: String,
        _token_id: String,
    ) -> StdResult<HookResponse> {
        Ok(HookResponse::default())
    }
    fn on_approve_all(
        &self,
        _deps: &DepsMut,
        _info: MessageInfo,
        _env: Env,
        _operator: String,
        _expires: Option<Expiration>,
    ) -> StdResult<HookResponse> {
        Ok(HookResponse::default())
    }
    fn on_revoke_all(
        &self,
        _deps: &DepsMut,
        _info: MessageInfo,
        _env: Env,
        _operator: String,
    ) -> StdResult<HookResponse> {
        Ok(HookResponse::default())
    }
    fn on_transfer_agreement(
        &self,
        _deps: &DepsMut,
        _info: MessageInfo,
        _env: Env,
        _token_id: String,
        _purchaser: String,
        _amount: u128,
        _denom: String,
    ) -> StdResult<HookResponse> {
        Ok(HookResponse::default())
    }
    fn on_burn(
        &self,
        _deps: &DepsMut,
        _info: MessageInfo,
        _env: Env,
        _token_id: String,
    ) -> StdResult<HookResponse> {
        Ok(HookResponse::default())
    }
    fn on_archive(
        &self,
        _deps: &DepsMut,
        _info: MessageInfo,
        _env: Env,
        _token_id: String,
    ) -> StdResult<HookResponse> {
        Ok(HookResponse::default())
    }
    fn on_agreed_transfer(
        &self,
        _deps: &DepsMut,
        _info: MessageInfo,
        _env: Env,
        _payments: &mut Vec<BankMsg>,
        _owner: String,
        _purchaser: String,
        _amount: Coin,
    ) -> StdResult<bool> {
        Ok(true)
    }
}
