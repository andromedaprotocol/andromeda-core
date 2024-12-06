use crate::contract::{execute, query};
use andromeda_data_storage::string_storage::{
    ExecuteMsg, GetDataOwnerResponse, GetValueResponse, QueryMsg, StringStorage,
    StringStorageRestriction,
};
use cosmwasm_std::{
    coin, from_json, testing::mock_env, BankMsg, CosmosMsg, Decimal, Response, SubMsg,
};

use andromeda_std::{
    ado_base::rates::{LocalRate, LocalRateType, LocalRateValue, PercentRate, Rate, RatesMessage},
    ado_contract::ADOContract,
    amp::{AndrAddr, Recipient},
    error::ContractError,
};

use super::mock::{
    delete_value, proper_initialization, query_value, set_value, set_value_with_funds,
};

#[test]
fn test_instantiation() {
    proper_initialization(StringStorageRestriction::Private);
}

#[test]
fn test_set_and_update_value() {
    let (mut deps, info) = proper_initialization(StringStorageRestriction::Private);
    let value = StringStorage::String("value".to_string());
    set_value(deps.as_mut(), &value, info.sender.as_ref()).unwrap();

    let query_res: GetValueResponse = query_value(deps.as_ref()).unwrap();

    assert_eq!(
        GetValueResponse {
            value: value.into()
        },
        query_res
    );

    let value = StringStorage::String("value2".to_string());
    set_value(deps.as_mut(), &value, info.sender.as_ref()).unwrap();

    let query_res: GetValueResponse = query_value(deps.as_ref()).unwrap();

    assert_eq!(
        GetValueResponse {
            value: value.into()
        },
        query_res
    );
}

#[test]
fn test_set_value_with_tax() {
    let (mut deps, info) = proper_initialization(StringStorageRestriction::Private);
    let value = StringStorage::String("value".to_string());
    let tax_recipient = "tax_recipient";

    // Set percent rates
    let set_percent_rate_msg = ExecuteMsg::Rates(RatesMessage::SetRate {
        action: "StringStorageSetValue".to_string(),
        rate: Rate::Local(LocalRate {
            rate_type: LocalRateType::Additive,
            recipient: Recipient {
                address: AndrAddr::from_string(String::default()),
                msg: None,
                ibc_recovery_address: None,
            },
            value: LocalRateValue::Percent(PercentRate {
                percent: Decimal::one(),
            }),
            description: None,
        }),
    });

    let err = execute(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        set_percent_rate_msg,
    )
    .unwrap_err();

    assert_eq!(err, ContractError::InvalidRate {});

    let rate: Rate = Rate::Local(LocalRate {
        rate_type: LocalRateType::Additive,
        recipient: Recipient {
            address: AndrAddr::from_string(tax_recipient.to_string()),
            msg: None,
            ibc_recovery_address: None,
        },
        value: LocalRateValue::Flat(coin(20_u128, "uandr")),
        description: None,
    });

    // Set rates
    ADOContract::default()
        .set_rates(deps.as_mut().storage, "SetValue", rate)
        .unwrap();

    // Sent the exact amount required for tax
    let res = set_value_with_funds(
        deps.as_mut(),
        &value,
        info.sender.as_ref(),
        coin(20_u128, "uandr".to_string()),
    )
    .unwrap();
    let expected_response: Response = Response::new()
        .add_submessage(SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: tax_recipient.to_string(),
            amount: vec![coin(20, "uandr")],
        })))
        .add_attributes(vec![("method", "set_value"), ("sender", "creator")])
        .add_attribute("value", format!("{value:?}"));
    assert_eq!(expected_response, res);

    // Sent less than amount required for tax
    let err = set_value_with_funds(
        deps.as_mut(),
        &value,
        info.sender.as_ref(),
        coin(19_u128, "uandr".to_string()),
    )
    .unwrap_err();
    assert_eq!(err, ContractError::InsufficientFunds {});

    // Sent more than required amount for tax
    let res = set_value_with_funds(
        deps.as_mut(),
        &value,
        info.sender.as_ref(),
        coin(200_u128, "uandr".to_string()),
    )
    .unwrap();
    let expected_response: Response = Response::new()
        .add_submessage(SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: tax_recipient.to_string(),
            amount: vec![coin(20, "uandr")],
        })))
        // 200 was sent, but the tax is only 20, so we send back the difference
        .add_submessage(SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "creator".to_string(),
            amount: vec![coin(180, "uandr")],
        })))
        .add_attributes(vec![("method", "set_value"), ("sender", "creator")])
        .add_attribute("value", format!("{value:?}"));
    assert_eq!(expected_response, res);
}

struct TestHandleStringStorage {
    name: &'static str,
    string_storage: StringStorage,
    expected_error: Option<ContractError>,
}

#[test]
fn test_set_value_invalid() {
    let test_cases = vec![TestHandleStringStorage {
        name: "Empty String",
        string_storage: StringStorage::String("".to_string()),
        expected_error: Some(ContractError::EmptyString {}),
    }];

    for test in test_cases {
        let res = test.string_storage.validate();

        if let Some(err) = test.expected_error {
            assert_eq!(res.unwrap_err(), err, "{}", test.name);
            continue;
        }

        assert!(res.is_ok())
    }
}

#[test]
fn test_delete_value() {
    let (mut deps, info) = proper_initialization(StringStorageRestriction::Private);
    let value = StringStorage::String("value".to_string());
    set_value(deps.as_mut(), &value, info.sender.as_ref()).unwrap();
    delete_value(deps.as_mut(), info.sender.as_ref()).unwrap();
    query_value(deps.as_ref()).unwrap_err();
}

#[test]
fn test_restriction_private() {
    let (mut deps, info) = proper_initialization(StringStorageRestriction::Private);

    let value = StringStorage::String("value".to_string());
    let external_user = "external".to_string();

    // Set Value as owner
    set_value(deps.as_mut(), &value, info.sender.as_ref()).unwrap();
    delete_value(deps.as_mut(), info.sender.as_ref()).unwrap();
    query_value(deps.as_ref()).unwrap_err();

    // Set Value as external user
    // This should error
    set_value(deps.as_mut(), &value, &external_user).unwrap_err();
    // Set a value by owner so we can test delete for it
    set_value(deps.as_mut(), &value, info.sender.as_ref()).unwrap();
    // Delete value set by owner by external user
    // This will error
    delete_value(deps.as_mut(), &external_user).unwrap_err();

    // Value is still present
    query_value(deps.as_ref()).unwrap();
}

#[test]
fn test_restriction_public() {
    let (mut deps, info) = proper_initialization(StringStorageRestriction::Public);

    let value = StringStorage::String("value".to_string());
    let external_user = "external".to_string();

    // Set Value as owner
    set_value(deps.as_mut(), &value, info.sender.as_ref()).unwrap();
    delete_value(deps.as_mut(), info.sender.as_ref()).unwrap();
    // This should error
    query_value(deps.as_ref()).unwrap_err();

    // Set Value as external user
    set_value(deps.as_mut(), &value, &external_user).unwrap();
    delete_value(deps.as_mut(), &external_user).unwrap();
    // This should error
    query_value(deps.as_ref()).unwrap_err();

    // Set Value as owner
    set_value(deps.as_mut(), &value, info.sender.as_ref()).unwrap();
    // Delete the value as external user
    delete_value(deps.as_mut(), &external_user).unwrap();
    // This should error
    query_value(deps.as_ref()).unwrap_err();
}

#[test]
fn test_restriction_restricted() {
    let (mut deps, info) = proper_initialization(StringStorageRestriction::Restricted);

    let value = StringStorage::String("value".to_string());
    let value2 = StringStorage::String("value2".to_string());
    let external_user = "external".to_string();
    let external_user2 = "external2".to_string();

    // Set Value as owner
    set_value(deps.as_mut(), &value, info.sender.as_ref()).unwrap();
    delete_value(deps.as_mut(), info.sender.as_ref()).unwrap();
    // This should error
    query_value(deps.as_ref()).unwrap_err();

    // Set Value as external user
    set_value(deps.as_mut(), &value, &external_user).unwrap();
    delete_value(deps.as_mut(), &external_user).unwrap();
    // This should error
    query_value(deps.as_ref()).unwrap_err();

    // Set Value as owner and try to delete as external user
    set_value(deps.as_mut(), &value, info.sender.as_ref()).unwrap();
    // Try to modify it as external user
    set_value(deps.as_mut(), &value2, &external_user).unwrap_err();
    // Delete the value as external user, this should error
    delete_value(deps.as_mut(), &external_user).unwrap_err();

    query_value(deps.as_ref()).unwrap();

    // Set Value as external user and try to delete as owner
    set_value(deps.as_mut(), &value, info.sender.as_ref()).unwrap();
    // Delete the value as external user, this will success as owner has permission to do anything
    delete_value(deps.as_mut(), info.sender.as_ref()).unwrap();

    query_value(deps.as_ref()).unwrap_err();

    // Set Value as external user 1 and try to delete as external user 2
    set_value(deps.as_mut(), &value, &external_user).unwrap();
    // Delete the value as external user, this will error
    delete_value(deps.as_mut(), &external_user2).unwrap_err();

    query_value(deps.as_ref()).unwrap();
}

#[test]
fn test_query_data_owner() {
    let (mut deps, _) = proper_initialization(StringStorageRestriction::Restricted);
    let external_user = "external".to_string();
    let external_user2 = "external2".to_string();
    let value = StringStorage::String("value".to_string());
    set_value(deps.as_mut(), &value, &external_user.clone()).unwrap();

    let res: GetDataOwnerResponse =
        from_json(query(deps.as_ref(), mock_env(), QueryMsg::GetDataOwner {}).unwrap()).unwrap();

    assert_eq!(
        res,
        GetDataOwnerResponse {
            owner: AndrAddr::from_string(external_user.clone())
        }
    );

    let res = delete_value(deps.as_mut(), &external_user2).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});

    delete_value(deps.as_mut(), &external_user).unwrap();

    query(deps.as_ref(), mock_env(), QueryMsg::GetDataOwner {}).unwrap_err();
}
