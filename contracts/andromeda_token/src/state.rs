use andromeda_protocol::token::Token;
use cosmwasm_std::{Env, StdResult};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TokenConfig {
    pub name: String,
    pub symbol: String,
    pub minter: String,
}

pub const CONFIG: Item<TokenConfig> = Item::new("config");
pub const TOKENS: Map<String, Token> = Map::new("ownership");

pub fn has_transfer_rights(env: &Env, addr: String, token: &Token) -> StdResult<bool> {
    Ok(token.owner.eq(&addr)
        || token
            .approvals
            .iter()
            .any(|a| a.spender.to_string().eq(&addr) && !a.is_expired(&env.block)))
}
