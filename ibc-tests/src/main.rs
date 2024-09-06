use std::cmp;

use andromeda_testing_e2e::chains::ALL_CHAINS;

use andromeda_std::ado_base::MigrateMsg;
use andromeda_testing_e2e::chains::{LOCAL_OSMO, TESTNET_MNEMONIC};
use andromeda_testing_e2e::mock::MockAndromeda;
use cosmwasm_std::{coin, to_json_binary, Uint128};
use cw_orch::interface;
use cw_orch::prelude::*;
use cw_orch_interchain::prelude::*;

use cw_orch_daemon::{
    queriers::{Staking, StakingBondStatus},
    DaemonBase, Wallet,
};
use ibc_tests::config::Config;
use ibc_tests::contract_interface;

use andromeda_app::app::{self, AppComponent};
use andromeda_finance::validator_staking;

contract_interface!(
    AppContract,
    andromeda_app_contract,
    app,
    "andromeda_app_contract",
    "andromeda_app_contract@1.1.1"
);

fn install_os(daemon: &DaemonBase<Wallet>) -> Addr {
    let mock_andromeda = MockAndromeda::install(daemon);
    let MockAndromeda { adodb_contract, .. } = &mock_andromeda;

    let app_contract = AppContract::new(daemon.clone());
    app_contract.upload().unwrap();

    adodb_contract.clone().execute_publish(
        app_contract.code_id().unwrap(),
        "app-contract".to_string(),
        "0.1.0".to_string(),
    );

    mock_andromeda.kernel_contract.address().unwrap()
}

fn main() {
    env_logger::init();
    let interchain_info: Vec<(ChainInfo, Option<String>)> = ALL_CHAINS
        .iter()
        .map(|chain| (chain.clone(), Some(TESTNET_MNEMONIC.to_string())))
        .collect();
    let interchain = DaemonInterchainEnv::new(interchain_info, &ChannelCreationValidator).unwrap();
    let mut config: Config = Config::default();
    for chain in ALL_CHAINS {
        let daemon = interchain.get_chain(chain.chain_id.to_string()).unwrap();
        let kernel_address = install_os(&daemon);
        config
            .installations
            .insert(chain.network_info.chain_name.to_string(), kernel_address);
    }
    println!("Installed OS on all chains");

    config.save();
}

contract_interface!(
    ValidatorStakingContract,
    andromeda_validator_staking,
    validator_staking,
    "validator_staking_contract",
    "validator_staking"
);

fn prepare_validator_staking(
    daemon: &DaemonBase<Wallet>,
    mock_andromeda: &MockAndromeda,
    app_contract: &AppContract<DaemonBase<Wallet>>,
) {
    let denom = LOCAL_OSMO.gas_denom;

    println!("//===============================Preparing Validator Staking=================================//");
    let validator_staking_contract = ValidatorStakingContract::new(daemon.clone());
    validator_staking_contract.upload().unwrap();

    let MockAndromeda {
        kernel_contract,
        adodb_contract,
        ..
    } = mock_andromeda;
    adodb_contract.clone().execute_publish(
        validator_staking_contract.code_id().unwrap(),
        "validator-staking".to_string(),
        "0.1.0".to_string(),
    );

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

    let get_addr_message = app::QueryMsg::GetAddress {
        name: validator_staking_component.name,
    };

    let validator_staking_addr: String = daemon
        .wasm_querier()
        .smart_query(app_contract.addr_str().unwrap(), &get_addr_message)
        .unwrap();

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
        let stake_msg = validator_staking::ExecuteMsg::Stake {
            validator: Some(Addr::unchecked(validator.address.to_string())),
        };
        let balance = daemon
            .balance(&daemon.sender_addr(), Some(denom.to_string()))
            .unwrap();
        let amount_to_send = cmp::min(balance[0].amount, Uint128::new(10000000000));
        validator_staking_contract
            .execute(&stake_msg, Some(&[coin(amount_to_send.u128(), denom)]))
            .unwrap();
        println!(
            "validator: {:?}, delegator: {:?}",
            validator.address,
            daemon.sender_addr()
        );
    });
    println!("//============================================================================================//");
}
