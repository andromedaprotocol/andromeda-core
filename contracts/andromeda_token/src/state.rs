use andromeda_protocol::token::Token;
use cosmwasm_std::{Env, StdResult, Storage};
use cw721::Expiration;
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
pub const OPERATOR: Map<(String, String), Expiration> = Map::new("operator");

pub fn has_transfer_rights(
    storage: &dyn Storage,
    env: &Env,
    addr: String,
    token: &Token,
) -> StdResult<bool> {
    Ok(token.owner.eq(&addr)
        || has_approval(env, &addr, token)
        || is_operator(storage, env, token.owner.clone(), addr)?)
}

pub fn has_approval(env: &Env, addr: &String, token: &Token) -> bool {
    token
        .approvals
        .iter()
        .any(|a| a.spender.to_string().eq(addr) && !a.is_expired(&env.block))
}

pub fn is_operator(
    storage: &dyn Storage,
    env: &Env,
    owner: String,
    addr: String,
) -> StdResult<bool> {
    let expiry = OPERATOR.may_load(storage, (owner, addr))?;

    match expiry {
        None => Ok(false),
        Some(e) => Ok(!e.is_expired(&env.block)),
    }
}
