use cosmwasm_std::{DepsMut, Env, MessageInfo};

use crate::amp::messages::AMPPkt;

pub struct ExecuteContext<'a>(
    pub DepsMut<'a>,
    pub MessageInfo,
    pub Env,
    pub Option<AMPPkt>,
);
