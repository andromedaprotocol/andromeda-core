use std::cmp;

use andromeda_app_contract::AppContract;
use andromeda_std::os::adodb::ExecuteMsgFns as AdodbExecuteMsgFns;
use andromeda_testing_e2e::mock::{mock_app, MockAndromeda};
use andromeda_validator_staking::ValidatorStakingContract;
use cosmwasm_std::{coin, to_json_binary, Uint128};
use cw_orch::prelude::*;
use cw_orch::{
    environment::{ChainKind, NetworkInfo},
    prelude::ChainInfo,
};
use cw_orch_daemon::{
    queriers::{Staking, StakingBondStatus},
    DaemonBase, Wallet,
};

use andromeda_app::app::{self, AppComponent};
use andromeda_finance::validator_staking::{self, ExecuteMsgFns as ValidatorStakingExecuteMsgFns};

const TESTNET_MNEMONIC: &str = "across left ignore gold echo argue track joy hire release captain enforce hotel wide flash hotel brisk joke midnight duck spare drop chronic stool";
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

fn main() {
    println!("//=============================Prereparing test environment===================================//");

    let local_terra = LOCAL_TERRA;
    println!("//===============================Prereparing Andromeda OS=====================================//");
    let daemon = mock_app(local_terra.clone(), TESTNET_MNEMONIC);
    let mock_andromeda = MockAndromeda::new(&daemon);
    let MockAndromeda {
        kernel_contract,
        adodb_contract,
        ..
    } = &mock_andromeda;

    println!("//================================Andromeda OS Setup Completed================================//");
    println!(
        "kernel_contract->code_id:  {:?}, kernel_contract-> address  {:?}",
        kernel_contract.code_id(),
        kernel_contract.addr_str()
    );
    println!(
        "adodb_contract->code_id:  {:?}, adodb_contract-> address  {:?}",
        adodb_contract.code_id(),
        adodb_contract.addr_str()
    );
    println!("//============================================================================================//");

    println!("Preparing App component");
    println!("//=======================================Preparing App=======================================//");
    let app_contract = AppContract::new(daemon.clone());
    app_contract.upload().unwrap();

    adodb_contract
        .clone()
        .publish(
            "app-contract".to_string(),
            app_contract.code_id().unwrap(),
            "0.1.0".to_string(),
            None,
            None,
        )
        .unwrap();
    println!("app_contract->code_id:  {:?}", app_contract.code_id());
    println!("//==============================Base Test Environment Ready=================================//");

    prepare_validator_staking(&daemon, &mock_andromeda, &app_contract);
}

fn prepare_validator_staking(
    daemon: &DaemonBase<Wallet>,
    mock_andromeda: &MockAndromeda,
    app_contract: &AppContract<DaemonBase<Wallet>>,
) {
    let denom = LOCAL_TERRA.gas_denom;

    println!("//===============================Preparing Validator Staking=================================//");
    let validator_staking_contract = ValidatorStakingContract::new(daemon.clone());
    validator_staking_contract.upload().unwrap();

    let MockAndromeda {
        kernel_contract,
        adodb_contract,
        ..
    } = mock_andromeda;
    adodb_contract
        .clone()
        .publish(
            "validator-staking".to_string(),
            validator_staking_contract.code_id().unwrap(),
            "0.1.0".to_string(),
            None,
            None,
        )
        .unwrap();

    println!("//================================Validator Staking Prepared-=================================//");
    println!(
        "validator_staking_contract->code_id:  {:?}",
        validator_staking_contract.code_id()
    );
    println!("//============================================================================================//");

    println!("//===========================Initialize app with Validator Staking============================//");
    let staking_querier = Staking::new(daemon);
    let validators = daemon
        .rt_handle
        .block_on(async { staking_querier._validators(StakingBondStatus::Bonded).await })
        .unwrap();

    if validators.len() < 5 {
        println!("At least 5 validators are required for this test");
        return;
    }

    let validator_staking_init_msg = validator_staking::InstantiateMsg {
        default_validator: Addr::unchecked(&validators[0].address), // fourth validator
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

    let validator_staking_addr = app_contract.get_address(validator_staking_component.name);

    validator_staking_contract.set_address(&Addr::unchecked(validator_staking_addr));

    println!("//==========================App with Validator Staking Initialized============================//");
    println!(
        "app_contract->code_id:  {:?}, app_contract-> address:  {:?}",
        app_contract.code_id(),
        app_contract.addr_str()
    );
    println!(
        "validator_staking_contract->code_id:  {:?}, validator_staking_contract-> address  {:?}",
        validator_staking_contract.code_id(),
        validator_staking_contract.addr_str()
    );
    println!("//============================================================================================//");

    println!("//===============================Processing Stake For testing=================================//");
    validators.into_iter().for_each(|validator| {
        let balance = daemon
            .balance(daemon.sender_addr(), Some(denom.to_string()))
            .unwrap();
        let amount_to_send = cmp::min(balance[0].amount, Uint128::new(10000000000));
        validator_staking_contract
            .stake(
                Some(Addr::unchecked(validator.address.to_string())),
                &[coin(amount_to_send.u128(), denom)],
            )
            .unwrap();

        println!(
            "validator: {:?}, delegator: {:?}",
            validator.address,
            daemon.sender_addr()
        );
    });
    println!("//============================================================================================//");
}
