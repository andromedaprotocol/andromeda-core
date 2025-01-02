use crate::{ado_contract::ADOContract, amp::messages::AMPPkt};
use cosmwasm_std::{DepsMut, Env, MessageInfo};

pub struct ExecuteContext<'a> {
    pub deps: DepsMut<'a>,
    pub info: MessageInfo,
    pub env: Env,
    pub amp_ctx: Option<AMPPkt>,
    pub contract: ADOContract<'a>,
    pub raw_info: MessageInfo,
}

impl ExecuteContext<'_> {
    #[inline]
    pub fn new(deps: DepsMut, info: MessageInfo, env: Env) -> ExecuteContext {
        ExecuteContext {
            deps,
            info: info.clone(),
            env,
            amp_ctx: None,
            contract: ADOContract::default(),
            raw_info: info,
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
