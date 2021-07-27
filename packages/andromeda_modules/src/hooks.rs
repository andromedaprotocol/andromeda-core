use cosmwasm_std::{
    Api, Coin, CosmosMsg, Env, Extern, HumanAddr, LogAttribute, Querier, StdResult, Storage,
};

#[derive(Debug, PartialEq)]
pub struct HookResponse {
    pub msgs: Vec<CosmosMsg>,
    pub logs: Vec<LogAttribute>,
}

impl HookResponse {
    pub fn default() -> Self {
        HookResponse {
            msgs: vec![],
            logs: vec![],
        }
    }
}

pub trait PreHooks {
    fn pre_handle<S: Storage, A: Api, Q: Querier>(
        &self,
        _deps: &mut Extern<S, A, Q>,
        _env: Env,
    ) -> StdResult<HookResponse> {
        Ok(HookResponse::default())
    }
    fn pre_publish<S: Storage, A: Api, Q: Querier>(
        &self,
        _deps: &mut Extern<S, A, Q>,
        _env: Env,
        _token_id: i64,
    ) -> StdResult<HookResponse> {
        Ok(HookResponse::default())
    }
    fn pre_transfer<S: Storage, A: Api, Q: Querier>(
        &self,
        _deps: &mut Extern<S, A, Q>,
        _env: Env,
        _token_id: i64,
        _from: HumanAddr,
        _to: HumanAddr,
    ) -> StdResult<HookResponse> {
        Ok(HookResponse::default())
    }
    fn pre_transfer_agreement<S: Storage, A: Api, Q: Querier>(
        &self,
        _deps: &mut Extern<S, A, Q>,
        _env: Env,
        _token_id: i64,
        _amount: Coin,
        _buyer: HumanAddr,
    ) -> StdResult<HookResponse> {
        Ok(HookResponse::default())
    }
    fn pre_burn<S: Storage, A: Api, Q: Querier>(
        &self,
        _deps: &mut Extern<S, A, Q>,
        _env: Env,
        _token_id: i64,
    ) -> StdResult<HookResponse> {
        Ok(HookResponse::default())
    }
    fn pre_archive<S: Storage, A: Api, Q: Querier>(
        &self,
        _deps: &mut Extern<S, A, Q>,
        _env: Env,
        _token_id: i64,
    ) -> StdResult<HookResponse> {
        Ok(HookResponse::default())
    }
}
