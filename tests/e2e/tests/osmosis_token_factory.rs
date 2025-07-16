use andromeda_osmosis_token_factory::OsmosisTokenFactoryContract;
use andromeda_socket::osmosis_token_factory::ExecuteMsgFns;
use andromeda_std::amp::AndrAddr;
use cosmwasm_std::Uint128;
use cw_orch::prelude::*;
use cw_orch_daemon::{Daemon, DaemonBase, Wallet};

use e2e::constants::OSMO_5;
use osmosis_std::types::cosmos::base::v1beta1::Coin as OsmosisCoin;
use rstest::{fixture, rstest};

struct TestCase {
    osmosis_token_factory_contract: OsmosisTokenFactoryContract<DaemonBase<Wallet>>,
}

const TEST_MNEMONIC: &str = "cereal gossip fox peace youth leader engage move brass sell gas trap issue simple dance source develop black hurt pulp burst predict patient onion";

#[fixture]
fn setup() -> TestCase {
    let daemon = Daemon::builder(OSMO_5)
        .mnemonic(TEST_MNEMONIC)
        .build()
        .unwrap();

    let osmosis_token_factory_contract = OsmosisTokenFactoryContract::new(daemon.clone());

    // Uncomment this if you want to upload and instantiate a new version of osmosis socket contract
    // Make sure to fund the contract after its instantiation
    osmosis_token_factory_contract.upload().unwrap();
    osmosis_token_factory_contract
        .instantiate(
            &andromeda_socket::osmosis_token_factory::InstantiateMsg {
                kernel_address: "osmo17gxc6ec2cz2h6662tt8wajqaq57kwvdlzl63ceq9keeqm470ywyqrp9qux"
                    .to_string(),
                authorized_address: AndrAddr::from_string(
                    "osmo1c2pgg87er3lg5wwrg8n475rdgvgjpqrz2mv3t7dzvl8egjpq95xsjquzc6",
                ),
                owner: None,
            },
            None,
            &[],
        )
        .unwrap();
    osmosis_token_factory_contract.set_address(&osmosis_token_factory_contract.address().unwrap());
    // osmosis_token_factory_contract.set_address(&Addr::unchecked(
    //     "osmo1r2vw2g92f5mt78mj029qlllfsfhrgyh6pzc4zgacllwg7p6x40rqnxgndc".to_string(),
    // ));

    TestCase {
        osmosis_token_factory_contract,
    }
}

#[rstest]
fn test_create_denom(setup: TestCase) {
    let TestCase {
        osmosis_token_factory_contract,
        ..
    } = setup;

    let socket_osmosis_addr: String = osmosis_token_factory_contract.addr_str().unwrap();
    println!("socket_osmosis_addr: {}", socket_osmosis_addr);

    let subdenom = "test".to_string();
    let amount = Uint128::from(10u128);

    let res = osmosis_token_factory_contract
        .create_denom(amount, subdenom, None, &[])
        .unwrap();
    println!("res: {:?}", res);
}

#[rstest]
fn test_burn(setup: TestCase) {
    let TestCase {
        osmosis_token_factory_contract,
        ..
    } = setup;

    let socket_osmosis_addr: String = osmosis_token_factory_contract.addr_str().unwrap();
    println!("socket_osmosis_addr: {}", socket_osmosis_addr);

    let subdenom = "test".to_string();
    let amount = Uint128::from(1u128);
    let denom = format!("factory/{}/{}", socket_osmosis_addr, subdenom);
    println!("denom: {}", denom);

    let coin = OsmosisCoin {
        denom: denom.clone(),
        amount: amount.to_string(),
    };

    let res = osmosis_token_factory_contract.burn(coin, &[]).unwrap();
    println!("res: {:?}", res);
}
