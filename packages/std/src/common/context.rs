use crate::{amp::messages::AMPPkt, error::ContractError};
use cosmwasm_std::{DepsMut, Env, MessageInfo};
pub struct ExecuteContext<'a> {
    pub deps: DepsMut<'a>,
    pub info: MessageInfo,
    pub env: Env,
    pub amp_ctx: Option<AMPPkt>,
}

impl<'a> ExecuteContext<'a> {
    #[inline]
    pub fn new(deps: DepsMut, info: MessageInfo, env: Env) -> ExecuteContext {
        ExecuteContext {
            deps,
            info,
            env,
            amp_ctx: None,
        }
    }

    pub fn with_ctx(mut self, amp_ctx: AMPPkt) -> Self {
        self.amp_ctx = Some(amp_ctx);
        self
    }
}
