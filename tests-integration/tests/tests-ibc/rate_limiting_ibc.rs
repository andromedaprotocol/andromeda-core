#![cfg(not(target_arch = "wasm32"))]

use andromeda_rate_limiting_withdrawals::RateLimitingWithdrawalsContract;
use cosmwasm_std::{to_json_binary, Coin, Uint128};
use cw_orch::mock::MockBase;
use andromeda_std::{
    amp::{messages::AMPMsg, AndrAddr},
    os,
    common::Milliseconds,
};
use andromeda_kernel::ack::make_ack_success;
use andromeda_testing::{
    ado_deployer,
    interchain::{ensure_packet_success, InterchainChain, DEFAULT_SENDER},
    InterchainTestEnv,
};
use cw_orch::prelude::*;
use cw_orch_interchain::prelude::*;
use andromeda_finance::rate_limiting_withdrawals::{InstantiateMsg, ExecuteMsg, CoinAndLimit, MinimumFrequency};


pub struct ChainMap<'a> {
    pub chains: Vec<(&'a InterchainChain, &'a InterchainChain)>,
}

ado_deployer!(
    deploy_rate_limiting,
    InterchainChain,
    RateLimitingWithdrawalsContract<MockBase>,
    &InstantiateMsg
);

#[test]
fn test_rate_limiting_withdrawals_ibc() {
    let InterchainTestEnv {
        juno,
        osmosis,
        andromeda,
        interchain,
        ..
    } = InterchainTestEnv::new();

    let chain_combos = ChainMap {
        chains: vec![
            (&osmosis, &juno),
            (&juno, &osmosis),
            (&andromeda, &juno),
        ],
    };

    for (chain1, chain2) in &chain_combos.chains {
        println!("Running test for chain1: {} and chain2: {} combo", 
                chain1.chain_name, chain2.chain_name);

        let contract = RateLimitingWithdrawalsContract::new(chain1.chain.clone());
        
        let deployed_contract = deploy_rate_limiting!(
            contract,
            chain1,
            &InstantiateMsg {
                allowed_coin: CoinAndLimit {
                    coin: chain1.denom.clone(),
                    limit: Uint128::new(100),
                },
                minimal_withdrawal_frequency: MinimumFrequency::Time {
                    time: Milliseconds::from_seconds(1),
                },
                kernel_address: chain1.aos.kernel.address().unwrap().into_string(),
                owner: None,
            },
            "rate-limiting-withdrawals"
        );

        // Setup recipient address for IBC
        let chain1_recipient = AndrAddr::from_string(format!(
            "ibc://{}/{}",
            chain1.chain_name,
            deployed_contract.address().unwrap()
        ));

        // First withdrawal (should succeed)
        let withdraw_msg = AMPMsg::new(
            chain1_recipient.clone(),
            to_json_binary(&ExecuteMsg::Withdraw {
                amount: Uint128::new(50),
            }).unwrap(),
            None
        );

        // Execute withdrawal from chain2
        let chain2_send_request = chain2
            .aos
            .kernel
            .execute(
                &os::kernel::ExecuteMsg::Send { message: withdraw_msg },
                Some(&[Coin {
                    denom: chain1.denom.clone(),
                    amount: Uint128::new(50),
                }]),
            )
            .unwrap();

        let packet_lifetime = interchain
            .await_packets(&chain2.chain_name, chain2_send_request)
            .unwrap();
        ensure_packet_success(packet_lifetime);

        // Setup trigger
        chain2.aos
            .kernel
            .execute(
                &os::kernel::ExecuteMsg::UpsertKeyAddress {
                    key: "trigger_key".to_string(),
                    value: DEFAULT_SENDER.to_string(),
                },
                None,
            )
            .unwrap();

        let packet_ack = make_ack_success();
        let channel_id = chain2.aos.get_aos_channel(chain1.chain_name.clone()).unwrap().ics20.unwrap();
        
        // Execute withdrawal relay
        let kernel_chain2_withdraw = chain2
            .aos
            .kernel
            .execute(
                &os::kernel::ExecuteMsg::TriggerRelay {
                    packet_sequence: 1,
                    packet_ack,
                    channel_id,
                },
                None,
            )
            .unwrap();

        let packet_lifetime = interchain
            .await_packets(&chain2.chain_name, kernel_chain2_withdraw)
            .unwrap();
        ensure_packet_success(packet_lifetime);

        // Verify withdrawal
        let ibc_denom = format!(
            "ibc/{}/{}",
            chain1.aos.get_aos_channel(&chain2.chain_name).unwrap().direct.unwrap(),
            chain2.chain_name.clone()
        );

        let balance = chain1.chain.query_all_balances(chain1.addresses[0].clone()).unwrap();
        assert_eq!(balance[0].denom, ibc_denom);
        assert_eq!(balance[0].amount, Uint128::new(50));

        // Second withdrawal attempt (should fail due to rate limit)
        let exceed_msg = AMPMsg::new(
            chain1_recipient,
            to_json_binary(&ExecuteMsg::Withdraw {
                amount: Uint128::new(60),
            }).unwrap(),
            Some(vec![Coin {
                denom: chain1.denom.clone(),
                amount: Uint128::new(60),
            }]),
        );

        // This should fail due to rate limiting
        let result = chain2
            .aos
            .kernel
            .execute(
                &os::kernel::ExecuteMsg::Send { message: exceed_msg },
                Some(&[Coin {
                    denom: chain1.denom.clone(),
                    amount: Uint128::new(60),
                }]),
            );

        assert!(result.is_err());
    }
}