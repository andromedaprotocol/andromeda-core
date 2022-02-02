use crate::{
    reply::{on_token_creation_reply, REPLY_CREATE_TOKEN},
    state::{
        is_address_defined, is_creator, read_address, read_code_id, store_address, store_code_id,
    },
};
use andromeda_protocol::{
    communication::encode_binary,
    communication::{parse_message, AndromedaMsg, AndromedaQuery},
    error::ContractError,
    factory::{AddressResponse, CodeIdResponse, ExecuteMsg, InstantiateMsg, QueryMsg},
    modules::ModuleDefinition,
    operators::{execute_update_operators, query_is_operator, query_operators},
    ownership::{execute_update_owner, is_contract_owner, query_contract_owner, CONTRACT_OWNER},
    require,
    token::InstantiateMsg as TokenInstantiateMsg,
};
use cosmwasm_std::{
    attr, entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Reply, ReplyOn, Response, StdError,
    SubMsg, WasmMsg,
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    CONTRACT_OWNER.save(deps.storage, &info.sender)?;

    Ok(Response::default()
        .add_attributes(vec![attr("action", "instantiate"), attr("type", "factory")]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.result.is_err() {
        return Err(ContractError::Std(StdError::generic_err(
            msg.result.unwrap_err(),
        )));
    }

    match msg.id {
        REPLY_CREATE_TOKEN => on_token_creation_reply(deps, msg),
        _ => Err(ContractError::InvalidReplyId {}),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Create {
            symbol,
            name,
            modules,
        } => create(deps, env, info, name, symbol, modules),
        ExecuteMsg::UpdateAddress {
            symbol,
            new_address,
        } => update_address(deps, env, info, symbol, new_address),
        ExecuteMsg::UpdateCodeId {
            code_id_key,
            code_id,
        } => add_update_code_id(deps, env, info, code_id_key, code_id),
        ExecuteMsg::AndrReceive(msg) => execute_andr_receive(deps, env, info, msg),
    }
}

fn execute_andr_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: AndromedaMsg,
) -> Result<Response, ContractError> {
    match msg {
        AndromedaMsg::Receive(data) => {
            let received: ExecuteMsg = parse_message(data)?;
            match received {
                ExecuteMsg::AndrReceive(..) => Err(ContractError::NestedAndromedaMsg {}),
                _ => execute(deps, env, info, received),
            }
        }
        AndromedaMsg::UpdateOwner { address } => execute_update_owner(deps, info, address),
        AndromedaMsg::UpdateOperators { operators } => {
            execute_update_operators(deps, info, operators)
        }
        AndromedaMsg::Withdraw { .. } => Err(ContractError::UnsupportedOperation {}),
    }
}

pub fn create(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    name: String,
    symbol: String,
    modules: Vec<ModuleDefinition>,
) -> Result<Response, ContractError> {
    //let config = read_config(deps.storage)?;

    require(
        !is_address_defined(deps.storage, symbol.to_string())?,
        ContractError::SymbolInUse {},
    )?;

    //Assign Code IDs to Modules
    let updated_modules: Vec<ModuleDefinition> = modules
        .iter()
        .map(|m| match m {
            ModuleDefinition::Whitelist {
                address,
                operators,
                code_id: _,
            } => ModuleDefinition::Whitelist {
                address: address.clone(),
                operators: operators.clone(),
                code_id: Some(read_code_id(deps.storage, "address_list".to_string()).unwrap()),
            },
            ModuleDefinition::Blacklist {
                address,
                operators,
                code_id: _,
            } => ModuleDefinition::Blacklist {
                address: address.clone(),
                operators: operators.clone(),
                code_id: Some(read_code_id(deps.storage, "address_list".to_string()).unwrap()),
            },
            ModuleDefinition::Receipt {
                address,
                operators,
                code_id: _,
            } => ModuleDefinition::Receipt {
                address: address.clone(),
                operators: operators.clone(),
                code_id: Some(read_code_id(deps.storage, "receipt".to_string()).unwrap()),
            },
            _ => m.clone(),
        })
        .collect();

    let token_inst_msg = TokenInstantiateMsg {
        name: name.to_string(),
        symbol: symbol.to_string(),
        minter: info.sender.to_string(),
        modules: updated_modules,
    };
    // [TOK-01 Validation Process]
    let validation = token_inst_msg.validate();
    match validation {
        Ok(true) => {}
        Err(error) => panic!("{:?}", error),
        _ => {}
    };

    let inst_msg = WasmMsg::Instantiate {
        admin: Some(info.sender.to_string()),
        code_id: read_code_id(deps.storage, "token".to_string())?,
        funds: vec![],
        label: String::from("Address list instantiation"),
        msg: encode_binary(&token_inst_msg)?,
    };

    let msg = SubMsg {
        msg: inst_msg.into(),
        gas_limit: None,
        id: REPLY_CREATE_TOKEN,
        reply_on: ReplyOn::Always,
    };

    Ok(Response::new().add_submessage(msg).add_attributes(vec![
        attr("action", "create"),
        attr("name", name),
        attr("symbol", symbol),
        attr("owner", info.sender.to_string()),
    ]))
}

pub fn update_address(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    symbol: String,
    new_address: String,
) -> Result<Response, ContractError> {
    require(
        is_creator(&deps, symbol.clone(), info.sender.to_string())?
            || is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;

    store_address(deps.storage, symbol, &new_address)?;

    Ok(Response::default())
}

pub fn add_update_code_id(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    code_id_key: String,
    code_id: u64,
) -> Result<Response, ContractError> {
    require(
        is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;
    store_code_id(deps.storage, code_id_key.clone(), code_id)?;

    Ok(Response::default().add_attributes(vec![
        attr("action", "add_update_code_id"),
        attr("code_id_key", code_id_key),
        attr("code_id", code_id.to_string()),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::GetAddress { symbol } => encode_binary(&query_address(deps, symbol)?),
        QueryMsg::CodeId { key } => encode_binary(&query_code_id(deps, key)?),
        QueryMsg::AndrQuery(msg) => handle_andromeda_query(deps, msg),
    }
}

fn handle_andromeda_query(deps: Deps, msg: AndromedaQuery) -> Result<Binary, ContractError> {
    match msg {
        AndromedaQuery::Get(data) => {
            let code_id_key: String = parse_message(data)?;
            encode_binary(&query_code_id(deps, code_id_key)?)
        }
        AndromedaQuery::Owner {} => encode_binary(&query_contract_owner(deps)?),
        AndromedaQuery::Operators {} => encode_binary(&query_operators(deps)?),
        AndromedaQuery::IsOperator { address } => {
            encode_binary(&query_is_operator(deps, &address)?)
        }
    }
}

fn query_address(deps: Deps, symbol: String) -> Result<AddressResponse, ContractError> {
    let address = read_address(deps.storage, symbol)?;
    Ok(AddressResponse { address })
}

fn query_code_id(deps: Deps, key: String) -> Result<CodeIdResponse, ContractError> {
    let code_id = read_code_id(deps.storage, key)?;
    Ok(CodeIdResponse { code_id })
}

#[cfg(test)]
mod tests {
    use crate::state::{CODE_ID, SYM_ADDRESS};

    use super::*;
    use andromeda_protocol::testing::mock_querier::mock_dependencies_custom;
    use cosmwasm_std::{
        from_binary,
        testing::{mock_dependencies, mock_env, mock_info},
        Addr,
    };

    static TOKEN_CODE_ID: u64 = 0;
    const TOKEN_NAME: &str = "test";
    const TOKEN_SYMBOL: &str = "TT";

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {};
        let env = mock_env();

        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn test_create() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);

        let init_msg = InstantiateMsg {};

        let res = instantiate(deps.as_mut(), env, info.clone(), init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        let msg = ExecuteMsg::UpdateCodeId {
            code_id_key: "address_list".to_string(),
            code_id: 1u64,
        };
        let _ = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        let msg = ExecuteMsg::UpdateCodeId {
            code_id_key: "receipt".to_string(),
            code_id: 2u64,
        };
        let _ = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        let msg = ExecuteMsg::UpdateCodeId {
            code_id_key: "token".to_string(),
            code_id: 0u64,
        };
        let _ = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::Create {
            name: TOKEN_NAME.to_string(),
            symbol: TOKEN_SYMBOL.to_string(),
            modules: vec![],
        };

        let token_inst_msg = TokenInstantiateMsg {
            name: TOKEN_NAME.to_string(),
            symbol: TOKEN_SYMBOL.to_string(),
            minter: info.sender.to_string(),
            modules: vec![],
        };
        // [TOK-01 Validation Process]
        let validation = token_inst_msg.validate();
        match validation {
            Ok(true) => {}
            Err(error) => panic!("{:?}", error),
            _ => {}
        };

        let inst_msg = WasmMsg::Instantiate {
            admin: Some(info.sender.to_string()),
            code_id: TOKEN_CODE_ID,
            funds: vec![],
            label: String::from("Address list instantiation"),
            msg: encode_binary(&token_inst_msg).unwrap(),
        };

        let expected_msg = SubMsg {
            msg: inst_msg.into(),
            gas_limit: None,
            id: REPLY_CREATE_TOKEN,
            reply_on: ReplyOn::Always,
        };

        let expected_res = Response::new()
            .add_submessage(expected_msg)
            .add_attributes(vec![
                attr("action", "create"),
                attr("name", TOKEN_NAME.to_string()),
                attr("symbol", TOKEN_SYMBOL.to_string()),
                attr("owner", info.sender.to_string()),
            ]);

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res, expected_res);
        assert_eq!(1, expected_res.messages.len())
    }

    #[test]
    fn test_update_address() {
        let creator = String::from("creator");
        let owner = String::from("owner");
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info(creator.as_str(), &[]);

        CONTRACT_OWNER
            .save(deps.as_mut().storage, &Addr::unchecked(owner))
            .unwrap();
        SYM_ADDRESS
            .save(
                deps.as_mut().storage,
                TOKEN_SYMBOL.to_string(),
                &String::from("factory_address"),
            )
            .unwrap();

        let new_address = String::from("new");
        let update_msg = ExecuteMsg::UpdateAddress {
            symbol: TOKEN_SYMBOL.to_string(),
            new_address: new_address.clone(),
        };

        let unauth_env = mock_env();
        let unauth_info = mock_info("anyone", &[]);
        let unauth_res =
            execute(deps.as_mut(), unauth_env, unauth_info, update_msg.clone()).unwrap_err();

        assert_eq!(unauth_res, ContractError::Unauthorized {},);

        let update_res = execute(deps.as_mut(), env.clone(), info, update_msg).unwrap();

        assert_eq!(update_res, Response::default());

        let query_msg = QueryMsg::GetAddress {
            symbol: TOKEN_SYMBOL.to_string(),
        };

        let addr_res = query(deps.as_ref(), env, query_msg).unwrap();
        let addr_val: AddressResponse = from_binary(&addr_res).unwrap();

        assert_eq!(new_address, addr_val.address);
    }

    #[test]
    fn test_update_code_id() {
        let owner = String::from("owner");
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info(owner.as_str(), &[]);

        CONTRACT_OWNER
            .save(deps.as_mut().storage, &Addr::unchecked(owner))
            .unwrap();

        let msg = ExecuteMsg::UpdateCodeId {
            code_id_key: "address_list".to_string(),
            code_id: 1u64,
        };

        let resp = execute(deps.as_mut(), env, info, msg).unwrap();

        let expected = Response::new().add_attributes(vec![
            attr("action", "add_update_code_id"),
            attr("code_id_key", "address_list"),
            attr("code_id", "1"),
        ]);

        assert_eq!(resp, expected);
    }

    #[test]
    fn test_andr_get_query() {
        let mut deps = mock_dependencies_custom(&[]);

        CODE_ID
            .save(deps.as_mut().storage, "code_id".to_string(), &1u64)
            .unwrap();

        let msg = QueryMsg::AndrQuery(AndromedaQuery::Get(Some(
            encode_binary(&"code_id").unwrap(),
        )));

        let res: CodeIdResponse =
            from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();

        assert_eq!(CodeIdResponse { code_id: 1u64 }, res);
    }
}
