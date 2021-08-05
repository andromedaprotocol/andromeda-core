use cosmwasm_std::{BankMsg, Coin, CosmosMsg, DepsMut, Env, MessageInfo, StdResult};
use cw721::Expiration;

#[derive(Debug, PartialEq)]
pub struct HookResponse {
    pub msgs: Vec<CosmosMsg>,
}

impl HookResponse {
    pub fn default() -> Self {
        HookResponse { msgs: vec![] }
    }
}

pub trait PreHooks {
    fn pre_execute(
        &self,
        _deps: &DepsMut,
        _info: MessageInfo,
        _env: Env,
    ) -> StdResult<HookResponse> {
        Ok(HookResponse::default())
    }
    fn pre_publish(
        &self,
        _deps: &DepsMut,
        _env: Env,
        _token_id: String,
    ) -> StdResult<HookResponse> {
        Ok(HookResponse::default())
    }
    fn pre_transfer(
        &self,
        _deps: &DepsMut,
        _env: Env,
        _recipient: String,
        _token_id: String,
    ) -> StdResult<HookResponse> {
        Ok(HookResponse::default())
    }
    fn pre_send(
        &self,
        _deps: &DepsMut,
        _env: Env,
        _contract: String,
        _token_id: String,
    ) -> StdResult<HookResponse> {
        Ok(HookResponse::default())
    }
    fn pre_approve(
        &self,
        _deps: &DepsMut,
        _env: Env,
        _sender: String,
        _token_id: String,
        _expires: Option<Expiration>,
    ) -> StdResult<HookResponse> {
        Ok(HookResponse::default())
    }
    fn pre_revoke(
        &self,
        _deps: &DepsMut,
        _env: Env,
        _sender: String,
        _token_id: String,
    ) -> StdResult<HookResponse> {
        Ok(HookResponse::default())
    }
    fn pre_transfer_agreement(
        &self,
        _deps: DepsMut,
        _env: Env,
        _token_id: String,
        _amount: Coin,
        _buyer: String,
    ) -> StdResult<HookResponse> {
        Ok(HookResponse::default())
    }
    fn pre_burn(&self, _deps: DepsMut, _env: Env, _token_id: String) -> StdResult<HookResponse> {
        Ok(HookResponse::default())
    }
    fn pre_archive(&self, _deps: DepsMut, _env: Env, _token_id: String) -> StdResult<HookResponse> {
        Ok(HookResponse::default())
    }
}

pub trait Payments {
    fn on_agreed_transfer(
        &self,
        _env: Env,
        _payments: &mut Vec<BankMsg>,
        _owner: String,
        _purchaser: String,
        _amount: Coin,
    ) -> StdResult<bool> {
        Ok(true)
    }
}
