use andromeda_socket::osmosis::ExecuteMsgFns;
use andromeda_socket_osmosis::SocketOsmosisContract;
use cosmwasm_std::{coin, Uint128};
use cw_orch::prelude::*;
use cw_orch_daemon::{Daemon, DaemonBase, Wallet};

use e2e::constants::OSMO_5;
use osmosis_std::types::{
    cosmos::base::v1beta1::Coin as OsmosisCoin,
    osmosis::gamm::v1beta1::{MsgExitPool, PoolAsset, PoolParams},
};
use rstest::{fixture, rstest};

struct TestCase {
    osmosis_socket_contract: SocketOsmosisContract<DaemonBase<Wallet>>,
}

const TEST_MNEMONIC: &str = "cereal gossip fox peace youth leader engage move brass sell gas trap issue simple dance source develop black hurt pulp burst predict patient onion";

#[fixture]
fn setup() -> TestCase {
    let daemon = Daemon::builder(OSMO_5)
        .mnemonic(TEST_MNEMONIC)
        .build()
        .unwrap();

    let osmosis_socket_contract = SocketOsmosisContract::new(daemon.clone());

    // // Uncomment this if you want to upload and instantiate a new version of osmosis socket contract
    // // Make sure to fund the contract after its instantiation
    // osmosis_socket_contract.upload().unwrap();
    // osmosis_socket_contract
    //     .instantiate(
    //         &andromeda_socket::osmosis::InstantiateMsg {
    //             kernel_address: "osmo17gxc6ec2cz2h6662tt8wajqaq57kwvdlzl63ceq9keeqm470ywyqrp9qux"
    //                 .to_string(),
    //             owner: None,
    //             swap_router: None,
    //         },
    //         None,
    //         &[],
    //     )
    //     .unwrap();
    // osmosis_socket_contract.set_address(&osmosis_socket_contract.address().unwrap());
    osmosis_socket_contract.set_address(&Addr::unchecked(
        "osmo1daajc40gp7ewhn7vjewmgwc8vwe4dqky98fp7smft5krjwlkxekqm70lxf".to_string(),
    ));

    TestCase {
        osmosis_socket_contract,
    }
}

#[rstest]
fn test_create_pool(setup: TestCase) {
    let TestCase {
        osmosis_socket_contract,
        ..
    } = setup;

    let socket_osmosis_addr: String = osmosis_socket_contract.addr_str().unwrap();
    println!("socket_osmosis_addr: {}", socket_osmosis_addr);

    let pool_assets = vec![
        PoolAsset {
            token: Some(OsmosisCoin {
                denom: "uosmo".to_string(),
                amount: "10000000".to_string(),
            }),
            weight: "50000".to_string(),
        },
        PoolAsset {
            token: Some(OsmosisCoin {
                denom: "uion".to_string(),
                amount: "1000".to_string(),
            }),
            weight: "50000".to_string(),
        },
    ];

    let pool_params = PoolParams {
        swap_fee: "1".into(),
        exit_fee: "0".into(),
        smooth_weight_change_params: None,
    };

    // The contract itself should have those funds, I funded the contract then called this function
    // The contract receives the lp tokens and then transfers them to the user in the reply function
    let res = osmosis_socket_contract
        .create_pool(
            andromeda_socket::osmosis::Pool::Balancer {
                pool_params: Some(pool_params),
                pool_assets,
            },
            &[coin(1000, "uion"), coin(10000000, "uosmo")],
        )
        .unwrap();
    println!("res: {:?}", res);
}

#[rstest]
fn test_withdraw_pool(setup: TestCase) {
    let TestCase {
        osmosis_socket_contract,
        ..
    } = setup;

    let socket_osmosis_addr: String = osmosis_socket_contract.addr_str().unwrap();
    println!("socket_osmosis_addr: {}", socket_osmosis_addr);

    let _wallet_address = "osmo18epw87zc64a6m63323l6je0nlwdhnjpghtsyq8".to_string();
    let withdraw_msg = MsgExitPool {
        sender: socket_osmosis_addr,
        pool_id: 940, // Don't forget to change the pool id if you created a new one
        share_in_amount: "50000000000000000000".to_string(),
        token_out_mins: vec![
            OsmosisCoin {
                denom: "uion".to_string(),
                amount: "487".to_string(),
            },
            OsmosisCoin {
                denom: "uosmo".to_string(),
                amount: "4875000".to_string(),
            },
        ],
    };
    // At this point, the lp tokens are in the user's wallet
    osmosis_socket_contract
        .withdraw_pool(withdraw_msg, &[coin(50000000000000000000, "gamm/pool/940")]) // The denom will need to be updated if you created a new pool
        .unwrap();
}

#[rstest]
fn test_create_denom(setup: TestCase) {
    let TestCase {
        osmosis_socket_contract,
        ..
    } = setup;

    let socket_osmosis_addr: String = osmosis_socket_contract.addr_str().unwrap();
    println!("socket_osmosis_addr: {}", socket_osmosis_addr);

    let subdenom = "test".to_string();
    let amount = Uint128::from(10u128);

    let res = osmosis_socket_contract
        .create_denom(amount, subdenom, &[])
        .unwrap();
    println!("res: {:?}", res);
}

#[rstest]
fn test_burn(setup: TestCase) {
    let TestCase {
        osmosis_socket_contract,
        ..
    } = setup;

    let socket_osmosis_addr: String = osmosis_socket_contract.addr_str().unwrap();
    println!("socket_osmosis_addr: {}", socket_osmosis_addr);

    let subdenom = "test".to_string();
    let amount = Uint128::from(1u128);
    let denom = format!("factory/{}/{}", socket_osmosis_addr, subdenom);

    let coin = OsmosisCoin {
        denom: denom.clone(),
        amount: amount.to_string(),
    };

    let res = osmosis_socket_contract.burn(coin, &[]).unwrap();
    println!("res: {:?}", res);
}
