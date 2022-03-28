use ado_base::state::ADOContract;
use andromeda_protocol::address_list::{
    add_address, includes_address, remove_address, ExecuteMsg, IncludesAddressResponse,
    InstantiateMsg, MigrateMsg, QueryMsg, IS_INCLUSIVE,
};
use common::{
    ado_base::{hooks::AndromedaHook, AndromedaQuery, InstantiateMsg as BaseInstantiateMsg},
    encode_binary,
    error::ContractError,
    parse_message, require,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{attr, Binary, Deps, DepsMut, Env, MessageInfo, Response};

use cw2::{get_contract_version, set_contract_version};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-addresslist";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    IS_INCLUSIVE.save(deps.storage, &msg.is_inclusive)?;
    ADOContract::default().instantiate(
        deps.storage,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "address_list".to_string(),
            operators: Some(msg.operators),
            modules: None,
            primitive_contract: None,
        },
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AndrReceive(msg) => {
            ADOContract::default().execute(deps, env, info, msg, execute)
        }
        ExecuteMsg::AddAddress { address } => execute_add_address(deps, info, address),
        ExecuteMsg::RemoveAddress { address } => execute_remove_address(deps, info, address),
    }
}

fn execute_add_address(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    require(
        ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?,
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
        ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;

    remove_address(deps.storage, &address);

    Ok(Response::new().add_attributes(vec![
        attr("action", "remove_address"),
        attr("address", address),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let version = get_contract_version(deps.storage)?;
    if version.contract != CONTRACT_NAME {
        return Err(ContractError::CannotMigrate {
            previous_contract: version.contract,
        });
    }
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::IncludesAddress { address } => encode_binary(&query_address(deps, &address)?),
        QueryMsg::AndrHook(msg) => encode_binary(&handle_andr_hook(deps, msg)?),
        QueryMsg::AndrQuery(msg) => handle_andromeda_query(deps, env, msg),
    }
}

fn handle_andr_hook(deps: Deps, msg: AndromedaHook) -> Result<Response, ContractError> {
    match msg {
        AndromedaHook::OnExecute { sender, .. } => {
            let is_included = includes_address(deps.storage, &sender)?;
            let is_inclusive = IS_INCLUSIVE.load(deps.storage)?;
            if is_included != is_inclusive {
                Err(ContractError::InvalidAddress {})
            } else {
                Ok(Response::default())
            }
        }
        _ => Err(ContractError::UnsupportedOperation {}),
    }
}

fn handle_andromeda_query(
    deps: Deps,
    env: Env,
    msg: AndromedaQuery,
) -> Result<Binary, ContractError> {
    match msg {
        AndromedaQuery::Get(data) => {
            let address: String = parse_message(&data)?;
            encode_binary(&query_address(deps, &address)?)
        }
        _ => ADOContract::default().query(deps, env, msg, query),
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
    use cosmwasm_std::from_binary;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    fn init(deps: DepsMut, info: MessageInfo) {
        instantiate(
            deps,
            mock_env(),
            info,
            InstantiateMsg {
                operators: vec![],
                is_inclusive: true,
            },
        )
        .unwrap();
    }

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

        init(deps.as_mut(), info.clone());

        ADOContract::default()
            .execute_update_operators(deps.as_mut(), info.clone(), vec![operator.to_owned()])
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

        init(deps.as_mut(), info.clone());

        //save operator
        ADOContract::default()
            .execute_update_operators(deps.as_mut(), info.clone(), vec![operator.to_owned()])
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
        init(deps.as_mut(), info.clone());

        let msg = ExecuteMsg::AddAddress {
            address: address.to_string(),
        };
        let _res = execute(deps.as_mut(), env, info, msg).unwrap();

        let msg = QueryMsg::AndrHook(AndromedaHook::OnExecute {
            sender: address.to_string(),
            payload: encode_binary(&"".to_string()).unwrap(),
        });

        let res: Response = from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
        assert_eq!(Response::default(), res);

        let msg = QueryMsg::AndrHook(AndromedaHook::OnExecute {
            sender: "random".to_string(),
            payload: encode_binary(&"".to_string()).unwrap(),
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
        init(deps.as_mut(), info.clone());

        // Mark it as a blacklist.
        IS_INCLUSIVE.save(deps.as_mut().storage, &false).unwrap();
        ADOContract::default()
            .execute_update_operators(deps.as_mut(), info.clone(), vec![operator.to_owned()])
            .unwrap();

        let msg = ExecuteMsg::AddAddress {
            address: address.to_string(),
        };
        let _res = execute(deps.as_mut(), env, info, msg).unwrap();

        let msg = QueryMsg::AndrHook(AndromedaHook::OnExecute {
            sender: "random".to_string(),
            payload: encode_binary(&"".to_string()).unwrap(),
        });

        let res: Response = from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
        assert_eq!(Response::default(), res);

        let msg = QueryMsg::AndrHook(AndromedaHook::OnExecute {
            sender: address.to_string(),
            payload: encode_binary(&"".to_string()).unwrap(),
        });

        let res_err: ContractError = query(deps.as_ref(), mock_env(), msg).unwrap_err();
        assert_eq!(ContractError::InvalidAddress {}, res_err);
    }

    #[test]
    fn test_andr_get_query() {
        let mut deps = mock_dependencies(&[]);

        let address = "whitelistee";

        ADDRESS_LIST
            .save(deps.as_mut().storage, address.to_string(), &true)
            .unwrap();

        let msg = QueryMsg::AndrQuery(AndromedaQuery::Get(Some(encode_binary(&address).unwrap())));

        let res: IncludesAddressResponse =
            from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();

        assert_eq!(IncludesAddressResponse { included: true }, res);
    }
}
