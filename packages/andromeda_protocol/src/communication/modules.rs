/*use std::convert::TryInto;

use cosmwasm_std::{
    to_binary, Addr, Api, Binary, CosmosMsg, DepsMut, Event, MessageInfo, Order, QuerierWrapper,
    QueryRequest, ReplyOn, Response, StdError, Storage, SubMsg, Uint64, WasmMsg, WasmQuery,
};
use cw_storage_plus::{Bound, Item, Map};
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    communication::{query_get, HookMsg},
    error::ContractError,
    factory::CodeIdResponse,
    primitive::{get_address, AndromedaContract},
    rates::Funds,
    require,
};

use super::hooks::{AndromedaHook, OnFundsTransferResponse};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ownership::CONTRACT_OWNER;
    use cosmwasm_std::testing::{mock_dependencies, mock_info};

    #[test]
    fn test_validate_addresslist() {
        let addresslist_module = Module {
            module_type: ModuleType::AddressList,
            instantiate: InstantiateType::Address("".to_string()),
            is_mutable: false,
        };

        let res = addresslist_module.validate(
            &[addresslist_module.clone(), addresslist_module.clone()],
            &ADOType::CW721,
        );
        assert_eq!(ContractError::ModuleNotUnique {}, res.unwrap_err());

        let auction_module = Module {
            module_type: ModuleType::Auction,
            instantiate: InstantiateType::Address("".into()),
            is_mutable: false,
        };
        addresslist_module
            .validate(
                &[addresslist_module.clone(), auction_module],
                &ADOType::CW721,
            )
            .unwrap();
    }

    #[test]
    fn test_validate_auction() {
        let module = Module {
            module_type: ModuleType::Auction,
            instantiate: InstantiateType::Address("".to_string()),
            is_mutable: false,
        };

        let res = module.validate(&[module.clone(), module.clone()], &ADOType::CW721);
        assert_eq!(ContractError::ModuleNotUnique {}, res.unwrap_err());

        let res = module.validate(&[module.clone()], &ADOType::CW20);
        assert_eq!(
            ContractError::IncompatibleModules {
                msg: "An Auction module cannot be used for a CW20 ADO".to_string()
            },
            res.unwrap_err()
        );

        let other_module = Module {
            module_type: ModuleType::Rates,
            instantiate: InstantiateType::Address("".to_string()),
            is_mutable: false,
        };
        module
            .validate(&[module.clone(), other_module], &ADOType::CW721)
            .unwrap();
    }

    #[test]
    fn test_validate_rates() {
        let module = Module {
            module_type: ModuleType::Rates,
            instantiate: InstantiateType::Address("".to_string()),
            is_mutable: false,
        };

        let res = module.validate(&[module.clone(), module.clone()], &ADOType::CW721);
        assert_eq!(ContractError::ModuleNotUnique {}, res.unwrap_err());

        let other_module = Module {
            module_type: ModuleType::AddressList,
            instantiate: InstantiateType::Address("".to_string()),
            is_mutable: false,
        };
        module
            .validate(&[module.clone(), other_module], &ADOType::CW721)
            .unwrap();
    }

    #[test]
    fn test_validate_receipt() {
        let module = Module {
            module_type: ModuleType::Receipt,
            instantiate: InstantiateType::Address("".to_string()),
            is_mutable: false,
        };

        let res = module.validate(&[module.clone(), module.clone()], &ADOType::CW721);
        assert_eq!(ContractError::ModuleNotUnique {}, res.unwrap_err());

        let other_module = Module {
            module_type: ModuleType::AddressList,
            instantiate: InstantiateType::Address("".to_string()),
            is_mutable: false,
        };
        module
            .validate(&[module.clone(), other_module], &ADOType::CW721)
            .unwrap();
    }

    #[test]
    fn test_validate_uniqueness() {
        let module1 = Module {
            module_type: ModuleType::Receipt,
            instantiate: InstantiateType::Address("addr1".to_string()),
            is_mutable: false,
        };

        let module2 = Module {
            module_type: ModuleType::Receipt,
            instantiate: InstantiateType::Address("addr2".to_string()),
            is_mutable: false,
        };

        let res = module1.validate(&[module1.clone(), module2], &ADOType::CW721);
        assert_eq!(ContractError::ModuleNotUnique {}, res.unwrap_err());
    }

    #[test]
    fn test_execute_register_module_unauthorized() {
        let mut deps = mock_dependencies(&[]);

        let module = Module {
            module_type: ModuleType::AddressList,
            instantiate: InstantiateType::Address("address".to_string()),
            is_mutable: false,
        };
        let deps_mut = deps.as_mut();
        CONTRACT_OWNER
            .save(deps_mut.storage, &Addr::unchecked("owner"))
            .unwrap();

        let res = execute_register_module(
            &deps_mut.querier,
            deps_mut.storage,
            deps_mut.api,
            "sender",
            &module,
            ADOType::CW20,
            true,
        );

        assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
    }

    #[test]
    fn test_execute_register_module_addr() {
        let mut deps = mock_dependencies(&[]);

        let module = Module {
            module_type: ModuleType::AddressList,
            instantiate: InstantiateType::Address("address".to_string()),
            is_mutable: false,
        };
        let deps_mut = deps.as_mut();
        CONTRACT_OWNER
            .save(deps_mut.storage, &Addr::unchecked("owner"))
            .unwrap();

        let res = execute_register_module(
            &deps_mut.querier,
            deps_mut.storage,
            deps_mut.api,
            "owner",
            &module,
            ADOType::CW20,
            true,
        )
        .unwrap();

        assert_eq!(
            Response::default().add_attribute("action", "register_module"),
            res
        );

        assert_eq!(
            module,
            MODULE_INFO.load(deps.as_mut().storage, "1").unwrap()
        );

        assert_eq!(
            "address".to_string(),
            MODULE_ADDR.load(deps.as_mut().storage, "1").unwrap()
        );
    }

    #[test]
    fn test_execute_register_module_validate() {
        let mut deps = mock_dependencies(&[]);

        let module = Module {
            module_type: ModuleType::Auction,
            instantiate: InstantiateType::Address("address".to_string()),
            is_mutable: false,
        };
        let deps_mut = deps.as_mut();
        CONTRACT_OWNER
            .save(deps_mut.storage, &Addr::unchecked("owner"))
            .unwrap();

        let res = execute_register_module(
            &deps_mut.querier,
            deps_mut.storage,
            deps_mut.api,
            "owner",
            &module,
            ADOType::CW20,
            true,
        );

        assert_eq!(
            ContractError::IncompatibleModules {
                msg: "An Auction module cannot be used for a CW20 ADO".to_string()
            },
            res.unwrap_err(),
        );

        let res = execute_register_module(
            &deps_mut.querier,
            deps_mut.storage,
            deps_mut.api,
            "owner",
            &module,
            ADOType::CW20,
            false,
        )
        .unwrap();

        assert_eq!(
            Response::default().add_attribute("action", "register_module"),
            res
        );
    }

    #[test]
    fn test_execute_alter_module_unauthorized() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("sender", &[]);
        let module = Module {
            module_type: ModuleType::AddressList,
            instantiate: InstantiateType::Address("address".to_string()),
            is_mutable: true,
        };
        CONTRACT_OWNER
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();

        let res = execute_alter_module(deps.as_mut(), info, 1u64.into(), &module, ADOType::CW20);

        assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
    }

    #[test]
    fn test_execute_alter_module_addr() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("owner", &[]);
        let module = Module {
            module_type: ModuleType::AddressList,
            instantiate: InstantiateType::Address("address".to_string()),
            is_mutable: true,
        };

        CONTRACT_OWNER
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();

        MODULE_INFO
            .save(deps.as_mut().storage, "1", &module)
            .unwrap();
        MODULE_ADDR
            .save(deps.as_mut().storage, "1", &Addr::unchecked("address"))
            .unwrap();

        let module = Module {
            module_type: ModuleType::Receipt,
            instantiate: InstantiateType::Address("other_address".to_string()),
            is_mutable: true,
        };

        let res =
            execute_alter_module(deps.as_mut(), info, 1u64.into(), &module, ADOType::CW20).unwrap();

        assert_eq!(
            Response::default()
                .add_attribute("action", "alter_module")
                .add_attribute("module_idx", "1"),
            res
        );

        assert_eq!(
            module,
            MODULE_INFO.load(deps.as_mut().storage, "1").unwrap()
        );

        assert_eq!(
            "other_address".to_string(),
            MODULE_ADDR.load(deps.as_mut().storage, "1").unwrap()
        );
    }

    #[test]
    fn test_execute_alter_module_immutable() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("owner", &[]);
        let module = Module {
            module_type: ModuleType::AddressList,
            instantiate: InstantiateType::Address("address".to_string()),
            is_mutable: false,
        };

        CONTRACT_OWNER
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();

        MODULE_INFO
            .save(deps.as_mut().storage, "1", &module)
            .unwrap();
        MODULE_ADDR
            .save(deps.as_mut().storage, "1", &Addr::unchecked("address"))
            .unwrap();

        let module = Module {
            module_type: ModuleType::Receipt,
            instantiate: InstantiateType::Address("other_address".to_string()),
            is_mutable: true,
        };

        let res = execute_alter_module(deps.as_mut(), info, 1u64.into(), &module, ADOType::CW20);

        assert_eq!(ContractError::ModuleImmutable {}, res.unwrap_err());
    }

    #[test]
    fn test_execute_alter_module_nonexisting_module() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("owner", &[]);
        let module = Module {
            module_type: ModuleType::Auction,
            instantiate: InstantiateType::Address("address".to_string()),
            is_mutable: true,
        };

        CONTRACT_OWNER
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();

        let res = execute_alter_module(deps.as_mut(), info, 1u64.into(), &module, ADOType::CW20);

        assert_eq!(ContractError::ModuleDoesNotExist {}, res.unwrap_err());
    }

    #[test]
    fn test_execute_alter_module_incompatible_module() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("owner", &[]);
        let module = Module {
            module_type: ModuleType::Auction,
            instantiate: InstantiateType::Address("address".to_string()),
            is_mutable: true,
        };

        CONTRACT_OWNER
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();

        MODULE_INFO
            .save(deps.as_mut().storage, "1", &module)
            .unwrap();
        MODULE_ADDR
            .save(deps.as_mut().storage, "1", &Addr::unchecked("address"))
            .unwrap();

        let res = execute_alter_module(deps.as_mut(), info, 1u64.into(), &module, ADOType::CW20);

        assert_eq!(
            ContractError::IncompatibleModules {
                msg: "An Auction module cannot be used for a CW20 ADO".to_string()
            },
            res.unwrap_err(),
        );
    }

    #[test]
    fn test_execute_deregister_module_unauthorized() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("sender", &[]);
        CONTRACT_OWNER
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();

        let res = execute_deregister_module(deps.as_mut(), info, 1u64.into());

        assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
    }

    #[test]
    fn test_execute_deregister_module() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("owner", &[]);
        CONTRACT_OWNER
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();

        let module = Module {
            module_type: ModuleType::AddressList,
            instantiate: InstantiateType::Address("address".to_string()),
            is_mutable: true,
        };

        MODULE_INFO
            .save(deps.as_mut().storage, "1", &module)
            .unwrap();

        MODULE_ADDR
            .save(deps.as_mut().storage, "1", &Addr::unchecked("address"))
            .unwrap();

        let res = execute_deregister_module(deps.as_mut(), info, 1u64.into()).unwrap();

        assert_eq!(
            Response::default()
                .add_attribute("action", "deregister_module")
                .add_attribute("module_idx", "1"),
            res
        );

        assert!(!MODULE_ADDR.has(deps.as_mut().storage, "1"));
        assert!(!MODULE_INFO.has(deps.as_mut().storage, "1"));
    }

    #[test]
    fn test_execute_deregister_module_immutable() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("owner", &[]);
        CONTRACT_OWNER
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();

        let module = Module {
            module_type: ModuleType::AddressList,
            instantiate: InstantiateType::Address("address".to_string()),
            is_mutable: false,
        };

        MODULE_INFO
            .save(deps.as_mut().storage, "1", &module)
            .unwrap();

        MODULE_ADDR
            .save(deps.as_mut().storage, "1", &Addr::unchecked("address"))
            .unwrap();

        let res = execute_deregister_module(deps.as_mut(), info, 1u64.into());
        assert_eq!(ContractError::ModuleImmutable {}, res.unwrap_err());
    }

    #[test]
    fn test_execute_deregister_module_nonexisting_module() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("owner", &[]);
        CONTRACT_OWNER
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();

        let res = execute_deregister_module(deps.as_mut(), info, 1u64.into());

        assert_eq!(ContractError::ModuleDoesNotExist {}, res.unwrap_err());
    }

    #[test]
    fn test_process_module_response() {
        let res: Option<Response> = process_module_response(Ok(Response::new())).unwrap();
        assert_eq!(Some(Response::new()), res);

        let res: Option<Response> = process_module_response(Err(StdError::generic_err(
            "XXXXXXX UnsupportedOperation XXXXXXX",
        )))
        .unwrap();
        assert_eq!(None, res);

        let res: ContractError =
            process_module_response::<Response>(Err(StdError::generic_err("AnotherError")))
                .unwrap_err();
        assert_eq!(
            ContractError::Std(StdError::generic_err("AnotherError")),
            res
        );
    }
}*/
