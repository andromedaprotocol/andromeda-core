use andromeda_protocol::token::Token;
use cosmwasm_std::{Env, StdError, StdResult, Storage};
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
pub const NUM_TOKENS: Item<u64> = Item::new("numtokens");

pub fn has_transfer_rights(
    storage: &dyn Storage,
    env: &Env,
    addr: String,
    token: &Token,
) -> StdResult<bool> {
    Ok(token.owner.eq(&addr)
        || has_approval(env, &addr, token)
        || is_operator(storage, env, token.owner.clone(), addr.clone())?
        || has_transfer_agreement(addr.clone(), token))
}

pub fn has_approval(env: &Env, addr: &String, token: &Token) -> bool {
    token
        .approvals
        .iter()
        .any(|a| a.spender.to_string().eq(addr) && !a.is_expired(&env.block))
}

pub fn has_transfer_agreement(addr: String, token: &Token) -> bool {
    match token.transfer_agreement.clone() {
        None => false,
        Some(ag) => ag.purchaser.eq(&addr),
    }
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

pub fn increment_num_tokens(storage: &mut dyn Storage) -> StdResult<()> {
    let token_count = NUM_TOKENS.load(storage).unwrap_or_default();
    NUM_TOKENS.save(storage, &(token_count + 1))
}

pub fn read_config(storage: &dyn Storage) -> StdResult<TokenConfig> {
    CONFIG.load(storage)
}

pub fn store_config(storage: &mut dyn Storage, config: &TokenConfig) -> StdResult<()> {
    CONFIG.save(storage, config)
}

pub fn decrement_num_tokens(storage: &mut dyn Storage) -> StdResult<()> {
    let token_count = NUM_TOKENS.load(storage).unwrap_or_default();
    if token_count == 0 {
        Err(StdError::generic_err(
            "Cannot decrement token count below 0",
        ))
    } else {
        NUM_TOKENS.save(storage, &(token_count - 1))
    }
}
