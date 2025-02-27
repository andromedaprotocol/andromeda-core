use andromeda_finance::splitter::AddressPercent;
use andromeda_fixed_amount_splitter::FixedAmountSplitterContract;
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

use andromeda_std::{amp::messages::AMPMsg, os};
use andromeda_testing::{ado_deployer, InterchainTestEnv};
use cosmwasm_std::{to_json_binary, Coin, Uint128};
use cw_orch::prelude::*;
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

#[test]
fn test_fixed_amount_splitter_local_with_message_and_funds() {
    let InterchainTestEnv { juno, .. } = InterchainTestEnv::new();

    let recipient = juno.chain.addr_make("recipient");

    // Deploy on Osmosis
    let splitter_juno = FixedAmountSplitterContract::new(juno.chain.clone());
    splitter_juno.upload().unwrap();

    splitter_juno
        .instantiate(
            &andromeda_finance::fixed_amount_splitter::InstantiateMsg {
                recipients: vec![andromeda_finance::fixed_amount_splitter::AddressAmount {
                    recipient: Recipient {
                        address: AndrAddr::from_string(recipient.clone()),
                        msg: None,
                        ibc_recovery_address: None,
                    },
                    coins: vec![Coin {
                        denom: juno.denom.clone(),
                        amount: Uint128::new(100),
                    }],
                }],
                default_recipient: None,
                lock_time: None,
                kernel_address: juno.aos.kernel.address().unwrap().into_string(),
                owner: None,
            },
            None,
            None,
        )
        .unwrap();

    // Register contract
    juno.aos
        .adodb
        .execute(
            &os::adodb::ExecuteMsg::Publish {
                code_id: splitter_juno.code_id().unwrap(),
                ado_type: "fixed-amount-splitter".to_string(),
                action_fees: None,
                version: "1.0.0".to_string(),
                publisher: None,
            },
            None,
        )
        .unwrap();

    let message = AMPMsg::new(
        AndrAddr::from_string(splitter_juno.address().unwrap().clone()),
        to_json_binary(
            &andromeda_finance::fixed_amount_splitter::ExecuteMsg::Send { config: None },
        )
        .unwrap(),
        Some(vec![Coin {
            amount: Uint128::new(100000000),
            denom: juno.denom.clone(),
        }]),
    );

    // Execute IBC transfer from Juno
    juno.aos
        .kernel
        .execute(
            &os::kernel::ExecuteMsg::Send { message },
            Some(&[Coin {
                amount: Uint128::new(100000000),
                denom: juno.denom.clone(),
            }]),
        )
        .unwrap();

    // Check balances
    let balances = juno.chain.query_all_balances(&recipient).unwrap();
    assert_eq!(balances.len(), 1);
    assert_eq!(balances[0].denom, juno.denom);
    assert_eq!(balances[0].amount.u128(), 100);
}

#[test]
fn test_fixed_amount_splitter_local_with_no_message() {
    let InterchainTestEnv { juno, .. } = InterchainTestEnv::new();

    let recipient = juno.chain.addr_make("recipient");

    // Deploy on Osmosis
    let splitter_juno = FixedAmountSplitterContract::new(juno.chain.clone());
    splitter_juno.upload().unwrap();

    splitter_juno
        .instantiate(
            &andromeda_finance::fixed_amount_splitter::InstantiateMsg {
                recipients: vec![andromeda_finance::fixed_amount_splitter::AddressAmount {
                    recipient: Recipient {
                        address: AndrAddr::from_string(recipient.clone()),
                        msg: None,
                        ibc_recovery_address: None,
                    },
                    coins: vec![Coin {
                        denom: juno.denom.clone(),
                        amount: Uint128::new(100),
                    }],
                }],
                default_recipient: None,
                lock_time: None,
                kernel_address: juno.aos.kernel.address().unwrap().into_string(),
                owner: None,
            },
            None,
            None,
        )
        .unwrap();

    // Register contract
    juno.aos
        .adodb
        .execute(
            &os::adodb::ExecuteMsg::Publish {
                code_id: splitter_juno.code_id().unwrap(),
                ado_type: "fixed-amount-splitter".to_string(),
                action_fees: None,
                version: "1.0.0".to_string(),
                publisher: None,
            },
            None,
        )
        .unwrap();

    let message = AMPMsg::new(
        AndrAddr::from_string(splitter_juno.address().unwrap().clone()),
        Binary::default(), // No message
        Some(vec![Coin {
            amount: Uint128::new(100000000),
            denom: juno.denom.clone(),
        }]),
    );

    // Execute IBC transfer from Juno
    juno.aos
        .kernel
        .execute(
            &os::kernel::ExecuteMsg::Send { message },
            Some(&[Coin {
                amount: Uint128::new(100000000),
                denom: juno.denom.clone(),
            }]),
        )
        .unwrap();

    // Check balances
    let balances = juno
        .chain
        .query_all_balances(&splitter_juno.address().unwrap())
        .unwrap();
    assert_eq!(balances.len(), 1);
    assert_eq!(balances[0].denom, juno.denom);
    assert_eq!(balances[0].amount.u128(), 100000000);
}

#[test]
fn test_fixed_amount_splitter_local_funds_mismatch() {
    let InterchainTestEnv { juno, .. } = InterchainTestEnv::new();

    let recipient = juno.chain.addr_make("recipient");

    // Deploy on Osmosis
    let splitter_juno = FixedAmountSplitterContract::new(juno.chain.clone());
    splitter_juno.upload().unwrap();

    splitter_juno
        .instantiate(
            &andromeda_finance::fixed_amount_splitter::InstantiateMsg {
                recipients: vec![andromeda_finance::fixed_amount_splitter::AddressAmount {
                    recipient: Recipient {
                        address: AndrAddr::from_string(recipient.clone()),
                        msg: None,
                        ibc_recovery_address: None,
                    },
                    coins: vec![Coin {
                        denom: juno.denom.clone(),
                        amount: Uint128::new(100),
                    }],
                }],
                default_recipient: None,
                lock_time: None,
                kernel_address: juno.aos.kernel.address().unwrap().into_string(),
                owner: None,
            },
            None,
            None,
        )
        .unwrap();

    // Register contract
    juno.aos
        .adodb
        .execute(
            &os::adodb::ExecuteMsg::Publish {
                code_id: splitter_juno.code_id().unwrap(),
                ado_type: "fixed-amount-splitter".to_string(),
                action_fees: None,
                version: "1.0.0".to_string(),
                publisher: None,
            },
            None,
        )
        .unwrap();

    let message = AMPMsg::new(
        AndrAddr::from_string(splitter_juno.address().unwrap().clone()),
        Binary::default(), // No message
        Some(vec![Coin {
            amount: Uint128::new(100000000),
            denom: juno.denom.clone(),
        }]),
    );

    // Execute IBC transfer from Juno
    let err: ContractError = juno
        .aos
        .kernel
        .execute(
            &os::kernel::ExecuteMsg::Send { message },
            Some(&[Coin {
                amount: Uint128::new(2),
                denom: juno.denom.clone(),
            }]),
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::InsufficientFunds {});
}
