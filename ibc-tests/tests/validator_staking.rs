use andromeda_finance::validator_staking::ExecuteMsgFns;
use andromeda_validator_staking::ValidatorStakingContract;
use cosmwasm_std::Uint128;
use cw_orch::prelude::*;
use cw_orch_daemon::queriers::Staking;
use cw_orch_daemon::queriers::StakingBondStatus;
use cw_orch_daemon::Daemon;
use ibc_tests::constants::LOCAL_TERRA;

const TESTNET_MNEMONIC: &str = "across left ignore gold echo argue track joy hire release captain enforce hotel wide flash hotel brisk joke midnight duck spare drop chronic stool";

#[test]
#[ignore]
fn test_validator_staking() {
    let local_terra = LOCAL_TERRA;
    let daemon = Daemon::builder(local_terra.clone()) // set the network to use
        .mnemonic(TESTNET_MNEMONIC)
        .build()
        .unwrap();
    let denom = local_terra.gas_denom;

    let validator_staking_contract = ValidatorStakingContract::new(daemon.clone());
    validator_staking_contract.set_address(&Addr::unchecked(
        "terra18cv7jca4dnsu8vuhu2t7fkwl23dxres8kpnhggdarf7f0dh4j4ysv3qhd7",
    ));

    let staking_querier = Staking::new(&daemon);
    let mut validators = daemon
        .rt_handle
        .block_on(async { staking_querier._validators(StakingBondStatus::Bonded).await })
        .unwrap();

    while validators.is_empty() {
        println!("================================waiting till bonded validators found================================");
        daemon.wait_seconds(10).unwrap();

        validators = daemon
            .rt_handle
            .block_on(async { staking_querier._validators(StakingBondStatus::Bonded).await })
            .unwrap();
    }

    let default_validator = &validators[0];

    let _rewards_response: Option<cosmwasm_std::FullDelegation> = validator_staking_contract
        .staked_tokens(Some(Addr::unchecked(default_validator.address.to_string())));

    validator_staking_contract
        .claim(
            None,
            Some(Addr::unchecked(default_validator.address.to_string())),
        )
        .unwrap();

    validator_staking_contract
        .unstake(
            None,
            Some(Addr::unchecked(default_validator.address.to_string())),
        )
        .unwrap();

    daemon.wait_seconds(61).unwrap();

    validator_staking_contract
        .withdraw_funds(Some(denom.to_string()), None)
        .unwrap();

    let contract_balance = daemon
        .balance(
            validator_staking_contract.addr_str().unwrap(),
            Some(denom.to_string()),
        )
        .unwrap()[0]
        .amount;
    assert_eq!(contract_balance, Uint128::zero());
}

#[test]
#[ignore]
fn test_kicked_validator() {
    // Pause validator before running this test
    let local_terra = LOCAL_TERRA;
    let daemon = Daemon::builder(local_terra.clone()) // set the network to use
        .mnemonic(TESTNET_MNEMONIC)
        .build()
        .unwrap();
    let denom = local_terra.gas_denom;

    let validator_staking_contract = ValidatorStakingContract::new(daemon.clone());
    validator_staking_contract.set_address(&Addr::unchecked(
        "terra18cv7jca4dnsu8vuhu2t7fkwl23dxres8kpnhggdarf7f0dh4j4ysv3qhd7",
    ));

    let staking_querier = Staking::new(&daemon);
    let mut kicked_validators = daemon
        .rt_handle
        .block_on(async {
            staking_querier
                ._validators(StakingBondStatus::Unbonded)
                .await
        })
        .unwrap();

    while kicked_validators.is_empty() {
        println!("================================waiting till one validator is kicked================================");
        daemon.wait_seconds(10).unwrap();

        kicked_validators = daemon
            .rt_handle
            .block_on(async {
                staking_querier
                    ._validators(StakingBondStatus::Unbonded)
                    .await
            })
            .unwrap();
    }

    let kicked_validator = &kicked_validators[0];

    let contract_balance = daemon
        .balance(
            validator_staking_contract.addr_str().unwrap(),
            Some(denom.to_string()),
        )
        .unwrap()[0]
        .amount;
    assert_eq!(contract_balance, Uint128::zero());

    validator_staking_contract
        .claim(
            None,
            Some(Addr::unchecked(kicked_validator.address.to_string())),
        )
        .unwrap();

    validator_staking_contract
        .unstake(
            None,
            Some(Addr::unchecked(kicked_validator.address.to_string())),
        )
        .unwrap();

    daemon.wait_seconds(61).unwrap();

    validator_staking_contract
        .withdraw_funds(Some(denom.to_string()), None)
        .unwrap();

    let contract_balance = daemon
        .balance(
            validator_staking_contract.addr_str().unwrap(),
            Some(denom.to_string()),
        )
        .unwrap()[0]
        .amount;
    assert_eq!(contract_balance, Uint128::zero());
}
