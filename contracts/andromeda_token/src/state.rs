use andromeda_protocol::token::Token;
use cosmwasm_std::{Addr, Env, StdError, StdResult, Storage};
use cw721::Expiration;
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TokenConfig {
    pub name: String,
    pub symbol: String,
    pub minter: String,
}

pub struct TokenIndexes<'a> {
    pub owner: MultiIndex<'a, (Addr, Vec<u8>), Option<Token>>,
}

impl<'a> IndexList<Option<Token>> for TokenIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Option<Token>>> + '_> {
        let v: Vec<&dyn Index<Option<Token>>> = vec![&self.owner];
        Box::new(v.into_iter())
    }
}

fn token_owner_index(t: &Option<Token>, k: Vec<u8>) -> (Addr, Vec<u8>) {
    match t {
        Some(token) => (Addr::unchecked(token.owner.clone()), k),
        None => (Addr::unchecked(""), k),
    }
}

pub fn tokens<'a>() -> IndexedMap<'a, String, Option<Token>, TokenIndexes<'a>> {
    IndexedMap::new(
        "ownership",
        TokenIndexes {
            owner: MultiIndex::new(token_owner_index, "ownership", "tokens_owner"),
        },
    )
}

pub const CONFIG: Item<TokenConfig> = Item::new("config");
pub const OPERATOR: Map<(String, String), Expiration> = Map::new("operator");
pub const NUM_TOKENS: Item<u64> = Item::new("numtokens");

pub fn mint_token(storage: &mut dyn Storage, token_id: String, token: Token) -> StdResult<()> {
    //Check if token with ID exists (may be None if token was burnt)
    let saved_token = tokens().may_load(storage, token_id.clone())?;
    if let Some(..) = saved_token {
        Err(StdError::generic_err("Token with given ID already exists"))
    } else {
        tokens().save(storage, token_id, &Some(token))
    }
}

pub fn load_token(storage: &dyn Storage, token_id: String) -> StdResult<Token> {
    let token = tokens().load(storage, token_id)?;

    if let Some(token) = token {
        Ok(token)
    } else {
        Err(StdError::not_found("Token"))
    }
}

pub fn has_transfer_rights(
    storage: &dyn Storage,
    env: &Env,
    addr: String,
    token: &Token,
) -> StdResult<bool> {
    Ok(token.owner.eq(&addr)
        || has_approval(env, &addr, token)
        || is_operator(storage, env, token.owner.clone(), addr.clone())?
        || has_transfer_agreement(addr, token))
}

pub fn has_approval(env: &Env, addr: &str, token: &Token) -> bool {
    token
        .approvals
        .iter()
        .any(|a| a.spender.to_string().eq(addr) && !a.is_expired(&env.block))
}

pub fn has_transfer_agreement(addr: String, token: &Token) -> bool {
    match token.transfer_agreement.clone() {
        None => false,
        Some(ag) => ag.purchaser == "*" || ag.purchaser.eq(&addr),
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
