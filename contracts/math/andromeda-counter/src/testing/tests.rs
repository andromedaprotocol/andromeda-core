use super::mock::{
    decrement, increment, proper_initialization, query_current_amount, query_decrease_amount,
    query_increase_amount, query_initial_amount, query_restriction, reset, set_decrease_amount,
    set_increase_amount, update_restriction,
};
use andromeda_math::counter::{CounterRestriction, State};
use andromeda_std::error::ContractError;
use cosmwasm_std::Attribute;

#[test]
fn test_instantiation_private() {
    let (deps, _) = proper_initialization(
        CounterRestriction::Private,
        State {
            initial_amount: None,
            increase_amount: None,
            decrease_amount: None,
        },
    );
    let initial_amount = query_initial_amount(deps.as_ref()).unwrap().initial_amount;
    assert_eq!(initial_amount, 0);
    let increase_amount = query_increase_amount(deps.as_ref())
        .unwrap()
        .increase_amount;
    assert_eq!(increase_amount, 1);
    let decrease_amount = query_decrease_amount(deps.as_ref())
        .unwrap()
        .decrease_amount;
    assert_eq!(decrease_amount, 1);
    let restriction = query_restriction(deps.as_ref()).unwrap().restriction;
    assert_eq!(restriction, CounterRestriction::Private);
}

#[test]
fn test_instantiation_public() {
    let (deps, _) = proper_initialization(
        CounterRestriction::Public,
        State {
            initial_amount: None,
            increase_amount: None,
            decrease_amount: None,
        },
    );
    let initial_amount = query_initial_amount(deps.as_ref()).unwrap().initial_amount;
    assert_eq!(initial_amount, 0);
    let increase_amount = query_increase_amount(deps.as_ref())
        .unwrap()
        .increase_amount;
    assert_eq!(increase_amount, 1);
    let decrease_amount = query_decrease_amount(deps.as_ref())
        .unwrap()
        .decrease_amount;
    assert_eq!(decrease_amount, 1);
    let restriction = query_restriction(deps.as_ref()).unwrap().restriction;
    assert_eq!(restriction, CounterRestriction::Public);
}

#[test]
fn test_update_restriction() {
    let (mut deps, info) = proper_initialization(
        CounterRestriction::Public,
        State {
            initial_amount: None,
            increase_amount: None,
            decrease_amount: None,
        },
    );
    let external_user = "external".to_string();
    let res =
        update_restriction(deps.as_mut(), CounterRestriction::Private, &external_user).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});
    let restriction = query_restriction(deps.as_ref()).unwrap().restriction;
    assert_eq!(restriction, CounterRestriction::Public);
    update_restriction(
        deps.as_mut(),
        CounterRestriction::Private,
        info.sender.as_ref(),
    )
    .unwrap();
    let restriction = query_restriction(deps.as_ref()).unwrap().restriction;
    assert_eq!(restriction, CounterRestriction::Private);
}

#[test]
fn test_increment_decrement() {
    let (mut deps, info) = proper_initialization(
        CounterRestriction::Private,
        State {
            initial_amount: None,
            increase_amount: None,
            decrease_amount: None,
        },
    );
    let res = increment(deps.as_mut(), info.sender.as_ref()).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            Attribute {
                key: "action".to_string(),
                value: "Increment".to_string()
            },
            Attribute {
                key: "sender".to_string(),
                value: "creator".to_string()
            },
            Attribute {
                key: "current_amount".to_string(),
                value: 1.to_string()
            },
        ]
    );

    let current_amount = query_current_amount(deps.as_ref()).unwrap().current_amount;
    assert_eq!(current_amount, 1);
    increment(deps.as_mut(), info.sender.as_ref()).unwrap();
    let current_amount = query_current_amount(deps.as_ref()).unwrap().current_amount;
    assert_eq!(current_amount, 2);
    increment(deps.as_mut(), info.sender.as_ref()).unwrap();
    let current_amount = query_current_amount(deps.as_ref()).unwrap().current_amount;
    assert_eq!(current_amount, 3);
    decrement(deps.as_mut(), info.sender.as_ref()).unwrap();
    let current_amount = query_current_amount(deps.as_ref()).unwrap().current_amount;
    assert_eq!(current_amount, 2);
    decrement(deps.as_mut(), info.sender.as_ref()).unwrap();
    let current_amount = query_current_amount(deps.as_ref()).unwrap().current_amount;
    assert_eq!(current_amount, 1);
    decrement(deps.as_mut(), info.sender.as_ref()).unwrap();
    let current_amount = query_current_amount(deps.as_ref()).unwrap().current_amount;
    assert_eq!(current_amount, 0);
    decrement(deps.as_mut(), info.sender.as_ref()).unwrap();
    let current_amount = query_current_amount(deps.as_ref()).unwrap().current_amount;
    assert_eq!(current_amount, 0);
}

#[test]
fn test_reset_initial_is_0() {
    let (mut deps, info) = proper_initialization(
        CounterRestriction::Private,
        State {
            initial_amount: None,
            increase_amount: None,
            decrease_amount: None,
        },
    );
    increment(deps.as_mut(), info.sender.as_ref()).unwrap();
    increment(deps.as_mut(), info.sender.as_ref()).unwrap();
    increment(deps.as_mut(), info.sender.as_ref()).unwrap();
    let current_amount = query_current_amount(deps.as_ref()).unwrap().current_amount;
    assert_eq!(current_amount, 3);

    reset(deps.as_mut(), info.sender.as_ref()).unwrap();
    let current_amount = query_current_amount(deps.as_ref()).unwrap().current_amount;
    assert_eq!(current_amount, 0);
}

#[test]
fn test_reset_initial_is_not_0() {
    let (mut deps, info) = proper_initialization(
        CounterRestriction::Private,
        State {
            initial_amount: Some(100),
            increase_amount: None,
            decrease_amount: None,
        },
    );
    increment(deps.as_mut(), info.sender.as_ref()).unwrap();
    increment(deps.as_mut(), info.sender.as_ref()).unwrap();
    increment(deps.as_mut(), info.sender.as_ref()).unwrap();
    let current_amount = query_current_amount(deps.as_ref()).unwrap().current_amount;
    assert_eq!(current_amount, 103);

    reset(deps.as_mut(), info.sender.as_ref()).unwrap();
    let current_amount = query_current_amount(deps.as_ref()).unwrap().current_amount;
    assert_eq!(current_amount, 100);
    decrement(deps.as_mut(), info.sender.as_ref()).unwrap();
    decrement(deps.as_mut(), info.sender.as_ref()).unwrap();
    let current_amount = query_current_amount(deps.as_ref()).unwrap().current_amount;
    assert_eq!(current_amount, 98);
}

#[test]
fn test_set_increase_decrease_amount() {
    let (mut deps, info) = proper_initialization(
        CounterRestriction::Private,
        State {
            initial_amount: None,
            increase_amount: None,
            decrease_amount: None,
        },
    );
    set_increase_amount(deps.as_mut(), 5, info.sender.as_ref()).unwrap();
    set_decrease_amount(deps.as_mut(), 5, info.sender.as_ref()).unwrap();
    let external_user = "external".to_string();
    set_increase_amount(deps.as_mut(), 10, &external_user).unwrap_err();
    set_decrease_amount(deps.as_mut(), 10, &external_user).unwrap_err();
    let increase_amount = query_increase_amount(deps.as_ref())
        .unwrap()
        .increase_amount;
    assert_eq!(increase_amount, 5);
    let decrease_amount = query_decrease_amount(deps.as_ref())
        .unwrap()
        .decrease_amount;
    assert_eq!(decrease_amount, 5);
}

#[test]
fn test_set_increment_private() {
    let (mut deps, _) = proper_initialization(
        CounterRestriction::Private,
        State {
            initial_amount: None,
            increase_amount: None,
            decrease_amount: None,
        },
    );
    let external_user = "external".to_string();
    let res = increment(deps.as_mut(), &external_user).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});
}
