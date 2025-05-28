use andromeda_socket::osmosis::ExecuteMsgFns;
use andromeda_socket_osmosis::SocketOsmosisContract;

use cosmwasm_std::coin;
use cw_orch::prelude::*;
use cw_orch_daemon::{Daemon, DaemonBase, Wallet};

use e2e::constants::OSMO_5;
use osmosis_std::types::{
    cosmos::base::v1beta1::Coin as OsmosisCoin, osmosis::gamm::v1beta1::PoolAsset,
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
    osmosis_socket_contract.set_address(&Addr::unchecked(
        "osmo188j4ew2wfz5xrp8am8ytsed45gqscehlprtd6u9y4mnlyvaut9rqn3nhzq".to_string(),
    ));
    // osmosis_socket_contract.upload().unwrap();
    // osmosis_socket_contract
    //     .instantiate(
    //         &InstantiateMsg {
    //             kernel_address: kernel_address.to_string(),
    //             owner: None,
    //             swap_router: None,
    //         },
    //         None,
    //         &[],
    //     )
    //     .unwrap();

    // // Prepare app components
    // let socket_osmosis_init_msg = InstantiateMsg {
    //     kernel_address: kernel_address.to_string(),
    //     owner: None,
    //     swap_router: None,
    // };

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

    // let socket_osmosis_addr: String = osmosis_socket_contract.addr_str().unwrap();
    // println!("socket_osmosis_addr: {}", socket_osmosis_addr);

    let pool_assets = vec![
        PoolAsset {
            token: Some(OsmosisCoin {
                denom: "uosmo".to_string(),
                amount: "10000000".to_string(),
            }),
            weight: "500000".to_string(),
        },
        PoolAsset {
            token: Some(OsmosisCoin {
                denom: "uion".to_string(),
                amount: "10000".to_string(),
            }),
            weight: "500000".to_string(),
        },
    ];

    osmosis_socket_contract
        .create_pool(
            andromeda_socket::osmosis::Pool::Balancer {
                pool_params: None,
                pool_assets,
            },
            &[coin(10000, "uion"), coin(10000000, "uosmo")],
        )
        .unwrap();
}
