use andromeda_protocol::{
    address_list::{
        add_address, includes_address, remove_address, ExecuteMsg, IncludesAddressResponse,
        InstantiateMsg, QueryMsg, IS_INCLUSIVE,
    },
    communication::{encode_binary, hooks::AndromedaHook},
    error::ContractError,
    operators::{execute_update_operators, initialize_operators, is_operator, query_is_operator},
    ownership::{execute_update_owner, query_contract_owner, CONTRACT_OWNER},
    require,
};
use cosmwasm_std::{attr, entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Response};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    initialize_operators(deps.storage, msg.operators)?;
    IS_INCLUSIVE.save(deps.storage, &msg.is_inclusive)?;
    CONTRACT_OWNER.save(deps.storage, &info.sender)?;
    Ok(Response::default().add_attributes(vec![
        attr("action", "instantiate"),
        attr("type", "address_list"),
    ]))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AddAddress { address } => execute_add_address(deps, info, address),
        ExecuteMsg::RemoveAddress { address } => execute_remove_address(deps, info, address),
        ExecuteMsg::UpdateOwner { address } => execute_update_owner(deps, info, address),
        ExecuteMsg::UpdateOperator { operators } => execute_update_operators(deps, info, operators),
    }
}

fn execute_add_address(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    require(
        is_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;
    add_address(deps.storage, &address)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "add_address"),
        attr("address", address),
    ]))
}

fn execute_remove_address(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    require(
        is_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;

    remove_address(deps.storage, &address);

    Ok(Response::new().add_attributes(vec![
        attr("action", "remove_address"),
        attr("address", address),
    ]))
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::IncludesAddress { address } => encode_binary(&query_address(deps, &address)?),
        QueryMsg::ContractOwner {} => encode_binary(&query_contract_owner(deps)?),
        QueryMsg::IsOperator { address } => encode_binary(&query_is_operator(deps, &address)?),
        QueryMsg::AndrHook(msg) => encode_binary(&handle_andr_hook(deps, msg)?),
    }
}

fn handle_andr_hook(deps: Deps, msg: AndromedaHook) -> Result<Response, ContractError> {
    match msg {
        AndromedaHook::OnExecute { sender, .. } => {
            let included = includes_address(deps.storage, &sender)?;
            let is_inclusive = IS_INCLUSIVE.load(deps.storage)?;
            if (is_inclusive && !included) || (!is_inclusive && included) {
                Err(ContractError::InvalidAddress {})
            } else {
                Ok(Response::default())
            }
        }
        _ => Err(ContractError::UnsupportedOperation {}),
    }
}

fn query_address(deps: Deps, address: &str) -> Result<IncludesAddressResponse, ContractError> {
    Ok(IncludesAddressResponse {
        included: includes_address(deps.storage, address)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use andromeda_protocol::address_list::ADDRESS_LIST;
    use andromeda_protocol::operators::OPERATORS;
    use cosmwasm_std::from_binary;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            operators: vec!["11".to_string(), "22".to_string()],
            is_inclusive: true,
        };
        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn test_add_address() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();

        let operator = "creator";
        let info = mock_info(operator, &[]);

        let address = "whitelistee";

        //input operator for test

        OPERATORS
            .save(deps.as_mut().storage, operator, &true)
            .unwrap();
        CONTRACT_OWNER
            .save(deps.as_mut().storage, &info.sender)
            .unwrap();

        let msg = ExecuteMsg::AddAddress {
            address: address.to_string(),
        };

        //add address for registered operator

        let res = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap();
        let expected = Response::default().add_attributes(vec![
            attr("action", "add_address"),
            attr("address", address),
        ]);
        assert_eq!(expected, res);

        let whitelisted = ADDRESS_LIST
            .load(deps.as_ref().storage, address.to_string())
            .unwrap();
        assert!(whitelisted);

        let included = ADDRESS_LIST
            .load(deps.as_ref().storage, "111".to_string())
            .unwrap_err();

        match included {
            cosmwasm_std::StdError::NotFound { .. } => {}
            _ => {
                panic!();
            }
        }

        //add address for unregistered operator
        let unauth_info = mock_info("anyone", &[]);
        let res = execute(deps.as_mut(), env, unauth_info, msg).unwrap_err();
        assert_eq!(ContractError::Unauthorized {}, res);
    }

    #[test]
    fn test_remove_address() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();

        let operator = "creator";
        let info = mock_info(operator, &[]);

        let address = "whitelistee";

        //save operator
        OPERATORS
            .save(deps.as_mut().storage, operator, &true)
            .unwrap();
        CONTRACT_OWNER
            .save(deps.as_mut().storage, &info.sender)
            .unwrap();

        let msg = ExecuteMsg::RemoveAddress {
            address: address.to_string(),
        };

        //add address for registered operator
        let res = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap();
        let expected = Response::default().add_attributes(vec![
            attr("action", "remove_address"),
            attr("address", address.to_string()),
        ]);
        assert_eq!(expected, res);

        let included_is_err = ADDRESS_LIST
            .load(deps.as_ref().storage, address.to_string())
            .is_err();
        assert!(included_is_err);

        //add address for unregistered operator
        let unauth_info = mock_info("anyone", &[]);
        let res = execute(deps.as_mut(), env, unauth_info, msg).unwrap_err();
        assert_eq!(ContractError::Unauthorized {}, res);
    }

    #[test]
    fn test_execute_hook_whitelist() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();

        let operator = "creator";
        let info = mock_info(operator, &[]);

        let address = "whitelistee";

        // Mark it as a whitelist.
        IS_INCLUSIVE.save(deps.as_mut().storage, &true).unwrap();
        OPERATORS
            .save(deps.as_mut().storage, operator, &true)
            .unwrap();
        CONTRACT_OWNER
            .save(deps.as_mut().storage, &info.sender)
            .unwrap();

        let msg = ExecuteMsg::AddAddress {
            address: address.to_string(),
        };
        let _res = execute(deps.as_mut(), env, info, msg).unwrap();

        let msg = QueryMsg::AndrHook(AndromedaHook::OnExecute {
            sender: address.to_string(),
            msg: encode_binary(&"".to_string()).unwrap(),
        });

        let res: Response = from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
        assert_eq!(Response::default(), res);

        let msg = QueryMsg::AndrHook(AndromedaHook::OnExecute {
            sender: "random".to_string(),
            msg: encode_binary(&"".to_string()).unwrap(),
        });

        let res_err: ContractError = query(deps.as_ref(), mock_env(), msg).unwrap_err();
        assert_eq!(ContractError::InvalidAddress {}, res_err);
    }

    #[test]
    fn test_execute_hook_blacklist() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();

        let operator = "creator";
        let info = mock_info(operator, &[]);

        let address = "blacklistee";

        // Mark it as a blacklist.
        IS_INCLUSIVE.save(deps.as_mut().storage, &false).unwrap();
        OPERATORS
            .save(deps.as_mut().storage, operator, &true)
            .unwrap();
        CONTRACT_OWNER
            .save(deps.as_mut().storage, &info.sender)
            .unwrap();

        let msg = ExecuteMsg::AddAddress {
            address: address.to_string(),
        };
        let _res = execute(deps.as_mut(), env, info, msg).unwrap();

        let msg = QueryMsg::AndrHook(AndromedaHook::OnExecute {
            sender: "random".to_string(),
            msg: encode_binary(&"".to_string()).unwrap(),
        });

        let res: Response = from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
        assert_eq!(Response::default(), res);

        let msg = QueryMsg::AndrHook(AndromedaHook::OnExecute {
            sender: address.to_string(),
            msg: encode_binary(&"".to_string()).unwrap(),
        });

        let res_err: ContractError = query(deps.as_ref(), mock_env(), msg).unwrap_err();
        assert_eq!(ContractError::InvalidAddress {}, res_err);
    }
}
