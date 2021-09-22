use andromeda_protocol::{
    factory::{AddressResponse, ExecuteMsg, InstantiateMsg, QueryMsg},
    hook::InitHook,
    modules::ModuleDefinition,
    require::require,
    token::InstantiateMsg as TokenInstantiateMsg,
};
use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
    WasmMsg,
};

use crate::state::{
    is_address_defined, is_creator, read_address, read_config, store_address, store_config,
    store_creator, Config,
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    store_config(
        deps.storage,
        &Config {
            token_code_id: msg.token_code_id,
            receipt_code_id: msg.receipt_code_id,
            owner: String::default(),
        },
    )?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::Create {
            symbol,
            name,
            extensions,
            metadata_limit,
        } => create(deps, env, info, name, symbol, extensions, metadata_limit),
        ExecuteMsg::TokenCreationHook { symbol, creator } => {
            token_creation(deps, env, info, symbol, creator)
        }
        ExecuteMsg::UpdateAddress {
            symbol,
            new_address,
        } => update_address(deps, env, info, symbol, new_address),
    }
}

pub fn create(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    name: String,
    symbol: String,
    modules: Vec<ModuleDefinition>,
    metadata_limit: Option<u64>,
) -> StdResult<Response> {
    let config = read_config(deps.storage)?;

    require(
        !is_address_defined(deps.storage, symbol.to_string())?,
        StdError::generic_err("Symbol is in use"),
    )?;

    Ok(Response::new().add_message(WasmMsg::Instantiate {
        code_id: config.token_code_id,
        funds: vec![],
        label: "".to_string(),
        admin: None,
        msg: to_binary(&TokenInstantiateMsg {
            name: name.to_string(),
            symbol: symbol.to_string(),
            minter: info.sender.to_string(),
            modules,
            receipt_code_id: config.receipt_code_id,
            init_hook: Some(InitHook {
                msg: to_binary(&ExecuteMsg::TokenCreationHook {
                    symbol: symbol.to_string(),
                    creator: info.sender.to_string(),
                })?,
                contract_addr: info.sender.to_string(),
            }),
            metadata_limit,
        })?,
    }))
}

pub fn token_creation(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    symbol: String,
    creator: String,
) -> StdResult<Response> {
    require(
        !is_address_defined(deps.storage, symbol.to_string())?,
        StdError::generic_err("Symbol already has a defined address"),
    )?;

    let address = info.sender.to_string();

    store_address(deps.storage, symbol.to_string(), &address)?;
    store_creator(deps.storage, symbol.to_string(), &creator)?;

    Ok(Response::default())
}

pub fn update_address(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    symbol: String,
    new_address: String,
) -> StdResult<Response> {
    require(
        is_creator(deps.storage, symbol.clone(), info.sender.to_string())?,
        StdError::generic_err("Cannot update address for ADO that you did not create"),
    )?;

    store_address(deps.storage, symbol.clone(), &new_address.clone())?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetAddress { symbol } => to_binary(&query_address(deps, symbol)?),
    }
}

fn query_address(deps: Deps, symbol: String) -> StdResult<AddressResponse> {
    let address = read_address(deps.storage, symbol)?;
    Ok(AddressResponse {
        address: address.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::read_creator;
    use cosmwasm_std::from_binary;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    static TOKEN_CODE_ID: u64 = 0;
    static RECEIPT_CODE_ID: u64 = 1;

    const TOKEN_NAME: &str = "test";
    const TOKEN_SYMBOL: &str = "T";

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            token_code_id: TOKEN_CODE_ID,
            receipt_code_id: RECEIPT_CODE_ID,
        };
        let env = mock_env();

        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn test_create() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);

        let init_msg = InstantiateMsg {
            token_code_id: TOKEN_CODE_ID,
            receipt_code_id: RECEIPT_CODE_ID,
        };

        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        let msg = ExecuteMsg::Create {
            name: TOKEN_NAME.to_string(),
            symbol: TOKEN_SYMBOL.to_string(),
            extensions: vec![],
            metadata_limit: None,
        };

        let expected_msg = WasmMsg::Instantiate {
            code_id: TOKEN_CODE_ID,
            funds: vec![],
            label: "".to_string(),
            admin: None,
            msg: to_binary(&TokenInstantiateMsg {
                name: TOKEN_NAME.to_string(),
                symbol: TOKEN_SYMBOL.to_string(),
                minter: String::from("creator"),
                modules: vec![],
                receipt_code_id: RECEIPT_CODE_ID,
                init_hook: Some(InitHook {
                    msg: to_binary(&ExecuteMsg::TokenCreationHook {
                        symbol: TOKEN_SYMBOL.to_string(),
                        creator: String::from("creator"),
                    })
                    .unwrap(),
                    contract_addr: info.sender.to_string(),
                }),
                metadata_limit: None,
            })
            .unwrap(),
        };

        let expected_res = Response::new().add_message(expected_msg);

        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        assert_eq!(res, expected_res);
        assert_eq!(1, expected_res.messages.len())
    }

    #[test]
    fn test_token_creation() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);

        let msg = ExecuteMsg::TokenCreationHook {
            symbol: TOKEN_SYMBOL.to_string(),
            creator: String::from("creator"),
        };

        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        assert_eq!(res, Response::default());

        let query_msg = QueryMsg::GetAddress {
            symbol: TOKEN_SYMBOL.to_string(),
        };

        let addr_res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
        let addr_val: AddressResponse = from_binary(&addr_res).unwrap();

        assert_eq!(info.sender, addr_val.address);
        let creator = match read_creator(&deps.storage, TOKEN_SYMBOL.to_string()) {
            Ok(addr) => addr,
            _ => String::default(),
        };
        assert_eq!(info.sender, creator)
    }

    #[test]
    fn test_update_address() {
        let creator = String::from("creator");
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info(creator.clone().as_str(), &[]);

        let msg = ExecuteMsg::TokenCreationHook {
            symbol: TOKEN_SYMBOL.to_string(),
            creator: creator.clone(),
        };

        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        assert_eq!(res, Response::default());

        let new_address = String::from("new");
        let update_msg = ExecuteMsg::UpdateAddress {
            symbol: TOKEN_SYMBOL.to_string(),
            new_address: new_address.clone(),
        };

        let update_res =
            execute(deps.as_mut(), env.clone(), info.clone(), update_msg.clone()).unwrap();

        assert_eq!(update_res, Response::default());

        let query_msg = QueryMsg::GetAddress {
            symbol: TOKEN_SYMBOL.to_string(),
        };

        let addr_res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
        let addr_val: AddressResponse = from_binary(&addr_res).unwrap();

        assert_eq!(new_address.clone(), addr_val.address);

        let unauth_env = mock_env();
        let unauth_info = mock_info("anyone", &[]);
        let unauth_res = execute(
            deps.as_mut(),
            unauth_env.clone(),
            unauth_info.clone(),
            update_msg.clone(),
        )
        .unwrap_err();

        assert_eq!(
            unauth_res,
            StdError::generic_err("Cannot update address for ADO that you did not create"),
        );
    }
}
