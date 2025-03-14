use andromeda_adodb::register_contract;
use andromeda_cw20::CW20Contract;
use andromeda_finance::splitter::AddressPercent;
use andromeda_fixed_amount_splitter::{
    fixed_amount_splitter_instantiate, FixedAmountSplitterContract,
};
use andromeda_splitter::mock::{
    mock_andromeda_splitter, mock_splitter_instantiate_msg, mock_splitter_send_msg, MockSplitter,
};
use andromeda_std::{
    amp::{AndrAddr, Recipient},
    error::ContractError,
};
use andromeda_testing::{
    mock::mock_app,
    mock_builder::MockAndromedaBuilder,
    mock_contract::{MockADO, MockContract},
};
use cosmwasm_std::{coin, Addr, Binary, Decimal};

use andromeda_std::os::kernel::Cw20HookMsg;
use andromeda_std::{amp::messages::AMPMsg, os};
use andromeda_testing::{ado_deployer, InterchainTestEnv};
use cosmwasm_std::{to_json_binary, Coin, Uint128};
use cw20::{BalanceResponse, Cw20Coin};
use cw_orch::prelude::*;
use rstest::*;
use std::vec;

ado_deployer!(
    deploy_splitter,
    SplitterContract<MockBase<MockApiBech32>>,
    &InstantiateMsg
);

#[test]
fn kernel() {
    let mut router = mock_app(None);
    let andr = MockAndromedaBuilder::new(&mut router, "admin")
        .with_wallets(vec![
            ("owner", vec![coin(1000, "uandr")]),
            ("user1", vec![]),
        ])
        .with_contracts(vec![("splitter", mock_andromeda_splitter())])
        .build(&mut router);

    let owner = andr.get_wallet("owner");
    let user1 = andr.get_wallet("user1");

    let splitter_msg = mock_splitter_instantiate_msg(
        vec![AddressPercent::new(
            Recipient::from_string(user1.to_string()).with_ibc_recovery(owner.clone()),
            Decimal::one(),
        )],
        andr.kernel.addr().clone(),
        None,
        None,
        None,
    );

    let res = andr
        .kernel
        .execute_create(
            &mut router,
            owner.clone(),
            "splitter",
            splitter_msg,
            Some(AndrAddr::from_string(andr.admin_address.to_string())),
            None,
        )
        .unwrap();

    let event_key = res
        .events
        .iter()
        .position(|ev| ev.ty == "instantiate")
        .unwrap();
    let inst_event = res.events.get(event_key).unwrap();
    let attr_key = inst_event
        .attributes
        .iter()
        .position(|attr| attr.key == "_contract_address")
        .unwrap();
    let attr = inst_event.attributes.get(attr_key).unwrap();
    let addr: Addr = Addr::unchecked(attr.value.clone());
    let splitter = MockSplitter::from(addr);
    splitter
        .accept_ownership(&mut router, andr.admin_address.clone())
        .unwrap();

    let splitter_owner = splitter.query_owner(&router);

    assert_eq!(splitter_owner, andr.admin_address.to_string());

    let res = andr
        .kernel
        .execute_send(
            &mut router,
            owner.clone(),
            splitter.addr(),
            mock_splitter_send_msg(None),
            vec![coin(100, "uandr")],
            None,
        )
        .unwrap();

    let user1_balance = router
        .wrap()
        .query_balance(user1, "uandr".to_string())
        .unwrap();

    // user1 had one coin before the splitter execute msg which is expected to increase his balance by 100uandr
    assert_eq!(user1_balance, coin(100, "uandr"));
    assert_eq!(user1_balance, coin(100, "uandr"));

    let owner_balance = router
        .wrap()
        .query_balance(owner, "uandr".to_string())
        .unwrap();

    // The owner's balance should be his starting balance subtracted by the 100 he sent with the splitter execute msg
    assert_eq!(owner_balance, coin(900, "uandr"));
    assert_eq!(owner_balance, coin(900, "uandr"));

    assert!(res.data.is_none());
}

#[rstest]
#[case(true)] // with message
#[case(false)] // without message
fn test_fixed_amount_splitter_local(#[case] with_message: bool) {
    // Setup environment
    let env = InterchainTestEnv::new();
    let recipient = env.juno.chain.addr_make("recipient");

    // Deploy splitter
    let splitter_juno = FixedAmountSplitterContract::new(env.juno.chain.clone());
    splitter_juno.upload().unwrap();

    // Instantiate the splitter
    splitter_juno
        .instantiate(
            fixed_amount_splitter_instantiate!(env.juno.aos, recipient, env.juno.denom),
            None,
            &vec![],
        )
        .unwrap();

    // Register contract
    register_contract!(env.juno.aos, splitter_juno, "fixed-amount-splitter");

    // Create message based on test case
    let binary_msg = if with_message {
        to_json_binary(&andromeda_finance::fixed_amount_splitter::ExecuteMsg::Send { config: None })
            .unwrap()
    } else {
        Binary::default()
    };

    let message = AMPMsg::new(
        AndrAddr::from_string(splitter_juno.address().unwrap().clone()),
        binary_msg,
        Some(vec![Coin {
            amount: Uint128::new(100000000),
            denom: env.juno.denom.clone(),
        }]),
    );

    // Execute transfer
    env.juno
        .aos
        .kernel
        .execute(
            &os::kernel::ExecuteMsg::Send { message },
            &[Coin {
                amount: Uint128::new(100000000),
                denom: env.juno.denom.clone(),
            }],
        )
        .unwrap();

    // Check balances
    if with_message {
        // When message is provided, funds should be sent to recipient
        let balances = env.juno.chain.query_all_balances(&recipient).unwrap();
        assert_eq!(balances.len(), 1);
        assert_eq!(balances[0].denom, env.juno.denom);
        assert_eq!(balances[0].amount.u128(), 100);
    } else {
        // When no message is provided, funds should remain in the contract
        let balances = env
            .juno
            .chain
            .query_all_balances(&splitter_juno.address().unwrap())
            .unwrap();
        assert_eq!(balances.len(), 1);
        assert_eq!(balances[0].denom, env.juno.denom);
        assert_eq!(balances[0].amount.u128(), 100000000);
    }
}

#[test]
fn test_fixed_amount_splitter_local_funds_mismatch() {
    // Setup environment
    let env = InterchainTestEnv::new();
    let recipient = env.juno.chain.addr_make("recipient");

    // Deploy splitter
    let splitter_juno = FixedAmountSplitterContract::new(env.juno.chain.clone());
    splitter_juno.upload().unwrap();

    // Instantiate the splitter
    splitter_juno
        .instantiate(
            fixed_amount_splitter_instantiate!(env.juno.aos, recipient, env.juno.denom),
            None,
            &vec![],
        )
        .unwrap();

    // Register contract
    register_contract!(env.juno.aos, splitter_juno, "fixed-amount-splitter");

    let message = AMPMsg::new(
        AndrAddr::from_string(splitter_juno.address().unwrap().clone()),
        Binary::default(),
        Some(vec![Coin {
            amount: Uint128::new(100000000),
            denom: env.juno.denom.clone(),
        }]),
    );

    // Execute with insufficient funds
    let err: ContractError = env
        .juno
        .aos
        .kernel
        .execute(
            &os::kernel::ExecuteMsg::Send { message },
            &[Coin {
                amount: Uint128::new(2), // Less than required
                denom: env.juno.denom.clone(),
            }],
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(err, ContractError::InsufficientFunds {});
}

#[rstest]
#[case(true)] // with message
#[case(false)] // without message
fn test_fixed_amount_splitter_cw20(#[case] with_message: bool) {
    // Setup environment
    let env = InterchainTestEnv::new();
    let recipient = env.juno.chain.addr_make("recipient");

    // Set up sender based on test case
    let sender = if with_message {
        env.juno.aos.kernel.address().unwrap().to_string()
    } else {
        env.juno.chain.addr_make("sender").to_string()
    };

    // Deploy CW20 token
    let mut cw20_token = CW20Contract::new(env.juno.chain.clone());
    cw20_token.upload().unwrap();

    cw20_token
        .instantiate(
            &andromeda_fungible_tokens::cw20::InstantiateMsg {
                name: "Test Token".to_string(),
                symbol: "TEST".to_string(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: sender.clone(),
                    amount: Uint128::new(1000000000),
                }],
                mint: None,
                marketing: None,
                kernel_address: env.juno.aos.kernel.address().unwrap().into_string(),
                owner: None,
            },
            None,
            &vec![],
        )
        .unwrap();

    // Deploy and instantiate splitter if needed
    let splitter_juno = FixedAmountSplitterContract::new(env.juno.chain.clone());

    if with_message {
        splitter_juno.upload().unwrap();
        splitter_juno
            .instantiate(
                fixed_amount_splitter_instantiate!(
                    env.juno.aos,
                    recipient,
                    cw20_token.address().unwrap().to_string()
                ),
                None,
                &vec![],
            )
            .unwrap();

        // Register splitter contract
        register_contract!(env.juno.aos, splitter_juno, "fixed-amount-splitter");
    }

    // Register CW20 contract
    register_contract!(env.juno.aos, cw20_token, "cw20");

    // Create message
    let target_addr = if with_message {
        splitter_juno.address().unwrap().clone()
    } else {
        recipient.clone()
    };

    let inner_msg = if with_message {
        to_json_binary(
            &andromeda_finance::fixed_amount_splitter::Cw20HookMsg::Send { config: None },
        )
        .unwrap()
    } else {
        Binary::default()
    };

    let message = AMPMsg::new(
        AndrAddr::from_string(target_addr),
        inner_msg,
        Some(vec![Coin {
            amount: Uint128::new(100000000),
            denom: cw20_token.address().unwrap().to_string(),
        }]),
    );

    // Set sender for CW20 token
    cw20_token.set_sender(&Addr::unchecked(sender));

    // Execute CW20 send
    cw20_token
        .execute(
            &andromeda_fungible_tokens::cw20::ExecuteMsg::Send {
                contract: AndrAddr::from_string(
                    env.juno.aos.kernel.address().unwrap().into_string(),
                ),
                amount: Uint128::new(100000000),
                msg: to_json_binary(&Cw20HookMsg::Send { message }).unwrap(),
            },
            &vec![],
        )
        .unwrap();

    // Check balances
    let balance: BalanceResponse = cw20_token
        .query(&andromeda_fungible_tokens::cw20::QueryMsg::Balance {
            address: recipient.to_string(),
        })
        .unwrap();

    if with_message {
        assert_eq!(balance.balance.u128(), 100); // Only the specified amount in the splitter
    } else {
        assert_eq!(balance.balance.u128(), 100000000); // Full amount
    }
}

#[test]
fn test_fixed_amount_splitter_cw20_funds_mismatch() {
    // Setup environment
    let env = InterchainTestEnv::new();
    let recipient = env.juno.chain.addr_make("recipient");
    let sender = env.juno.chain.addr_make("sender");

    // Deploy CW20 token
    let mut cw20_token = CW20Contract::new(env.juno.chain.clone());
    cw20_token.upload().unwrap();

    cw20_token
        .instantiate(
            &andromeda_fungible_tokens::cw20::InstantiateMsg {
                name: "Test Token".to_string(),
                symbol: "TEST".to_string(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: sender.to_string(),
                    amount: Uint128::new(1000000000),
                }],
                mint: None,
                marketing: None,
                kernel_address: env.juno.aos.kernel.address().unwrap().into_string(),
                owner: None,
            },
            None,
            &vec![],
        )
        .unwrap();

    let message = AMPMsg::new(
        AndrAddr::from_string(recipient.clone()),
        Binary::default(),
        Some(vec![Coin {
            amount: Uint128::new(100000000),
            denom: cw20_token.address().unwrap().to_string(),
        }]),
    );

    cw20_token.set_sender(&sender);

    // Execute CW20 send with insufficient funds
    let err: ContractError = cw20_token
        .execute(
            &andromeda_fungible_tokens::cw20::ExecuteMsg::Send {
                contract: AndrAddr::from_string(
                    env.juno.aos.kernel.address().unwrap().into_string(),
                ),
                amount: Uint128::new(2), // Less than required
                msg: to_json_binary(&Cw20HookMsg::Send { message }).unwrap(),
            },
            &vec![],
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(err, ContractError::InsufficientFunds {});
}

#[rstest]
#[case(1)] // Single recipient
#[case(2)] // Multiple recipients
fn test_fixed_amount_splitter_multiple_recipients(#[case] num_recipients: usize) {
    // Setup environment
    let env = InterchainTestEnv::new();

    // Create recipients
    let recipients: Vec<String> = (0..num_recipients)
        .map(|i| {
            env.juno
                .chain
                .addr_make(format!("recipient{}", i))
                .to_string()
        })
        .collect();

    // Deploy splitter
    let splitter_juno = FixedAmountSplitterContract::new(env.juno.chain.clone());
    splitter_juno.upload().unwrap();

    // Instantiate with multiple recipients
    match num_recipients {
        1 => {
            splitter_juno
                .instantiate(
                    fixed_amount_splitter_instantiate!(env.juno.aos, recipients[0], env.juno.denom),
                    None,
                    &vec![],
                )
                .unwrap();
        }
        2 => {
            splitter_juno
                .instantiate(
                    fixed_amount_splitter_instantiate!(
                        env.juno.aos,
                        [
                            (
                                recipients[0].as_str(),
                                env.juno.denom.as_str(),
                                Uint128::new(100)
                            ),
                            (
                                recipients[1].as_str(),
                                env.juno.denom.as_str(),
                                Uint128::new(200)
                            )
                        ]
                    ),
                    None,
                    &vec![],
                )
                .unwrap();
        }
        _ => panic!(
            "Test case not implemented for {} recipients",
            num_recipients
        ),
    }

    // Register contract
    register_contract!(env.juno.aos, splitter_juno, "fixed-amount-splitter");

    // Execute send
    let message = AMPMsg::new(
        AndrAddr::from_string(splitter_juno.address().unwrap().clone()),
        to_json_binary(
            &andromeda_finance::fixed_amount_splitter::ExecuteMsg::Send { config: None },
        )
        .unwrap(),
        Some(vec![Coin {
            amount: Uint128::new(1000000),
            denom: env.juno.denom.clone(),
        }]),
    );

    env.juno
        .aos
        .kernel
        .execute(
            &os::kernel::ExecuteMsg::Send { message },
            &[Coin {
                amount: Uint128::new(1000000),
                denom: env.juno.denom.clone(),
            }],
        )
        .unwrap();

    // Check balances for all recipients
    for (i, recipient) in recipients.iter().enumerate().take(num_recipients) {
        let balances = env
            .juno
            .chain
            .query_all_balances(&Addr::unchecked(recipient.clone()))
            .unwrap();
        assert_eq!(balances.len(), 1);
        assert_eq!(balances[0].denom, env.juno.denom);

        // First recipient gets 100, second gets 200
        let expected_amount = if i == 0 { 100u128 } else { 200u128 };
        assert_eq!(balances[0].amount.u128(), expected_amount);
    }
}
