use crate::contract::{execute, instantiate, query};
use crate::testing::mock_querier::mock_dependencies_custom;
use andromeda_fungible_tokens::cw20::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::ado_base::permissioning::{LocalPermission, Permission};
use andromeda_std::ado_base::rates::{LocalRate, LocalRateType, LocalRateValue, PercentRate, Rate};
use andromeda_std::ado_contract::ADOContract;
use andromeda_std::amp::{AndrAddr, Recipient};
use andromeda_std::common::context::ExecuteContext;

use andromeda_std::{error::ContractError, testing::mock_querier::MOCK_KERNEL_CONTRACT};
use cosmwasm_std::{attr, Decimal, Event};
use cosmwasm_std::{
    testing::{message_info, mock_env},
    to_json_binary, Response, Uint128,
};

use cw20::{Cw20Coin, Cw20ReceiveMsg};
use cw20_base::state::BALANCES;

use super::mock_querier::{TestDeps, MOCK_CW20_CONTRACT};


use andromeda_fungible_tokens::state::LOCKED_TOKENS;

fn init(deps: &mut TestDeps) -> Response {
    let sender = deps.api.addr_make("sender");
    let rates_recipient = deps.api.addr_make("rates_recipient");
    let royalty_recipient = deps.api.addr_make("royalty_recipient");
    let msg = InstantiateMsg {
        name: MOCK_CW20_CONTRACT.into(),
        symbol: "Symbol".into(),
        decimals: 6,
        initial_balances: vec![
            Cw20Coin {
                amount: 1000u128.into(),
                address: sender.to_string(),
            },
            Cw20Coin {
                amount: 1u128.into(),
                address: rates_recipient.to_string(),
            },
            Cw20Coin {
                amount: 1u128.into(),
                address: royalty_recipient.to_string(),
            },
        ],
        mint: None,
        marketing: None,
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
    };

    let owner = deps.api.addr_make("owner");
    let info = message_info(&owner, &[]);
    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap()
}

#[test]
fn test_andr_query() {
    let mut deps = mock_dependencies_custom(&[]);
    let _res = init(&mut deps);

    let msg = QueryMsg::Owner {};
    let res = query(deps.as_ref(), mock_env(), msg);
    // Test that the query is hooked up correctly.
    assert!(res.is_ok())
}

#[test]
fn test_transfer() {
    let mut deps = mock_dependencies_custom(&[]);
    let res = init(&mut deps);
    let owner = deps.api.addr_make("owner");
    assert_eq!(
        Response::new()
            .add_attribute("method", "instantiate")
            .add_attribute("type", "cw20")
            .add_attribute("kernel_address", MOCK_KERNEL_CONTRACT)
            .add_attribute("owner", owner.to_string()),
        res
    );

    let sender = deps.api.addr_make("sender");
    assert_eq!(
        Uint128::from(1000u128),
        BALANCES.load(deps.as_ref().storage, &sender).unwrap()
    );

    let other = deps.api.addr_make("other");
    let msg = ExecuteMsg::Transfer {
        recipient: AndrAddr::from_string(other.to_string()),
        amount: 100u128.into(),
    };

    let royalty_recipient = deps.api.addr_make("royalty_recipient");
    // Set a royalty of 10% to be paid to royalty_recipient
    let rate = Rate::Local(LocalRate {
        rate_type: LocalRateType::Deductive,
        recipient: Recipient {
            address: AndrAddr::from_string(royalty_recipient.to_string()),
            msg: None,
            ibc_recovery_address: None,
        },
        value: LocalRateValue::Percent(PercentRate {
            percent: Decimal::percent(10),
        }),
        description: None,
    });

    // Set rates
    ADOContract::default()
        .set_rates(deps.as_mut().storage, "Transfer", rate)
        .unwrap();

    // The expected events for the royalty
    let expected_event = Event::new("royalty").add_attributes(vec![
        attr("deducted", "10cosmwasm1jpev2csrppg792t22rn8z8uew8h3sjcpglcd0qv9g8gj8ky922tscp8avs"),
        attr("payment", "cosmwasm15r4uytzhmpnefdw0ykpfjrmja37tpcf092wzyfjkfe40g7zf3w4svuasg3<10cosmwasm1jpev2csrppg792t22rn8z8uew8h3sjcpglcd0qv9g8gj8ky922tscp8avs"),
    ]);

    // Blacklist the sender who otherwise would have been able to call the function successfully
    let permission = Permission::Local(LocalPermission::blacklisted(None, None));
    let actors = vec![AndrAddr::from_string(sender.to_string())];
    let action = "Transfer";
    let ctx = ExecuteContext::new(deps.as_mut(), message_info(&owner, &[]), mock_env());
    ADOContract::default()
        .execute_set_permission(ctx, actors, action, permission)
        .unwrap();
    let info = message_info(&sender, &[]);
    let err = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap_err();

    assert_eq!(err, ContractError::Unauthorized {});

    // Now whitelist the sender, that should allow him to call the function successfully
    let permission = Permission::Local(LocalPermission::whitelisted(None, None));
    let actors = vec![AndrAddr::from_string(sender.to_string())];
    let action = "Transfer";
    let owner = deps.api.addr_make("owner");
    let ctx = ExecuteContext::new(deps.as_mut(), message_info(&owner, &[]), mock_env());
    ADOContract::default()
        .execute_set_permission(ctx, actors, action, permission)
        .unwrap();
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_event(expected_event)
            .add_attribute("action", "transfer")
            .add_attribute("from", sender.to_string())
            .add_attribute("to", other.to_string())
            .add_attribute("amount", "90"),
        res
    );

    // Funds deducted from the sender (100 for send, 10 for tax).
    assert_eq!(
        Uint128::from(900u128),
        BALANCES.load(deps.as_ref().storage, &sender).unwrap()
    );

    // Funds given to the receiver. Remove 10 for the royalty
    assert_eq!(
        Uint128::from(100u128 - 10u128),
        BALANCES.load(deps.as_ref().storage, &other).unwrap()
    );

    // Royalty given to royalty_recipient
    assert_eq!(
        Uint128::from(1u128 + 10u128),
        BALANCES
            .load(deps.as_ref().storage, &royalty_recipient)
            .unwrap()
    );
}

#[test]
fn test_send() {
    let mut deps = mock_dependencies_custom(&[]);
    let sender = deps.api.addr_make("sender");
    let info = message_info(&sender, &[]);

    let res = init(&mut deps);

    let owner = deps.api.addr_make("owner");
    assert_eq!(
        Response::new()
            .add_attribute("method", "instantiate")
            .add_attribute("type", "cw20")
            .add_attribute("kernel_address", MOCK_KERNEL_CONTRACT)
            .add_attribute("owner", owner.to_string()),
        res
    );

    assert_eq!(
        Uint128::from(1000u128),
        BALANCES.load(deps.as_ref().storage, &sender).unwrap()
    );

    let rates_recipient = deps.api.addr_make("rates_recipient");
    let rate = Rate::Local(LocalRate {
        rate_type: LocalRateType::Additive,
        recipient: Recipient {
            address: AndrAddr::from_string(rates_recipient.to_string()),
            msg: None,
            ibc_recovery_address: None,
        },
        value: LocalRateValue::Percent(PercentRate {
            percent: Decimal::percent(10),
        }),
        description: None,
    });

    // Set rates
    ADOContract::default()
        .set_rates(deps.as_mut().storage, "Send", rate)
        .unwrap();

    let contract = deps.api.addr_make("contract");
    let msg = ExecuteMsg::Send {
        contract: AndrAddr::from_string(contract.to_string()),
        amount: 100u128.into(),
        msg: to_json_binary(&"msg").unwrap(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    let expected_event = Event::new("tax").add_attributes(vec![attr(
        "payment",
        format!(
            "{}<10cosmwasm1jpev2csrppg792t22rn8z8uew8h3sjcpglcd0qv9g8gj8ky922tscp8avs",
            rates_recipient
        ),
    )]);

    assert_eq!(
        Response::new()
            .add_attribute("action", "send")
            .add_attribute("from", sender.to_string())
            .add_attribute("to", contract.to_string())
            .add_attribute("amount", "100")
            .add_message(
                Cw20ReceiveMsg {
                    sender: sender.to_string(),
                    amount: 100u128.into(),
                    msg: to_json_binary(&"msg").unwrap(),
                }
                .into_cosmos_msg(contract.to_string())
                .unwrap(),
            )
            .add_event(expected_event),
        res
    );

    // Funds deducted from the sender (100 for send, 10 for tax).
    assert_eq!(
        Uint128::from(1_000u128 - 100u128 - 10u128),
        BALANCES.load(deps.as_ref().storage, &sender).unwrap()
    );

    // Funds given to the receiver.
    assert_eq!(
        Uint128::from(100u128),
        BALANCES.load(deps.as_ref().storage, &contract).unwrap()
    );

    // The rates recipient started with a balance of 1, and received 10 from the tax
    assert_eq!(
        Uint128::from(1u128 + 10u128),
        BALANCES
            .load(deps.as_ref().storage, &rates_recipient)
            .unwrap()
    );
}

//to make this test work add the factory contract address
#[test]
#[ignore] // TODO: Add real factory contract address
fn test_lock_and_mint_factory_success() {
    let mut deps = mock_dependencies_custom(&[]);
    init(&mut deps);
    let owner = deps.api.addr_make("owner");
    let factory_contract = deps.api.addr_make("factory");
    
    let msg = ExecuteMsg::LockAndMintFactory {
        amount: Uint128::from(100u128),
        factory_contract: AndrAddr::from_string(factory_contract.to_string()),
    };
    
    let info = message_info(&owner, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    
    // Check that the response has the right attributes
    assert!(res.attributes.iter().any(|attr| attr.key == "action" && attr.value == "lock_and_mint_factory"));
    assert!(res.attributes.iter().any(|attr| attr.key == "cw20_tokens_locked" && attr.value == "100"));
    
    // Check that tokens are locked in state
    let locked_info = LOCKED_TOKENS.load(deps.as_ref().storage).unwrap();
    assert_eq!(locked_info.amount, Uint128::from(100u128));
    assert_eq!(locked_info.user, owner);
    
    // Check that owner's balance was reduced by the locked amount
    let owner_balance = BALANCES.load(deps.as_ref().storage, &owner).unwrap();
    assert_eq!(owner_balance, Uint128::from(900u128)); // 1000 - 100 locked
}

#[test] 
fn test_lock_and_mint_factory_insufficient_balance() {
    let mut deps = mock_dependencies_custom(&[]);
    init(&mut deps);
    
    let owner = deps.api.addr_make("owner");
    let factory_contract = deps.api.addr_make("factory");
    
    // Try to lock more tokens than available (owner has 0 initial balance in this setup)
    let msg = ExecuteMsg::LockAndMintFactory {
        amount: Uint128::from(1000000u128), // Way more than available
        factory_contract: AndrAddr::from_string(factory_contract.to_string()),
    };
    
    let info = message_info(&owner, &[]);
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    
    // Should fail with insufficient balance error
    assert!(err.to_string().contains("Insufficient available balance"));
}

#[test]
fn test_lock_and_mint_factory_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);
    init(&mut deps);
    
    let not_owner = deps.api.addr_make("not_owner");
    let factory_contract = deps.api.addr_make("factory");
    
    let msg = ExecuteMsg::LockAndMintFactory {
        amount: Uint128::from(100u128),
        factory_contract: AndrAddr::from_string(factory_contract.to_string()),
    };
    
    let info = message_info(&not_owner, &[]);
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    
    // Should fail with unauthorized error
    assert_eq!(err, ContractError::Unauthorized {});
}

#[test]
fn test_transfer_with_locked_tokens() {
    let mut deps = mock_dependencies_custom(&[]);
    init(&mut deps);
    
    let sender = deps.api.addr_make("sender");
    let recipient = deps.api.addr_make("recipient");
    
    // First, set up some locked tokens for sender
    // We'll manually save some locked tokens to simulate the locked state
    let locked_info = andromeda_fungible_tokens::state::FactoryInfo {
        factory_contract: AndrAddr::from_string("factory"),
        amount: Uint128::from(800u128), // Lock 800 out of 1000
        user: sender.clone(),
    };
    LOCKED_TOKENS.save(deps.as_mut().storage, &locked_info).unwrap();
    
    // Try to transfer 300 tokens (should succeed since available = 1000 - 800 = 200, but 300 > 200)
    let msg = ExecuteMsg::Transfer {
        recipient: AndrAddr::from_string(recipient.to_string()),
        amount: Uint128::from(300u128),
    };
    
    let info = message_info(&sender, &[]);
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    
    // Should fail because trying to transfer more than available balance
    assert!(err.to_string().contains("Insufficient available balance"));
}

#[test]
fn test_transfer_with_available_tokens() {
    let mut deps = mock_dependencies_custom(&[]);
    init(&mut deps);
    
    let sender = deps.api.addr_make("sender");
    let recipient = deps.api.addr_make("recipient");
    
    // Set up locked tokens for sender (lock 800 out of 1000)
    let locked_info = andromeda_fungible_tokens::state::FactoryInfo {
        factory_contract: AndrAddr::from_string("factory"),
        amount: Uint128::from(800u128),
        user: sender.clone(),
    };
    LOCKED_TOKENS.save(deps.as_mut().storage, &locked_info).unwrap();
    
    // Try to transfer 100 tokens (should succeed since available = 1000 - 800 = 200)
    let msg = ExecuteMsg::Transfer {
        recipient: AndrAddr::from_string(recipient.to_string()),
        amount: Uint128::from(100u128),
    };
    
    let info = message_info(&sender, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    
    // Should succeed
    assert!(res.is_ok());
}

#[test]
fn test_burn_with_locked_tokens() {
    let mut deps = mock_dependencies_custom(&[]);
    init(&mut deps);
    
    let sender = deps.api.addr_make("sender");
    
    // Set up locked tokens for sender
    let locked_info = andromeda_fungible_tokens::state::FactoryInfo {
        factory_contract: AndrAddr::from_string("factory"),
        amount: Uint128::from(900u128), // Lock 900 out of 1000
        user: sender.clone(),
    };
    LOCKED_TOKENS.save(deps.as_mut().storage, &locked_info).unwrap();
    
    // Try to burn 200 tokens (should fail since available = 100)
    let msg = ExecuteMsg::Burn {
        amount: Uint128::from(200u128),
    };
    
    let info = message_info(&sender, &[]);
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    
    // Should fail
    assert!(err.to_string().contains("Insufficient available balance"));
}

#[test]
fn test_unlock_from_factory_success() {
    let mut deps = mock_dependencies_custom(&[]);
    init(&mut deps);
    
    let owner = deps.api.addr_make("owner");
    let user = deps.api.addr_make("user");
    
    // Setup: Lock 100 tokens for user, reduce their balance
    let locked_info = andromeda_fungible_tokens::state::FactoryInfo {
        factory_contract: AndrAddr::from_string("factory"),
        amount: Uint128::from(100u128),
        user: user.clone(),
    };
    LOCKED_TOKENS.save(deps.as_mut().storage, &locked_info).unwrap();
    BALANCES.save(deps.as_mut().storage, &user, &Uint128::from(900u128)).unwrap();
    
    // Owner unlocks 50 tokens for user
    let msg = ExecuteMsg::UnlockFromFactory {
        user: user.to_string(),
        amount: Uint128::from(50u128),
    };
    
    let info = message_info(&owner, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    
    // Check response attributes
    assert!(res.attributes.iter().any(|attr| attr.key == "action" && attr.value == "unlock_from_factory"));
    assert!(res.attributes.iter().any(|attr| attr.key == "amount_unlocked" && attr.value == "50"));
    
    // Check user's balance increased
    let user_balance = BALANCES.load(deps.as_ref().storage, &user).unwrap();
    assert_eq!(user_balance, Uint128::from(950u128)); // 900 + 50
    
    // Check locked amount decreased
    let locked_info = LOCKED_TOKENS.load(deps.as_ref().storage).unwrap();
    assert_eq!(locked_info.amount, Uint128::from(50u128)); // 100 - 50
}

#[test]
fn test_unlock_from_factory_full_unlock() {
    let mut deps = mock_dependencies_custom(&[]);
    init(&mut deps);
    
    let owner = deps.api.addr_make("owner");
    let user = deps.api.addr_make("user");
    
    // Setup: Lock 100 tokens for user
    let locked_info = andromeda_fungible_tokens::state::FactoryInfo {
        factory_contract: AndrAddr::from_string("factory"),
        amount: Uint128::from(100u128),
        user: user.clone(),
    };
    LOCKED_TOKENS.save(deps.as_mut().storage, &locked_info).unwrap();
    BALANCES.save(deps.as_mut().storage, &user, &Uint128::from(900u128)).unwrap();
    
    // Owner unlocks all 100 tokens
    let msg = ExecuteMsg::UnlockFromFactory {
        user: user.to_string(),
        amount: Uint128::from(100u128),
    };
    
    let info = message_info(&owner, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    
    // Check response
    assert!(res.attributes.iter().any(|attr| attr.key == "amount_unlocked" && attr.value == "100"));
    
    // Check user's balance fully restored
    let user_balance = BALANCES.load(deps.as_ref().storage, &user).unwrap();
    assert_eq!(user_balance, Uint128::from(1000u128)); // 900 + 100
    
    // Check locked tokens state was cleared
    let locked_tokens = LOCKED_TOKENS.may_load(deps.as_ref().storage).unwrap();
    assert!(locked_tokens.is_none());
}

#[test]
fn test_unlock_from_factory_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);
    init(&mut deps);
    
    let not_owner = deps.api.addr_make("not_owner");
    let user = deps.api.addr_make("user");
    
    let msg = ExecuteMsg::UnlockFromFactory {
        user: user.to_string(),
        amount: Uint128::from(50u128),
    };
    
    let info = message_info(&not_owner, &[]);
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    
    // Should fail with unauthorized error
    assert_eq!(err, ContractError::Unauthorized {});
}
