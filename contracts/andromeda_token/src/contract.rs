use andromeda_modules::hooks::PreHooks;
use andromeda_modules::modules::{read_modules, store_modules};
use andromeda_protocol::token::{HandleMsg, InitMsg, OwnerResponse, QueryMsg};
use cosmwasm_std::{
    to_binary, Api, Binary, CosmosMsg, Env, Extern, HandleResponse, InitResponse, Querier,
    StdResult, Storage, WasmMsg,
};

use crate::state::{get_owner, store_config, store_owner, TokenConfig};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    msg.validate()?;

    let config = TokenConfig {
        name: msg.name,
        symbol: msg.symbol,
        creator: msg.creator,
    };

    store_config(&mut deps.storage, &config)?;
    store_modules(&mut deps.storage, msg.modules)?;

    match msg.init_hook {
        Some(hook) => Ok(InitResponse {
            messages: vec![CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: hook.contract_addr,
                msg: hook.msg,
                send: vec![],
            })],
            log: vec![],
        }),
        None => Ok(InitResponse::default()),
    }
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    let modules = read_modules(&deps.storage)?;
    for module in modules {
        module.pre_handle(deps, env.clone())?;
    }

    match msg {
        HandleMsg::Mint { token_id } => mint(deps, env, token_id),
    }
}

pub fn mint<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    token_id: i64,
) -> StdResult<HandleResponse> {
    let modules = read_modules(&deps.storage)?;
    for module in modules {
        module.pre_publish(deps, env.clone(), token_id)?;
    }

    let sender = env.message.sender;

    store_owner(&mut deps.storage, &token_id, &sender.clone())?;
    Ok(HandleResponse::default())
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetOwner { token_id } => to_binary(&query_owner(deps, token_id)?),
    }
}

fn query_owner<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    token_id: i64,
) -> StdResult<OwnerResponse> {
    let owner = get_owner(&deps.storage, &token_id)?;
    Ok(OwnerResponse {
        owner: owner.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{coins, from_binary, HumanAddr};

    const TOKEN_NAME: &str = "test";
    const TOKEN_SYMBOL: &str = "T";

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {
            name: TOKEN_NAME.to_string(),
            symbol: TOKEN_SYMBOL.to_string(),
            modules: vec![],
            creator: HumanAddr::from("creator"),
            init_hook: None,
        };

        let env = mock_env("creator", &coins(1000, "earth"));

        let res = init(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn test_mint() {
        let mut deps = mock_dependencies(20, &[]);
        let env = mock_env("owner", &coins(1000, "earth"));
        let token_id = 1;

        let msg = HandleMsg::Mint { token_id };

        handle(&mut deps, env, msg).unwrap();

        let query_msg = QueryMsg::GetOwner { token_id };

        let query_res = query(&deps, query_msg).unwrap();
        let query_val: OwnerResponse = from_binary(&query_res).unwrap();

        assert_eq!(query_val.owner, HumanAddr::from("owner"))
    }
}
