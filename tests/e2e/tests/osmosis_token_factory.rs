use andromeda_osmosis_token_factory::OsmosisTokenFactoryContract;
// use andromeda_socket::osmosis_token_factory::{
//     AllLockedResponse, ExecuteMsgFns, FactoryDenomResponse, LockedResponse, QueryMsgFns,
// };
use andromeda_socket::osmosis_token_factory::ExecuteMsgFns;
// use cosmwasm_std::Uint128;
use cw_orch::prelude::*;
use cw_orch_daemon::{Daemon, DaemonBase, Wallet};

use e2e::constants::OSMO_5;
// use osmosis_std::types::cosmos::base::v1beta1::Coin as OsmosisCoin;
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

    let res = osmosis_token_factory_contract
        .create_denom(subdenom, &[])
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
    // let amount = Uint128::from(1u128);
    let denom = format!("factory/{}/{}", socket_osmosis_addr, subdenom);
    println!("denom: {}", denom);

    // let coin = OsmosisCoin {
    //     denom: denom.clone(),
    //     amount: amount.to_string(),
    // };

    let res = osmosis_token_factory_contract.burn(&[]).unwrap(); // TODO send funds
    println!("res: {:?}", res);
}

// #[rstest]
// fn test_mint(setup: TestCase) {
//     let TestCase {
//         osmosis_token_factory_contract,
//         ..
//     } = setup;

//     let socket_osmosis_addr: String = osmosis_token_factory_contract.addr_str().unwrap();
//     println!("socket_osmosis_addr: {}", socket_osmosis_addr);

//     let subdenom = "mint_test".to_string();
//     let amount = Uint128::from(100u128);
//     let denom = format!("factory/{}/{}", socket_osmosis_addr, subdenom);

//     let coin = OsmosisCoin {
//         denom: denom.clone(),
//         amount: amount.to_string(),
//     };

//     // First create the denom
//     let _create_res = osmosis_token_factory_contract
//         .create_denom(amount, subdenom.clone(), None, &[])
//         .unwrap();

//     // Then mint additional tokens
//     let mint_res = osmosis_token_factory_contract
//         .mint(coin, None, &[])
//         .unwrap();
//     println!("mint_res: {:?}", mint_res);
// }

// #[rstest]
// fn test_query_token_authority(setup: TestCase) {
//     let TestCase {
//         osmosis_token_factory_contract,
//         ..
//     } = setup;

//     let socket_osmosis_addr: String = osmosis_token_factory_contract.addr_str().unwrap();
//     let subdenom = "authority_test".to_string();
//     let amount = Uint128::from(50u128);
//     let denom = format!("factory/{}/{}", socket_osmosis_addr, subdenom);

//     // Create a denom first
//     let _create_res = osmosis_token_factory_contract
//         .create_denom(amount, subdenom, None, &[])
//         .unwrap();

//     // Query the token authority
//     let authority_res = osmosis_token_factory_contract
//         .token_authority(denom.clone())
//         .unwrap();

//     println!("token_authority for {}: {:?}", denom, authority_res);

//     // The contract should be the authority
//     assert_eq!(authority_res.authority_metadata.unwrap().admin, socket_osmosis_addr);
// }

// #[rstest]
// fn test_query_locked_empty(setup: TestCase) {
//     let TestCase {
//         osmosis_token_factory_contract,
//         ..
//     } = setup;

//     let user_addr = Addr::unchecked("osmo1test_user");
//     let cw20_addr = Addr::unchecked("osmo1test_cw20");

//     // Query locked amount for non-existent lock
//     let locked_res: LockedResponse = osmosis_token_factory_contract
//         .locked(user_addr, cw20_addr)
//         .unwrap();

//     println!("locked_res: {:?}", locked_res);
//     assert_eq!(locked_res.amount, Uint128::zero());
// }

// #[rstest]
// fn test_query_factory_denom_empty(setup: TestCase) {
//     let TestCase {
//         osmosis_token_factory_contract,
//         ..
//     } = setup;

//     let cw20_addr = Addr::unchecked("osmo1test_cw20_nonexistent");

//     // Query factory denom for non-existent CW20
//     let denom_res: FactoryDenomResponse = osmosis_token_factory_contract
//         .factory_denom(cw20_addr)
//         .unwrap();

//     println!("factory_denom_res: {:?}", denom_res);
//     assert_eq!(denom_res.denom, None);
// }

// #[rstest]
// fn test_query_all_locked_empty(setup: TestCase) {
//     let TestCase {
//         osmosis_token_factory_contract,
//         ..
//     } = setup;

//     let user_addr = Addr::unchecked("osmo1test_user_empty");

//     // Query all locked for user with no locks
//     let all_locked_res: AllLockedResponse = osmosis_token_factory_contract
//         .all_locked(user_addr)
//         .unwrap();

//     println!("all_locked_res: {:?}", all_locked_res);
//     assert_eq!(all_locked_res.locked.len(), 0);
// }

// #[rstest]
// fn test_error_burn_nonexistent_denom(setup: TestCase) {
//     let TestCase {
//         osmosis_token_factory_contract,
//         ..
//     } = setup;

//     let nonexistent_denom = "factory/osmo1nonexistent/fake".to_string();
//     let amount = Uint128::from(100u128);

//     let coin = OsmosisCoin {
//         denom: nonexistent_denom,
//         amount: amount.to_string(),
//     };

//     // This should fail because the denom doesn't exist or we don't have tokens
//     let burn_result = osmosis_token_factory_contract.burn(coin, &[]);

//     match burn_result {
//         Ok(_) => panic!("Expected burn to fail for nonexistent denom"),
//         Err(e) => {
//             println!("Expected error when burning nonexistent denom: {:?}", e);
//             // This is expected behavior
//         }
//     }
// }

// // #[rstest]
// // fn test_error_unlock_invalid_denom(setup: TestCase) {
// //     let TestCase {
// //         osmosis_token_factory_contract,
// //         ..
// //     } = setup;

// //     let cw20_addr = Addr::unchecked("osmo1fake_cw20");
// //     let fake_denom = "factory/osmo1fake/wrong".to_string();
// //     let amount = Uint128::from(100u128);

// //     // This should fail because there's no factory denom mapping for this CW20
// //     let unlock_result = osmosis_token_factory_contract.unlock(
// //         amount,
// //         cw20_addr.clone(),
// //         fake_denom,
// //         &[]
// //     );

// //     match unlock_result {
// //         Ok(_) => panic!("Expected unlock to fail for invalid denom mapping"),
// //         Err(e) => {
// //             println!("Expected error when unlocking with invalid denom: {:?}", e);
// //             // This is expected behavior - should fail with "Invalid factory denom" or similar
// //         }
// //     }
// // }
