use andromeda_protocol::{
    factory::{AddressResponse, ExecuteMsg, InstantiateMsg, QueryMsg},
    modules::ModuleDefinition,
    ownership::{execute_update_owner, is_contract_owner, query_contract_owner, CONTRACT_OWNER},
    require::require,
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
            owner: info.sender.to_string(),
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
            metadata_limit,
        } => create(deps, env, info, name, symbol, modules, metadata_limit),
        ExecuteMsg::UpdateAddress {
            symbol,
            new_address,
        } => update_address(deps, env, info, symbol, new_address),
        ExecuteMsg::UpdateOwner { address } => execute_update_owner(deps, info, address),
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
                code_id: Some(config.address_list_code_id.clone()),
            },
            ModuleDefinition::Blacklist {
                address,
                moderators,
                code_id: _,
            } => ModuleDefinition::Blacklist {
                address: address.clone(),
                moderators: moderators.clone(),
                code_id: Some(config.address_list_code_id.clone()),
            },
            ModuleDefinition::Receipt {
                address,
                moderators,
                code_id: _,
            } => ModuleDefinition::Receipt {
                address: address.clone(),
                moderators: moderators.clone(),
                code_id: Some(config.receipt_code_id.clone()),
            },
            _ => m.clone(),
        })
        .collect();

    let token_inst_msg = TokenInstantiateMsg {
        name: name.to_string(),
        symbol: symbol.to_string(),
        minter: info.sender.to_string(),
        modules: updated_modules,
        metadata_limit,
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

    store_address(deps.storage, symbol.clone(), &new_address.clone())?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetAddress { symbol } => to_binary(&query_address(deps, symbol)?),
        QueryMsg::ContractOwner {} => to_binary(&query_contract_owner(deps)?),
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
    const TOKEN_SYMBOL: &str = "T";

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
            metadata_limit: None,
        };

        let token_inst_msg = TokenInstantiateMsg {
            name: TOKEN_NAME.to_string(),
            symbol: TOKEN_SYMBOL.to_string(),
            minter: info.sender.to_string(),
            modules: vec![],
            metadata_limit: None,
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
        let info = mock_info(creator.clone().as_str(), &[]);

        CONTRACT_OWNER
            .save(deps.as_mut().storage, &owner.clone())
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

        let update_res =
            execute(deps.as_mut(), env.clone(), info.clone(), update_msg.clone()).unwrap();

        assert_eq!(update_res, Response::default());

        let query_msg = QueryMsg::GetAddress {
            symbol: TOKEN_SYMBOL.to_string(),
        };

        let addr_res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
        let addr_val: AddressResponse = from_binary(&addr_res).unwrap();

        assert_eq!(new_address.clone(), addr_val.address);
    }
}
