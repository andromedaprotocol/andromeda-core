use std::cmp;
use std::collections::HashMap;

use andromeda_app::app::AppComponent;
use andromeda_std::ado_base::MigrateMsg;
use andromeda_testing::mock::mock_app;
use cosmwasm_std::coin;
use cosmwasm_std::to_json_binary;
use cosmwasm_std::Uint128;
use cw_orch::interface;
use cw_orch::prelude::*;
use cw_orch_daemon::{networks, Daemon};
use ibc_tests::contract_interface;
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;

// import messages
use andromeda_app::app;
use andromeda_finance::validator_staking;
use andromeda_std::os::adodb;
use andromeda_std::os::economics;
use andromeda_std::os::kernel;
use andromeda_std::os::vfs;

pub const CONTRACT_ID: &str = "validator_staking_contract";
const GENESIS_VALIDATOR_MNEMONIC: &str = "razor dog gown public private couple ecology paper flee connect local robot diamond stay rude join sound win ribbon soup kidney glass robot vehicle";
// const GENESIS_VALIDATOR_MNEMONIC: &str = "issue have volume expire shoe year finish poem alien urban license undo rural endless food host opera fix forum crack wide example firm learn";
// razor dog gown public private couple ecology paper flee connect local robot diamond stay rude join sound win ribbon soup kidney glass robot vehicle
// osmo1qjtcxl86z0zua2egcsz4ncff2gzlcndz2jeczk
// OSMO1C3DDFF822002C6BE6A61AB51230A66272DF6BCEF
const TESTNET_MNEMONIC: &str = "across left ignore gold echo argue track joy hire release captain enforce hotel wide flash hotel brisk joke midnight duck spare drop chronic stool";
// osmo1jdpunqljj5xypxk6f7dnpga6cjfatwu6jqv0jw

// define kernel contract interface
contract_interface!(
    KernelContract,
    andromeda_kernel,
    kernel,
    "kernel_contract",
    "kernel"
);

// define adodb contract interface
contract_interface!(
    AdodbContract,
    andromeda_adodb,
    adodb,
    "adodb_contract",
    "adodb"
);

// define vfs contract interface
contract_interface!(VfsContract, andromeda_vfs, vfs, "vfs_contract", "vfs");

// define economics contract interface
contract_interface!(
    EconomicsContract,
    andromeda_economics,
    economics,
    "economics_contract",
    "economics"
);

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

#[derive(Serialize, Deserialize, Debug)]
pub struct AirdropRequest {
    /// Address of the address asking for funds
    pub address: String,
    /// Denom asked for
    pub denom: String,
}
async fn airdrop(addr: String, chain: &ChainInfo) {
    let client = reqwest::Client::new();
    let url = chain.fcd_url.unwrap_or("http://localhost:8001");
    let url = format!("{}/credit", url);
    client
        .post(url)
        .json(&AirdropRequest {
            address: addr.to_string(),
            denom: chain.gas_denom.to_string(),
        })
        .send()
        .await
        .unwrap();
}

#[test]
fn test_validator_staking() {
    let mut local_osmo = networks::LOCAL_OSMO;
    local_osmo.chain_id = "osmosis-1";
    local_osmo.grpc_urls = &["http://localhost:9091"];

    let chain = mock_app(local_osmo.clone(), TESTNET_MNEMONIC);
    // ================================= Setup OS ================================= //
    // 1. instantiate kernel contract
    let kernel_contract = KernelContract::new(chain.clone());
    kernel_contract.upload().unwrap();
    let msg = kernel::InstantiateMsg {
        chain_name: "osmosis".to_string(),
        owner: None,
    };
    kernel_contract.instantiate(&msg, None, None).unwrap();

    // 2. instantiate adodb contract
    let adodb_contract = AdodbContract::new(chain.clone());
    adodb_contract.upload().unwrap();
    let msg = adodb::InstantiateMsg {
        kernel_address: kernel_contract.addr_str().unwrap(),
        owner: None,
    };
    adodb_contract.instantiate(&msg, None, None).unwrap();

    // 3. instantiate vfs contract
    let vfs_contract = VfsContract::new(chain.clone());
    vfs_contract.upload().unwrap();
    let msg = vfs::InstantiateMsg {
        kernel_address: kernel_contract.addr_str().unwrap(),
        owner: None,
    };
    vfs_contract.instantiate(&msg, None, None).unwrap();

    // 4. instantiate economics contract
    let economics_contract = EconomicsContract::new(chain.clone());
    economics_contract.upload().unwrap();
    let msg = economics::InstantiateMsg {
        kernel_address: kernel_contract.addr_str().unwrap(),
        owner: None,
    };
    economics_contract.instantiate(&msg, None, None).unwrap();

    // 5. upload app contract
    let app_contract = AppContract::new(chain.clone());
    app_contract.upload().unwrap();

    // 6. upload validator staking contract
    let validator_staking_contract = ValidatorStakingContract::new(chain.clone());
    validator_staking_contract.upload().unwrap();

    // 7. register code ids in ado db
    chain
        .execute(
            &adodb::ExecuteMsg::Publish {
                code_id: adodb_contract.code_id().unwrap(),
                ado_type: "adodb".to_string(),
                action_fees: None,
                version: "0.1.0".to_string(),
                publisher: None,
            },
            &[],
            &adodb_contract.address().unwrap(),
        )
        .unwrap();
    chain
        .execute(
            &adodb::ExecuteMsg::Publish {
                code_id: vfs_contract.code_id().unwrap(),
                ado_type: "vfs".to_string(),
                action_fees: None,
                version: "0.1.0".to_string(),
                publisher: None,
            },
            &[],
            &adodb_contract.address().unwrap(),
        )
        .unwrap();

    chain
        .execute(
            &adodb::ExecuteMsg::Publish {
                code_id: kernel_contract.code_id().unwrap(),
                ado_type: "kernel".to_string(),
                action_fees: None,
                version: "0.1.0".to_string(),
                publisher: None,
            },
            &[],
            &adodb_contract.address().unwrap(),
        )
        .unwrap();

    chain
        .execute(
            &adodb::ExecuteMsg::Publish {
                code_id: app_contract.code_id().unwrap(),
                ado_type: "app-contract".to_string(),
                action_fees: None,
                version: "0.1.0".to_string(),
                publisher: None,
            },
            &[],
            &adodb_contract.address().unwrap(),
        )
        .unwrap();

    chain
        .execute(
            &adodb::ExecuteMsg::Publish {
                code_id: validator_staking_contract.code_id().unwrap(),
                ado_type: "validator-staking".to_string(),
                action_fees: None,
                version: "0.1.0".to_string(),
                publisher: None,
            },
            &[],
            &adodb_contract.address().unwrap(),
        )
        .unwrap();

    // 8. update kernel
    chain
        .execute(
            &kernel::ExecuteMsg::UpsertKeyAddress {
                key: "adodb".to_string(),
                value: adodb_contract.addr_str().unwrap(),
            },
            &[],
            &kernel_contract.address().unwrap(),
        )
        .unwrap();
    chain
        .execute(
            &kernel::ExecuteMsg::UpsertKeyAddress {
                key: "vfs".to_string(),
                value: vfs_contract.addr_str().unwrap(),
            },
            &[],
            &kernel_contract.address().unwrap(),
        )
        .unwrap();
    chain
        .execute(
            &kernel::ExecuteMsg::UpsertKeyAddress {
                key: "economics".to_string(),
                value: economics_contract.addr_str().unwrap(),
            },
            &[],
            &kernel_contract.address().unwrap(),
        )
        .unwrap();

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
