use andromeda_protocol::{
    factory::{AddressResponse, CodeIdsResponse, ExecuteMsg, InstantiateMsg, QueryMsg},
    modules::ModuleDefinition,
    ownership::{execute_update_owner, is_contract_owner, query_contract_owner, CONTRACT_OWNER},
    require,
    token::InstantiateMsg as TokenInstantiateMsg,
};
use cosmwasm_std::{
    attr, entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, ReplyOn,
    Response, StdError, StdResult, SubMsg, WasmMsg,
};

use crate::{
    reply::{on_token_creation_reply, REPLY_CREATE_TOKEN},
    state::{
        is_address_defined, is_creator, read_address, read_config, store_address, store_config,
        Config,
    },
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    store_config(
        deps.storage,
        &Config {
            token_code_id: msg.token_code_id,
            receipt_code_id: msg.receipt_code_id,
            address_list_code_id: msg.address_list_code_id,
        },
    )?;

    CONTRACT_OWNER.save(deps.storage, &info.sender.to_string())?;

    Ok(Response::default()
        .add_attributes(vec![attr("action", "instantiate"), attr("type", "factory")]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> StdResult<Response> {
    if msg.result.is_err() {
        return Err(StdError::generic_err(msg.result.unwrap_err()));
    }

    match msg.id {
        REPLY_CREATE_TOKEN => on_token_creation_reply(deps, msg),
        _ => Err(StdError::generic_err("reply id is invalid")),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
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
        ExecuteMsg::UpdateOwner { address } => execute_update_owner(deps, info, address),
        ExecuteMsg::UpdateCodeId {
            address_list_code_id,
            receipt_code_id,
            token_code_id,
        } => update_code_id(
            deps,
            env,
            info,
            receipt_code_id,
            address_list_code_id,
            token_code_id,
        ),
    }
}

pub fn create(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    name: String,
    symbol: String,
    modules: Vec<ModuleDefinition>,
) -> StdResult<Response> {
    let config = read_config(deps.storage)?;

    require(
        !is_address_defined(deps.storage, symbol.to_string())?,
        StdError::generic_err("Symbol is in use"),
    )?;

    //Assign Code IDs to Modules
    let updated_modules: Vec<ModuleDefinition> = modules
        .iter()
        .map(|m| match m {
            ModuleDefinition::Whitelist {
                address,
                moderators,
                code_id: _,
            } => ModuleDefinition::Whitelist {
                address: address.clone(),
                moderators: moderators.clone(),
                code_id: Some(config.address_list_code_id),
            },
            ModuleDefinition::Blacklist {
                address,
                moderators,
                code_id: _,
            } => ModuleDefinition::Blacklist {
                address: address.clone(),
                moderators: moderators.clone(),
                code_id: Some(config.address_list_code_id),
            },
            ModuleDefinition::Receipt {
                address,
                moderators,
                code_id: _,
            } => ModuleDefinition::Receipt {
                address: address.clone(),
                moderators: moderators.clone(),
                code_id: Some(config.receipt_code_id),
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
        code_id: config.token_code_id,
        funds: vec![],
        label: String::from("Address list instantiation"),
        msg: to_binary(&token_inst_msg)?,
    };

    let msg = SubMsg {
        msg: inst_msg.into(),
        gas_limit: None,
        id: REPLY_CREATE_TOKEN,
        reply_on: ReplyOn::Always,
    };

    Ok(Response::new().add_submessage(msg).add_attributes(vec![
        attr("action", "create"),
        attr("name", name.clone()),
        attr("symbol", symbol.clone()),
    ]))
}

pub fn update_address(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    symbol: String,
    new_address: String,
) -> StdResult<Response> {
    require(
        is_creator(&deps, symbol.clone(), info.sender.to_string())?
            || is_contract_owner(deps.storage, info.sender.to_string())?,
        StdError::generic_err("Cannot update address for ADO that you did not create"),
    )?;

    store_address(deps.storage, symbol, &new_address)?;

    Ok(Response::default())
}

pub fn update_code_id(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    receipt_code_id: Option<u64>,
    address_list_code_id: Option<u64>,
    token_code_id: Option<u64>,
) -> StdResult<Response> {
    require(receipt_code_id.is_some() || address_list_code_id.is_some() || token_code_id.is_some(), StdError::generic_err("Must provide one of the following: \"receipt_code_id\", \"token_code_id\", \"address_list_code_id\""))?;
    require(
        is_contract_owner(deps.storage, info.sender.to_string())?,
        StdError::generic_err("Can only be used by the contract owner"),
    )?;
    let mut config = read_config(deps.storage)?;

    // [GLOBAL-02] Changing is_some() + .unwrap() to if let Some()
    if let Some(receipt_code_id) = receipt_code_id {
        config.receipt_code_id = receipt_code_id;
    }

    // [GLOBAL-02] Changing is_some() + .unwrap() to if let Some()
    if let Some(address_list_code_id) = address_list_code_id {
        config.address_list_code_id = address_list_code_id;
    }

    // [GLOBAL-02] Changing is_some() + .unwrap() to if let Some()
    if let Some(token_code_id) = token_code_id {
        config.token_code_id = token_code_id;
    }

    store_config(deps.storage, &config)?;

    Ok(Response::default().add_attributes(vec![
        attr("action", "update_code_id"),
        attr("receipt_code_id", config.receipt_code_id.to_string()),
        attr("token_code_id", config.token_code_id.to_string()),
        attr(
            "address_list_code_id",
            config.address_list_code_id.to_string(),
        ),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetAddress { symbol } => to_binary(&query_address(deps, symbol)?),
        QueryMsg::ContractOwner {} => to_binary(&query_contract_owner(deps)?),
        QueryMsg::CodeIds {} => to_binary(&query_code_ids(deps)?),
    }
}

fn query_address(deps: Deps, symbol: String) -> StdResult<AddressResponse> {
    let address = read_address(deps.storage, symbol)?;
    Ok(AddressResponse { address })
}

fn query_code_ids(deps: Deps) -> StdResult<CodeIdsResponse> {
    let config = read_config(deps.storage)?;

    Ok(CodeIdsResponse {
        receipt_code_id: config.receipt_code_id,
        address_list_code_id: config.address_list_code_id,
        token_code_id: config.token_code_id,
    })
}

#[cfg(test)]
mod tests {
    use crate::state::SYM_ADDRESS;

    use super::*;
    use andromeda_protocol::testing::mock_querier::mock_dependencies_custom;
    use cosmwasm_std::{
        from_binary,
        testing::{mock_dependencies, mock_env, mock_info},
    };

    static TOKEN_CODE_ID: u64 = 0;
    static RECEIPT_CODE_ID: u64 = 1;

    static ADDRESS_LIST_CODE_ID: u64 = 2;
    const TOKEN_NAME: &str = "test";
    const TOKEN_SYMBOL: &str = "TT";

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            token_code_id: TOKEN_CODE_ID,
            receipt_code_id: RECEIPT_CODE_ID,
            address_list_code_id: ADDRESS_LIST_CODE_ID,
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
            address_list_code_id: ADDRESS_LIST_CODE_ID,
        };

        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), init_msg).unwrap();
        assert_eq!(0, res.messages.len());

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
            msg: to_binary(&token_inst_msg).unwrap(),
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
            ]);

        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
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

        CONTRACT_OWNER.save(deps.as_mut().storage, &owner).unwrap();
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

        assert_eq!(
            unauth_res,
            StdError::generic_err("Cannot update address for ADO that you did not create"),
        );

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
        let info = mock_info(owner.clone().as_str(), &[]);
        let unauth_info = mock_info("anyone", &[]);
        let config = Config {
            address_list_code_id: ADDRESS_LIST_CODE_ID,
            receipt_code_id: RECEIPT_CODE_ID,
            token_code_id: TOKEN_CODE_ID,
        };
        store_config(deps.as_mut().storage, &config).unwrap();

        CONTRACT_OWNER.save(deps.as_mut().storage, &owner).unwrap();

        let invalid_msg = ExecuteMsg::UpdateCodeId {
            receipt_code_id: None,
            token_code_id: None,
            address_list_code_id: None,
        };

        let resp = execute(deps.as_mut(), env.clone(), info.clone(), invalid_msg).unwrap_err();
        let expected = StdError::generic_err("Must provide one of the following: \"receipt_code_id\", \"token_code_id\", \"address_list_code_id\"");

        assert_eq!(resp, expected);

        let new_receipt_code_id = 4;
        let msg = ExecuteMsg::UpdateCodeId {
            receipt_code_id: Some(new_receipt_code_id),
            token_code_id: None,
            address_list_code_id: None,
        };

        let resp = execute(deps.as_mut(), env.clone(), unauth_info, msg.clone()).unwrap_err();
        let expected = StdError::generic_err("Can only be used by the contract owner");

        assert_eq!(resp, expected);

        let resp = execute(deps.as_mut(), env, info, msg).unwrap();
        let expected = Response::default().add_attributes(vec![
            attr("action", "update_code_id"),
            attr("receipt_code_id", new_receipt_code_id.to_string()),
            attr("token_code_id", TOKEN_CODE_ID.to_string()),
            attr("address_list_code_id", ADDRESS_LIST_CODE_ID.to_string()),
        ]);

        assert_eq!(resp, expected);

        let new_config = read_config(deps.as_ref().storage).unwrap();
        let expected = Config {
            receipt_code_id: new_receipt_code_id,
            address_list_code_id: ADDRESS_LIST_CODE_ID,
            token_code_id: TOKEN_CODE_ID,
        };

        assert_eq!(new_config, expected);
    }
}
