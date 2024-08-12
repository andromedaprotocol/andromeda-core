use std::cmp;

use andromeda_app::app::AppComponent;
use andromeda_std::ado_base::MigrateMsg;
use andromeda_testing_e2e::mock::mock_app;
use andromeda_testing_e2e::mock::MockAndromeda;
use cosmwasm_std::coin;
use cosmwasm_std::to_json_binary;
use cosmwasm_std::Uint128;
use cw_orch::interface;
use cw_orch::prelude::*;
use cw_orch_daemon::{networks};
use ibc_tests::contract_interface;

// import messages
use andromeda_app::app;
use andromeda_finance::validator_staking;

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

#[test]
fn test_validator_staking() {
    let mut local_osmo = networks::LOCAL_OSMO;
    local_osmo.chain_id = "osmosis-1";
    local_osmo.grpc_urls = &["http://localhost:9091"];

    let chain = mock_app(local_osmo.clone(), TESTNET_MNEMONIC);
    let MockAndromeda {kernel_contract, adodb_contract, ..} = MockAndromeda::new(&chain);

    let app_contract = AppContract::new(chain.clone());
    app_contract.upload().unwrap();

    // 6. upload validator staking contract
    let validator_staking_contract = ValidatorStakingContract::new(chain.clone());
    validator_staking_contract.upload().unwrap();

    // publish app contract and validator staking contract
    adodb_contract.clone().execute_publish(
        app_contract.code_id().unwrap(),
        "app-contract".to_string(),
        "0.1.0".to_string()
    );
    adodb_contract.clone().execute_publish(
        validator_staking_contract.code_id().unwrap(),
        "validator-staking".to_string(),
        "0.1.0".to_string()
    );
    // ================================= Create App with modules ================================= //
    let validator_staking_init_msg = validator_staking::InstantiateMsg {
        default_validator: Addr::unchecked("osmovaloper1qjtcxl86z0zua2egcsz4ncff2gzlcndzs93m43"), // genesis validator
        kernel_address: kernel_contract.addr_str().unwrap(),
        owner: None,
    };

    let validator_staking_component = AppComponent::new(
        "validator-staking-component",
        "validator-staking",
        to_json_binary(&validator_staking_init_msg).unwrap(),
    );

    let app_components = vec![validator_staking_component.clone()];
    let app_init_msg = app::InstantiateMsg {
        app_components,
        kernel_address: kernel_contract.addr_str().unwrap(),
        name: "Validator Staking App".to_string(),
        owner: None,
        chain_info: None,
    };
    app_contract.instantiate(&app_init_msg, None, None).unwrap();

    let get_addr_message = app::QueryMsg::GetAddress {
        name: validator_staking_component.name,
    };

    let validator_staking_addr: String = chain
        .wasm_querier()
        .smart_query(app_contract.addr_str().unwrap(), &get_addr_message)
        .unwrap();

    validator_staking_contract.set_address(&Addr::unchecked(validator_staking_addr));

    // stake
    let stake_msg = validator_staking::ExecuteMsg::Stake { validator: None };
    let balance = chain
        .balance(chain.sender_addr(), Some(local_osmo.gas_denom.to_string()))
        .unwrap();
    let amount_to_send = cmp::min(balance[0].amount, Uint128::new(10000));
    let resp = validator_staking_contract
        .execute(
            &stake_msg,
            Some(&[coin(amount_to_send.u128(), local_osmo.gas_denom)]),
        )
        .unwrap();

    let staking_query_msg = validator_staking::QueryMsg::StakedTokens { validator: None };
    let res: Option<cosmwasm_std::FullDelegation> = validator_staking_contract
        .query(&staking_query_msg)
        .unwrap();
    println!(
        "======================staking query msg result: {:?}======================",
        res
    );
}
