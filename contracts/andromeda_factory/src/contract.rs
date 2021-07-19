use andromeda_extensions::extension::Extension;
use andromeda_protocol::{
    factory::{AddressResponse, HandleMsg, InitMsg, QueryMsg},
    hook::InitHook,
    require::require,
    token::InitMsg as TokenInitMsg,
};
use cosmwasm_std::{
    to_binary, Api, Binary, CanonicalAddr, CosmosMsg, Env, Extern, HandleResponse, InitResponse,
    Querier, StdError, StdResult, Storage, WasmMsg, HumanAddr
};

use crate::state::{
    is_address_defined, read_address, read_config, store_address, store_config, Config, store_creator, read_creator
};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    store_config(
        &mut deps.storage,
        &Config {
            token_code_id: msg.token_code_id,
            owner: CanonicalAddr::default(),
        },
    )?;

    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::Create {
            symbol,
            name,
            extensions,
        } => create(deps, env, name, symbol, extensions),
        HandleMsg::TokenCreationHook { symbol, creator } => token_creation(deps, env, symbol, creator),
    }
}

pub fn create<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    name: String,
    symbol: String,
    extensions: Vec<Extension>,
) -> StdResult<HandleResponse> {
    let config = read_config(&deps.storage)?;

    require(
        !is_address_defined(&deps.storage, symbol.to_string())?,
        StdError::unauthorized(),
    )?;

    Ok(HandleResponse {
        messages: vec![CosmosMsg::Wasm(WasmMsg::Instantiate {
            code_id: config.token_code_id,
            send: vec![],
            label: None,
            msg: to_binary(&TokenInitMsg {
                name: name.to_string(),
                symbol: symbol.to_string(),
                creator: env.message.sender.clone(),
                extensions,
                init_hook: Some(InitHook {
                    msg: to_binary(&HandleMsg::TokenCreationHook {
                        symbol: symbol.to_string(),
                        creator: env.message.sender.clone(),
                    })?,
                    contract_addr: env.contract.address,
                }),
            })?,
        })],
        log: vec![],
        data: None,
    })
}

pub fn token_creation<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    symbol: String,
    creator: HumanAddr,
) -> StdResult<HandleResponse> {
    //TODO: VALIDATE IF MESSAGE FROM CONTRACT
    require(
        !is_address_defined(&deps.storage, symbol.to_string())?,
        StdError::unauthorized(),
    )?;

    let address = env.message.sender;

    store_address(&mut deps.storage, symbol.to_string(), address)?;
    store_creator(&mut deps.storage, &symbol.to_string(), &creator)?;

    Ok(HandleResponse::default())
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetAddress { symbol } => to_binary(&query_address(deps, symbol)?),
    }
}

fn query_address<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    symbol: String,
) -> StdResult<AddressResponse> {
    let address = read_address(&deps.storage, symbol)?;
    Ok(AddressResponse {
        address: address.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{coins, from_binary};

    static TOKEN_CODE_ID: u64 = 0;
    const TOKEN_NAME: &str = "test";
    const TOKEN_SYMBOL: &str = "T";

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {
            token_code_id: TOKEN_CODE_ID,
        };
        let env = mock_env("creator", &coins(1000, "earth"));

        let res = init(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn test_create() {
        let mut deps = mock_dependencies(20, &[]);
        let env = mock_env("creator", &coins(1000, "earth"));

        let init_msg = InitMsg {
            token_code_id: TOKEN_CODE_ID,
        };

        let res = init(&mut deps, env, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        let msg = HandleMsg::Create {
            name: TOKEN_NAME.to_string(),
            symbol: TOKEN_SYMBOL.to_string(),
            extensions: vec![],
        };

        let expected_msg = CosmosMsg::Wasm(WasmMsg::Instantiate {
            code_id: TOKEN_CODE_ID,
            send: vec![],
            label: None,
            msg: to_binary(&TokenInitMsg {
                name: TOKEN_NAME.to_string(),
                symbol: TOKEN_SYMBOL.to_string(),
                creator: HumanAddr::from("creator"),
                extensions: vec![],
                init_hook: Some(InitHook {
                    msg: to_binary(&HandleMsg::TokenCreationHook {
                        symbol: TOKEN_SYMBOL.to_string(),
                        creator: HumanAddr::from("creator")
                    })
                    .unwrap(),
                    contract_addr: mock_env("creator", &coins(1000, "earth")).contract.address,
                }),
            })
            .unwrap(),
        });

        let res = handle(&mut deps, mock_env("creator", &coins(1000, "earth")), msg).unwrap();
        assert_eq!(
            res,
            HandleResponse {
                messages: vec![expected_msg],
                log: vec![],
                data: None
            }
        )
    }

    #[test]
    fn test_token_creation() {
        let mut deps = mock_dependencies(20, &[]);
        let env = mock_env("creator", &coins(1000, "earth"));

        let msg = HandleMsg::TokenCreationHook {
            symbol: TOKEN_SYMBOL.to_string(),
            creator: HumanAddr::from("creator")
        };

        let res = handle(&mut deps, env.clone(), msg).unwrap();

        assert_eq!(res, HandleResponse::default());

        let query_msg = QueryMsg::GetAddress {
            symbol: TOKEN_SYMBOL.to_string(),
        };

        let addr_res = query(&deps, query_msg).unwrap();
        let addr_val: AddressResponse = from_binary(&addr_res).unwrap();

        assert_eq!(env.message.sender, addr_val.address);
        let creator = match read_creator(&deps.storage, TOKEN_SYMBOL.to_string()) {
            Ok(addr) => addr,
            _ => HumanAddr::default()
        };
        assert_eq!(env.message.sender, creator)
    }
}
