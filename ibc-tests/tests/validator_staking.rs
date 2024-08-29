use andromeda_std::ado_base::MigrateMsg;
use cosmwasm_std::Uint128;
use cw_orch::environment::ChainKind;
use cw_orch::environment::NetworkInfo;
use cw_orch::interface;
use cw_orch::prelude::*;
use cw_orch_daemon::queriers::Staking;
use cw_orch_daemon::queriers::StakingBondStatus;
use cw_orch_daemon::Daemon;
use ibc_tests::contract_interface;

// import messages
use andromeda_app::app;
use andromeda_finance::validator_staking;

// const TESTNET_MNEMONIC: &str = "increase bread alpha rigid glide amused approve oblige print asset idea enact lawn proof unfold jeans rabbit audit return chuckle valve rather cactus great";
const TESTNET_MNEMONIC: &str = "across left ignore gold echo argue track joy hire release captain enforce hotel wide flash hotel brisk joke midnight duck spare drop chronic stool";
// osmo1jdpunqljj5xypxk6f7dnpga6cjfatwu6jqv0jw

// define app contract interface
contract_interface!(
    AppContract,
    andromeda_app_contract,
    app,
    "andromeda_app_contract",
    "app_contract"
);

// include ados be tested
contract_interface!(
    ValidatorStakingContract,
    andromeda_validator_staking,
    validator_staking,
    "validator_staking_contract",
    "validator_staking"
);

pub const TERRA_NETWORK: NetworkInfo = NetworkInfo {
    chain_name: "terra",
    pub_address_prefix: "terra",
    coin_type: 330u32,
};

pub const LOCAL_TERRA: ChainInfo = ChainInfo {
    kind: ChainKind::Local,
    chain_id: "localterraa-1",
    gas_denom: "uluna",
    gas_price: 0.15,
    grpc_urls: &["http://localhost:20331"],
    network_info: TERRA_NETWORK,
    lcd_url: None,
    fcd_url: None,
};

#[test]
fn test_validator_staking() {
    // Pause validator before running this test
    let local_terra = LOCAL_TERRA;
    let daemon = Daemon::builder(local_terra.clone()) // set the network to use
        .mnemonic(TESTNET_MNEMONIC)
        .build()
        .unwrap();
    let denom = local_terra.gas_denom;

    let validator_staking_contract = ValidatorStakingContract::new(daemon.clone());
    validator_staking_contract.set_address(&Addr::unchecked(
        "terra1vk603ncakghk33t8lklvpdq4aff03hwu2rak73f5zdayruead20qcwp0rf",
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
    println!(
        "================================default validator: {:?}================================",
        default_validator
    );
    let staking_query_msg = validator_staking::QueryMsg::StakedTokens {
        validator: Some(Addr::unchecked(default_validator.address.to_string())),
    };
    let rewards_response: Option<cosmwasm_std::FullDelegation> = validator_staking_contract
        .query(&staking_query_msg)
        .unwrap();
    println!(
        "================================rewards_response: {:?}================================",
        rewards_response
    );

    let contract_balance = daemon
        .balance(
            validator_staking_contract.addr_str().unwrap(),
            Some(denom.to_string()),
        )
        .unwrap()[0]
        .amount;
    assert_eq!(contract_balance, Uint128::zero());

    // let user_balance = daemon
    //     .balance(daemon.sender_addr(), Some(denom.to_string()))
    //     .unwrap()[0].amount;

    let claim_msg = validator_staking::ExecuteMsg::Claim {
        validator: Some(Addr::unchecked(default_validator.address.to_string())),
        recipient: None,
    };
    validator_staking_contract
        .execute(&claim_msg, None)
        .unwrap();

    // let contract_balance = daemon
    //     .balance(validator_staking_contract.addr_str().unwrap(), Some(denom.to_string()))
    //     .unwrap()[0].amount;
    // assert_eq!(contract_balance, Uint128::zero());

    // let updated_user_balance = daemon
    //     .balance(daemon.sender_addr(), Some(denom.to_string()))
    //     .unwrap()[0].amount;

    // assert_eq!(user_balance + rewards_response.unwrap().accumulated_rewards[0].amount, updated_user_balance);

    let unstake_msg = validator_staking::ExecuteMsg::Unstake {
        validator: Some(Addr::unchecked(default_validator.address.to_string())),
        amount: None,
    };
    validator_staking_contract
        .execute(&unstake_msg, None)
        .unwrap();
    let contract_balance = daemon
        .balance(
            validator_staking_contract.addr_str().unwrap(),
            Some(denom.to_string()),
        )
        .unwrap()[0]
        .amount;
    assert_eq!(contract_balance, Uint128::zero());

    daemon.wait_seconds(61).unwrap();
    // let user_balance = daemon
    //     .balance(daemon.sender_addr(), Some(denom.to_string()))
    //     .unwrap()[0].amount;

    let withdraw_msg = validator_staking::ExecuteMsg::WithdrawFunds {
        denom: Some(denom.to_string()),
        recipient: None,
    };
    validator_staking_contract
        .execute(&withdraw_msg, None)
        .unwrap();
    let updated_user_balance = daemon
        .balance(daemon.sender_addr(), Some(denom.to_string()))
        .unwrap()[0]
        .amount;
    println!(
        "============================{:?}============================",
        updated_user_balance
    );
    // assert_eq!(user_balance + Uint128::from(10000000000u128), updated_user_balance);
}

#[test]
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
        "terra1373lhzrt3nqqemvz7gs50nlfu3ckphgxv9vlepaqs22vjxxz7guqwnmjz8",
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
    println!(
        "================================kicked validator: {:?}================================",
        kicked_validator
    );
    let staking_query_msg = validator_staking::QueryMsg::StakedTokens {
        validator: Some(Addr::unchecked(kicked_validator.address.to_string())),
    };
    let rewards_response: Option<cosmwasm_std::FullDelegation> = validator_staking_contract
        .query(&staking_query_msg)
        .unwrap();
    println!(
        "================================rewards_response: {:?}================================",
        rewards_response
    );

    let contract_balance = daemon
        .balance(
            validator_staking_contract.addr_str().unwrap(),
            Some(denom.to_string()),
        )
        .unwrap()[0]
        .amount;
    assert_eq!(contract_balance, Uint128::zero());

    // let user_balance = daemon
    //     .balance(daemon.sender_addr(), Some(denom.to_string()))
    //     .unwrap()[0].amount;

    let claim_msg = validator_staking::ExecuteMsg::Claim {
        validator: Some(Addr::unchecked(kicked_validator.address.to_string())),
        recipient: None,
    };
    validator_staking_contract
        .execute(&claim_msg, None)
        .unwrap();

    // let contract_balance = daemon
    //     .balance(validator_staking_contract.addr_str().unwrap(), Some(denom.to_string()))
    //     .unwrap()[0].amount;
    // assert_eq!(contract_balance, Uint128::zero());

    // let updated_user_balance = daemon
    //     .balance(daemon.sender_addr(), Some(denom.to_string()))
    //     .unwrap()[0].amount;

    // assert_eq!(user_balance + rewards_response.unwrap().accumulated_rewards[0].amount, updated_user_balance);

    let unstake_msg = validator_staking::ExecuteMsg::Unstake {
        validator: Some(Addr::unchecked(kicked_validator.address.to_string())),
        amount: None,
    };
    validator_staking_contract
        .execute(&unstake_msg, None)
        .unwrap();
    let contract_balance = daemon
        .balance(
            validator_staking_contract.addr_str().unwrap(),
            Some(denom.to_string()),
        )
        .unwrap()[0]
        .amount;
    assert_eq!(contract_balance, Uint128::zero());

    daemon.wait_seconds(61).unwrap();
    // let user_balance = daemon
    //     .balance(daemon.sender_addr(), Some(denom.to_string()))
    //     .unwrap()[0].amount;

    let withdraw_msg = validator_staking::ExecuteMsg::WithdrawFunds {
        denom: Some(denom.to_string()),
        recipient: None,
    };
    validator_staking_contract
        .execute(&withdraw_msg, None)
        .unwrap();
    let updated_user_balance = daemon
        .balance(daemon.sender_addr(), Some(denom.to_string()))
        .unwrap()[0]
        .amount;
    println!(
        "============================{:?}============================",
        updated_user_balance
    );
    // assert_eq!(user_balance + Uint128::from(10000000000u128), updated_user_balance);
}
