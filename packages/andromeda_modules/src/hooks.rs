use cosmwasm_std::{BankMsg, Coin, CosmosMsg, DepsMut, Env, MessageInfo, StdResult};

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
    fn pre_execute(&self, _deps: DepsMut, info: MessageInfo, _env: Env) -> StdResult<HookResponse> {
        Ok(HookResponse::default())
    }
    fn pre_publish(&self, _deps: DepsMut, _env: Env, _token_id: i64) -> StdResult<HookResponse> {
        Ok(HookResponse::default())
    }
    fn pre_transfer(
        &self,
        _deps: DepsMut,
        _env: Env,
        _token_id: i64,
        _from: String,
        _to: String,
    ) -> StdResult<HookResponse> {
        Ok(HookResponse::default())
    }
    fn pre_transfer_agreement(
        &self,
        _deps: DepsMut,
        _env: Env,
        _token_id: i64,
        _amount: Coin,
        _buyer: String,
    ) -> StdResult<HookResponse> {
        Ok(HookResponse::default())
    }
    fn pre_burn(&self, _deps: DepsMut, _env: Env, _token_id: i64) -> StdResult<HookResponse> {
        Ok(HookResponse::default())
    }
    fn pre_archive(&self, _deps: DepsMut, _env: Env, _token_id: i64) -> StdResult<HookResponse> {
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
