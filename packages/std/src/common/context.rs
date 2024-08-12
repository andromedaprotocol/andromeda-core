use crate::amp::messages::AMPPkt;
use cosmwasm_std::{CustomQuery, DepsMut, Env, MessageInfo};

pub struct ExecuteContext<'a, C: CustomQuery> {
    pub deps: DepsMut<'a, C>,
    pub info: MessageInfo,
    pub env: Env,
    pub amp_ctx: Option<AMPPkt>,
}

impl<'a, C: CustomQuery> ExecuteContext<'a, C> {
    #[inline]
    pub fn new(deps: DepsMut<C>, info: MessageInfo, env: Env) -> ExecuteContext<C> {
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

    pub fn contains_sender(&self, addr: &str) -> bool {
        if self.info.sender == addr {
            return true;
        }

        match &self.amp_ctx {
            None => false,
            Some(ctx) => ctx.ctx.get_origin() == addr || ctx.ctx.get_previous_sender() == addr,
        }
    }
}
