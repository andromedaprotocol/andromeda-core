// use std::str::FromStr;

// use andromeda_adodb::ADODBContract;
// use andromeda_app::app::AppComponent;
// use andromeda_app_contract::AppContract;
// use andromeda_economics::EconomicsContract;
// use andromeda_finance::splitter::AddressPercent;
// use andromeda_kernel::KernelContract;
// use andromeda_socket::osmosis::{
//     ExecuteMsgFns, InstantiateMsg, QueryMsgFns, Slippage, SwapAmountInRoute,
// };
// use andromeda_socket_osmosis::SocketOsmosisContract;
// use andromeda_std::amp::Recipient;
// use andromeda_std::os::adodb::{self, ExecuteMsgFns as AdodbExecuteMsgFns};
// use andromeda_std::os::kernel::{self, ExecuteMsgFns as KernelExecuteMsgFns};
// use andromeda_std::os::{economics, vfs};
// use andromeda_testing_e2e::mock::{mock_app, MockAndromeda};
// use andromeda_vfs::VFSContract;
// use cosmwasm_std::{coin, to_json_binary, Decimal, Uint128};
// use cw_orch::prelude::*;
// use cw_orch_daemon::{Daemon, DaemonBase, TxSender, Wallet};
// use e2e::constants::{OSMO_5, RECIPIENT_MNEMONIC_1, RECIPIENT_MNEMONIC_2};
// use osmosis_std::types::osmosis::gamm::v1beta1::PoolParams;
// use osmosis_std::types::{
//     cosmos::base::v1beta1::Coin as OsmosisCoin, osmosis::gamm::v1beta1::PoolAsset,
// };
// use rstest::{fixture, rstest};
// use std::time::{SystemTime, UNIX_EPOCH};

// struct TestCase {
//     daemon: DaemonBase<Wallet>,
//     app_contract: AppContract<DaemonBase<Wallet>>,
//     app_name: String,
// }

// const TEST_MNEMONIC: &str = "cereal gossip fox peace youth leader engage move brass sell gas trap issue simple dance source develop black hurt pulp burst predict patient onion";

// #[fixture]
// fn setup(
//     #[default(12441)] app_code_id: u64,
//     #[default("osmo17gxc6ec2cz2h6662tt8wajqaq57kwvdlzl63ceq9keeqm470ywyqrp9qux")]
//     kernel_address: String,
//     #[default(true)] use_splitter: bool,
// ) -> TestCase {
//     // println!("setup1");
//     // let daemon = mock_app(OSMO_5.clone(), TEST_MNEMONIC);
//     // println!("setup2");
//     // let mock_andromeda = MockAndromeda::new(&daemon);
//     // println!("setup3");
//     // let kernel_address = mock_andromeda.kernel_contract.addr_str().unwrap();

//     // let osmosis_socket_contract = SocketOsmosisContract::new(daemon.clone());
//     // osmosis_socket_contract.upload().unwrap();

//     let socket_osmosis_type = "socket-osmosis@0.1.2-b.1";
//     let socket_osmosis_component_name = "socket-osmosis";

//     // mock_andromeda
//     //     .adodb_contract
//     //     .clone()
//     //     .publish(
//     //         "socket-osmosis".to_string(),
//     //         osmosis_socket_contract.code_id().unwrap(),
//     //         "0.1.2-b.1".to_string(),
//     //         None,
//     //         None,
//     //     )
//     //     .unwrap();

//     let app_name = format!(
//         "socket osmosis with recipient {}",
//         SystemTime::now()
//             .duration_since(UNIX_EPOCH)
//             .expect("Check system time")
//             .as_millis()
//     );

//     let daemon = Daemon::builder(OSMO_5)
//         .mnemonic(TEST_MNEMONIC)
//         .build()
//         .unwrap();
//     let app_contract = AppContract::new(daemon.clone());
//     let kernel_contract = KernelContract::new(daemon.clone());
//     kernel_contract.upload().unwrap();
//     kernel_contract
//         .clone()
//         .instantiate(
//             &kernel::InstantiateMsg {
//                 chain_name: OSMO_5.network_info.chain_name.to_string(),
//                 owner: None,
//             },
//             None,
//             &[],
//         )
//         .unwrap();

//     let adodb_contract = ADODBContract::new(daemon.clone());
//     adodb_contract.upload().unwrap();
//     adodb_contract
//         .clone()
//         .instantiate(
//             &adodb::InstantiateMsg {
//                 kernel_address: kernel_contract.addr_str().unwrap(),
//                 owner: None,
//             },
//             None,
//             &[],
//         )
//         .unwrap();

//     let vfs_contract = VFSContract::new(daemon.clone());
//     vfs_contract.upload().unwrap();
//     vfs_contract
//         .clone()
//         .instantiate(
//             &vfs::InstantiateMsg {
//                 kernel_address: kernel_contract.addr_str().unwrap(),
//                 owner: None,
//             },
//             None,
//             &[],
//         )
//         .unwrap();

//     let economics_contract = EconomicsContract::new(daemon.clone());
//     economics_contract.upload().unwrap();
//     economics_contract
//         .clone()
//         .instantiate(
//             &economics::InstantiateMsg {
//                 kernel_address: kernel_contract.addr_str().unwrap(),
//                 owner: None,
//             },
//             None,
//             &[],
//         )
//         .unwrap();

//     adodb_contract
//         .clone()
//         .publish(
//             "adodb".to_string(),
//             adodb_contract.code_id().unwrap(),
//             "0.1.0".to_string(),
//             None,
//             None,
//         )
//         .unwrap();

//     adodb_contract
//         .clone()
//         .publish(
//             "vfs".to_string(),
//             vfs_contract.code_id().unwrap(),
//             "0.1.0".to_string(),
//             None,
//             None,
//         )
//         .unwrap();

//     adodb_contract
//         .clone()
//         .publish(
//             "kernel".to_string(),
//             kernel_contract.code_id().unwrap(),
//             "0.1.0".to_string(),
//             None,
//             None,
//         )
//         .unwrap();

//     // update kernel
//     kernel_contract
//         .clone()
//         .upsert_key_address("adodb".to_string(), adodb_contract.addr_str().unwrap())
//         .unwrap();
//     // .upsert_key_address("adodb".to_string(), adodb_contract.addr_str().unwrap());
//     kernel_contract
//         .clone()
//         .upsert_key_address("vfs".to_string(), vfs_contract.addr_str().unwrap())
//         .unwrap();
//     kernel_contract
//         .clone()
//         .upsert_key_address(
//             "economics".to_string(),
//             economics_contract.addr_str().unwrap(),
//         )
//         .unwrap();
//     // let kernel_contract = KernelContract::new(daemon.clone());
//     // let osmosis_socket_contract = SocketOsmosisContract::new(daemon.clone());
//     // osmosis_socket_contract.upload().unwrap();
//     // let adodb_contract = ADODBContract::new(daemon.clone());
//     // adodb_contract
//     //     .clone()
//     //     .publish(
//     //         "socket-osmosis".to_string(),
//     //         osmosis_socket_contract.code_id().unwrap(),
//     //         "0.1.2-b.1".to_string(),
//     //         None,
//     //         None,
//     //     )
//     //     .unwrap();

//     // kernel_contract.upload().unwrap();

//     // kernel_contract.instantiate(instantiate_msg, admin, coins)
//     app_contract.set_code_id(app_code_id);
//     let osmosis_socket_contract = SocketOsmosisContract::new(daemon.clone());
//     osmosis_socket_contract.upload().unwrap();

//     adodb_contract
//         .clone()
//         .publish(
//             "socket-osmosis".to_string(),
//             osmosis_socket_contract.code_id().unwrap(),
//             "0.1.2-b.1".to_string(),
//             None,
//             None,
//         )
//         .unwrap();

//     // Prepare app components
//     let socket_osmosis_init_msg = InstantiateMsg {
//         kernel_address: kernel_address.to_string(),
//         owner: None,
//         swap_router: None,
//     };

//     let socket_osmosis_component = AppComponent::new(
//         socket_osmosis_component_name,
//         socket_osmosis_type,
//         to_json_binary(&socket_osmosis_init_msg).unwrap(),
//     );
//     let app_components = if use_splitter {
//         let recipient_1_daemon = daemon
//             .rebuild()
//             .mnemonic(RECIPIENT_MNEMONIC_1)
//             .build()
//             .unwrap();
//         let recipient_2_daemon = daemon
//             .rebuild()
//             .mnemonic(RECIPIENT_MNEMONIC_2)
//             .build()
//             .unwrap();

//         let recipients = vec![
//             AddressPercent {
//                 recipient: Recipient::from_string(
//                     recipient_1_daemon.sender().address().to_string(),
//                 ),
//                 percent: Decimal::from_str("0.5").unwrap(),
//             },
//             AddressPercent {
//                 recipient: Recipient::from_string(
//                     recipient_2_daemon.sender().address().to_string(),
//                 ),
//                 percent: Decimal::from_str("0.5").unwrap(),
//             },
//         ];
//         let splitter_init_msg = andromeda_finance::splitter::InstantiateMsg {
//             recipients,
//             default_recipient: None,
//             lock_time: None,
//             kernel_address: kernel_address.to_string(),
//             owner: None,
//         };
//         let splitter_component = AppComponent::new(
//             "splitter".to_string(),
//             "splitter@2.3.1-b.3".to_string(),
//             to_json_binary(&splitter_init_msg).unwrap(),
//         );

//         vec![splitter_component.clone(), socket_osmosis_component.clone()]
//     } else {
//         vec![socket_osmosis_component.clone()]
//     };
//     println!("init1");
//     app_contract
//         .instantiate(
//             &andromeda_app::app::InstantiateMsg {
//                 app_components,
//                 name: app_name.clone(),
//                 chain_info: None,
//                 kernel_address: kernel_contract.addr_str().unwrap(),
//                 owner: None,
//             },
//             None,
//             &[],
//         )
//         .unwrap();
//     println!("init2");
//     TestCase {
//         daemon,
//         app_contract,
//         app_name,
//     }
// }

// #[rstest]
// fn test_onchain_native(setup: TestCase) {
//     let TestCase {
//         daemon,
//         app_contract,
//         app_name,
//     } = setup;
//     let app_name_parsed = app_name.replace(' ', "_");

//     let socket_osmosis_addr: String = app_contract.get_address("socket-osmosis");

//     let socket_osmosis_contract = SocketOsmosisContract::new(daemon.clone());
//     socket_osmosis_contract.set_address(&Addr::unchecked(socket_osmosis_addr));

//     // execute swap operation
//     let slippage = Slippage::MinOutputAmount(Uint128::one());
//     let atom_denom =
//         "ibc/A8C2D23A1E6F95DA4E48BA349667E322BD7A6C996D8A4AAE8BA72E190F3D1477".to_string();
//     let _res = socket_osmosis_contract.get_route("uosmo", atom_denom.clone());
//     let forward_msg =
//         to_json_binary(&andromeda_finance::splitter::ExecuteMsg::Send { config: None }).unwrap();
//     let forward_addr = Recipient::new(
//         format!(
//             "/home/{}/{}/{}",
//             daemon.sender().address(),
//             app_name_parsed,
//             "splitter"
//         ),
//         Some(forward_msg),
//     );

//     socket_osmosis_contract
//         .swap_and_forward(
//             slippage,
//             atom_denom.clone(),
//             Some(forward_addr),
//             Some(vec![SwapAmountInRoute {
//                 pool_id: "94".to_string(),
//                 token_out_denom: atom_denom.to_string(),
//             }]),
//             &[coin(1000000, OSMO_5.gas_denom)],
//         )
//         .unwrap();
// }

// #[rstest]
// fn test_create_pool(
//     #[with(
//         12441,
//         "osmo17gxc6ec2cz2h6662tt8wajqaq57kwvdlzl63ceq9keeqm470ywyqrp9qux".to_string(),
//         false
//     )]
//     setup: TestCase,
// ) {
//     let TestCase {
//         daemon,
//         app_contract,
//         ..
//     } = setup;

//     let socket_osmosis_addr: String = app_contract.get_address("socket-osmosis");

//     let socket_osmosis_contract = SocketOsmosisContract::new(daemon.clone());
//     socket_osmosis_contract.set_address(&Addr::unchecked(socket_osmosis_addr));

//     let pool_assets = vec![
//         PoolAsset {
//             token: Some(OsmosisCoin {
//                 denom: "uosmo".to_string(),
//                 amount: "2000".to_string(),
//             }),
//             weight: "2".to_string(),
//         },
//         PoolAsset {
//             token: Some(OsmosisCoin {
//                 denom: "uion".to_string(),
//                 amount: "1000".to_string(),
//             }),
//             weight: "1".to_string(),
//         },
//     ];

//     socket_osmosis_contract
//         .create_pool(andromeda_socket::osmosis::Pool::Balancer {
//             pool_params: None,
//             pool_assets,
//         })
//         .unwrap();
// }
