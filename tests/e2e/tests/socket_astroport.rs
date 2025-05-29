use andromeda_socket::astroport::QueryMsgFns;
use lazy_static::lazy_static;
use std::str::FromStr;
use std::sync::Mutex;
use std::sync::Once;

use andromeda_app::app::AppComponent;
use andromeda_app_contract::AppContract;
use andromeda_finance::splitter::AddressPercent;
use andromeda_socket::astroport::{
    AssetEntry, AssetInfo, ExecuteMsg as SocketAstroportExecuteMsg, ExecuteMsgFns, InstantiateMsg,
    PairType,
};

use andromeda_cw20::CW20Contract;
use andromeda_fungible_tokens::cw20::ExecuteMsg as Cw20ExecuteMsg;
use andromeda_std::{
    amp::{messages::AMPMsg, AndrAddr, Recipient},
    common::denom::Asset,
};
use cosmwasm_std::{coin, to_json_binary, Decimal, Uint128};
use cw_orch::prelude::*;
use cw_orch_daemon::{Daemon, DaemonBase, TxSender, Wallet};
use e2e::constants::{PION_1, RECIPIENT_MNEMONIC_1, RECIPIENT_MNEMONIC_2};

use std::time::{SystemTime, UNIX_EPOCH};

use andromeda_socket_astroport::SocketAstroportContract;

use rstest::{fixture, rstest};

struct TestCase {
    daemon: DaemonBase<Wallet>,
    app_contract: AppContract<DaemonBase<Wallet>>,
    app_name: String,
    kernel: andromeda_kernel::KernelContract<DaemonBase<Wallet>>,
}

const TEST_MNEMONIC: &str = "cereal gossip fox peace youth leader engage move brass sell gas trap issue simple dance source develop black hurt pulp burst predict patient onion";

//Added to prevent concurrency issues (Accessing the same state file at the same time)
lazy_static! {
    static ref DAEMON_MUTEX: Mutex<()> = Mutex::new(());
    static ref INIT_LOGGER: Once = Once::new();
}

#[fixture]
fn setup(
    #[default(11806)] app_code_id: u64,
    #[default("neutron1p3gh4zanng04zdnvs8cdh2kdsjrcp43qkp9zk32enu9waxv4wrmqpqnl9g")]
    kernel_address: String,
) -> TestCase {
    INIT_LOGGER.call_once(|| {
        env_logger::init();
    });

    let _lock = DAEMON_MUTEX.lock().unwrap();

    let socket_astroport_type = "socket-astroport@0.1.7-b.1";
    let socket_astroport_component_name = "socket-astroport";
    let app_name = format!(
        "socket astroport with recipient {}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Check system time")
            .as_millis()
    );

    let daemon = Daemon::builder(PION_1)
        .mnemonic(TEST_MNEMONIC)
        .build()
        .unwrap();
    let app_contract = AppContract::new(daemon.clone());
    app_contract.set_code_id(app_code_id);

    let kernel_address = kernel_address.to_string();

    // Prepare app components
    let socket_astroport_init_msg = InstantiateMsg {
        kernel_address: kernel_address.to_string().clone(),
        owner: None,
        swap_router: None,
    };

    // kernal is already on chain add a varuable to access it
    //its not a component but a contract
    let kernel = andromeda_kernel::KernelContract::new(daemon.clone());
    kernel.set_address(&Addr::unchecked(kernel_address.clone()));

    let socket_astroport_component = AppComponent::new(
        socket_astroport_component_name,
        socket_astroport_type,
        to_json_binary(&socket_astroport_init_msg).unwrap(),
    );

    let recipient_1_daemon = daemon
        .rebuild()
        .mnemonic(RECIPIENT_MNEMONIC_1)
        .build()
        .unwrap();
    let recipient_2_daemon = daemon
        .rebuild()
        .mnemonic(RECIPIENT_MNEMONIC_2)
        .build()
        .unwrap();

    let recipients = vec![
        AddressPercent {
            recipient: Recipient::from_string(recipient_1_daemon.sender().address().to_string()),
            percent: Decimal::from_str("0.5").unwrap(),
        },
        AddressPercent {
            recipient: Recipient::from_string(recipient_2_daemon.sender().address().to_string()),
            percent: Decimal::from_str("0.5").unwrap(),
        },
    ];
    let splitter_init_msg = andromeda_finance::splitter::InstantiateMsg {
        recipients,
        default_recipient: None,
        lock_time: None,
        kernel_address: kernel_address.to_string(),
        owner: None,
    };

    let splitter_component = AppComponent::new(
        "splitter".to_string(),
        "splitter@2.3.1-b.3".to_string(),
        to_json_binary(&splitter_init_msg).unwrap(),
    );

    // Add CW20 component for creating native CW20 tokens on Neutron
    let cw20_init_msg = andromeda_fungible_tokens::cw20::InstantiateMsg {
        name: "TestToken".to_string(),
        symbol: "Test".to_string(),
        decimals: 6,
        initial_balances: vec![cw20::Cw20Coin {
            address: daemon.sender().address().to_string(),
            amount: Uint128::new(1000000000), // 1000 tokens with 6 decimals
        }],
        mint: Some(cw20::MinterResponse {
            minter: daemon.sender().address().to_string(),
            cap: Some(Uint128::new(10000000000)), // 10k token cap
        }),
        marketing: None,
        kernel_address: kernel_address.to_string(),
        owner: Some(daemon.sender().address().to_string()),
    };

    let cw20_component = AppComponent::new(
        "cw20".to_string(),
        "cw20@2.1.1-b.2".to_string(),
        to_json_binary(&cw20_init_msg).unwrap(),
    );

    let app_components = vec![
        splitter_component.clone(),
        socket_astroport_component.clone(),
        cw20_component.clone(),
    ];

    app_contract
        .instantiate(
            &andromeda_app::app::InstantiateMsg {
                app_components,
                name: app_name.clone(),
                chain_info: None,
                kernel_address: kernel_address.to_string(),
                owner: None,
            },
            None,
            &[],
        )
        .unwrap();

    let socket_astroport_contract = SocketAstroportContract::new(daemon.clone());
    socket_astroport_contract.upload().unwrap();

    // Instantiate the fresh contract
    let fresh_init_msg = InstantiateMsg {
        kernel_address: kernel.address().unwrap().to_string(),
        owner: None,
        swap_router: None,
    };

    socket_astroport_contract
        .instantiate(&fresh_init_msg, None, &[])
        .unwrap();
    let fresh_socket_addr = socket_astroport_contract.address().unwrap().to_string();
    println!("🚀 Using fresh contract at: {}", fresh_socket_addr);

    TestCase {
        daemon,
        app_contract,
        app_name,
        kernel,
    }
}

#[rstest]
fn test_onchain_native_astroport(setup: TestCase) {
    let TestCase {
        daemon,
        app_contract,
        ..
    } = setup;

    let socket_astroport_addr: String = app_contract.get_address("socket-astroport");

    let socket_astroport_contract = SocketAstroportContract::new(daemon.clone());
    socket_astroport_contract.set_address(&Addr::unchecked(socket_astroport_addr));
    // execute swap operation
    let usdt_address = "neutron1vpsgrzedwd8fezpsu9fcfewvp6nmv4kzd7a6nutpmgeyjk3arlqsypnlhm";

    socket_astroport_contract
        .swap_and_forward(
            Asset::Cw20Token(AndrAddr::from_string(usdt_address)),
            None,
            None,
            None,
            None,
            &[coin(100, PION_1.gas_denom)],
        )
        .unwrap();
}

#[rstest]
fn test_onchain_cw20(setup: TestCase) {
    let TestCase {
        daemon,
        app_contract,
        app_name,
        ..
    } = setup;

    let app_name_parsed = app_name.replace(' ', "_");

    let socket_astroport_addr: String = app_contract.get_address("socket-astroport");

    let socket_astroport_contract = SocketAstroportContract::new(daemon.clone());
    socket_astroport_contract.set_address(&Addr::unchecked(socket_astroport_addr));

    // execute swap operation
    let usdt_address = "neutron1vpsgrzedwd8fezpsu9fcfewvp6nmv4kzd7a6nutpmgeyjk3arlqsypnlhm";

    let forward_msg =
        to_json_binary(&andromeda_finance::splitter::ExecuteMsg::Send { config: None }).unwrap();
    let recipient = Recipient::new(
        format!(
            "/home/{}/{}/{}",
            daemon.sender().address(),
            app_name_parsed,
            "splitter"
        ),
        Some(forward_msg),
    );

    socket_astroport_contract.execute_swap_from_cw20(
        &daemon,
        usdt_address,
        Uint128::new(36),
        Asset::NativeToken(PION_1.gas_denom.to_string()),
        Some(recipient),
        None,
        None,
        None,
    );
}

#[rstest]
fn test_onchain_native_to_native(setup: TestCase) {
    let TestCase {
        daemon,
        app_contract,
        app_name,
        ..
    } = setup;

    let app_name_parsed = app_name.replace(' ', "_");

    let socket_astroport_addr: String = app_contract.get_address("socket-astroport");

    let socket_astroport_contract = SocketAstroportContract::new(daemon.clone());
    socket_astroport_contract.set_address(&Addr::unchecked(socket_astroport_addr));

    // execute swap operation
    let forward_msg =
        to_json_binary(&andromeda_finance::splitter::ExecuteMsg::Send { config: None }).unwrap();
    let recipient = Recipient::new(
        format!(
            "/home/{}/{}/{}",
            daemon.sender().address(),
            app_name_parsed,
            "splitter"
        ),
        Some(forward_msg),
    );

    let osmos_denom = "ibc/0471F1C4E7AFD3F07702BEF6DC365268D64570F7C1FDC98EA6098DD6DE59817B";
    let astro_denom = "ibc/8D8A7F7253615E5F76CB6252A1E1BD921D5EDB7BBAAF8913FB1C77FF125D9995";

    let _ = socket_astroport_contract.swap_and_forward(
        Asset::NativeToken(osmos_denom.to_owned()),
        None,
        None,
        None,
        Some(recipient),
        &[coin(100000000, astro_denom)],
    );
}

// This test is added for debugging purposes
#[rstest]
fn test_create_pair(setup: TestCase) {
    let TestCase {
        daemon,
        app_contract,
        ..
    } = setup;

    let socket_astroport_addr: String = app_contract.get_address("socket-astroport");

    let socket_astroport_contract = SocketAstroportContract::new(daemon.clone());
    socket_astroport_contract.set_address(&Addr::unchecked(socket_astroport_addr));

    // Get token addresses
    let usdt_address = "neutron1vpsgrzedwd8fezpsu9fcfewvp6nmv4kzd7a6nutpmgeyjk3arlqsypnlhm";
    let osmos_denom = "ibc/0471F1C4E7AFD3F07702BEF6DC365268D64570F7C1FDC98EA6098DD6DE59817B";

    // Create asset infos for the pair
    let asset_infos = vec![
        AssetInfo::Token {
            contract_addr: Addr::unchecked(usdt_address),
        },
        AssetInfo::NativeToken {
            denom: osmos_denom.to_string(),
        },
    ];

    let pair_type = PairType::Xyk {};

    let result = socket_astroport_contract.create_pair(asset_infos, pair_type, None);

    println!("Create pair result: {:?}", result);
    assert!(result.is_ok(), "Create pair should succeed");
}

#[rstest]
fn test_create_pair_and_provide_liquidity_direct_socket_call(setup: TestCase) {
    let TestCase {
        daemon,
        app_contract,
        ..
    } = setup;

    let socket_astroport_addr: String = app_contract.get_address("socket-astroport");
    // Get the socket contract address from the app
    let socket_astroport_contract = SocketAstroportContract::new(daemon.clone());
    socket_astroport_contract.set_address(&Addr::unchecked(socket_astroport_addr));

    let cw20_token_address: String = app_contract.get_address("cw20");
    let cw20_contract = CW20Contract::new(daemon.clone());
    cw20_contract.set_address(&Addr::unchecked(cw20_token_address.clone()));

    // Get the CW20 token address from the app components
    let neutron_native_denom = "untrn"; // Neutron's native token

    // Create asset infos for the pair (CW20 token + native token)
    let asset_infos = vec![
        AssetInfo::Token {
            contract_addr: Addr::unchecked(cw20_token_address.clone()),
        },
        AssetInfo::NativeToken {
            denom: neutron_native_denom.to_string(),
        },
    ];

    let pair_type = PairType::Xyk {};

    let native_amount = Uint128::new(100000);
    let cw20_amount = Uint128::new(1000000);

    // Create the assets for liquidity provision
    let assets = vec![
        AssetEntry {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked(cw20_token_address.clone()),
            },
            amount: cw20_amount,
        },
        AssetEntry {
            info: AssetInfo::NativeToken {
                denom: neutron_native_denom.to_string(),
            },
            amount: native_amount,
        },
    ];

    let cw20_transfer_msg = Cw20ExecuteMsg::Transfer {
        recipient: AndrAddr::from_string(app_contract.get_address("socket-astroport").clone()),
        amount: cw20_amount,
    };

    let result = cw20_contract.execute(&cw20_transfer_msg, &[]);

    assert!(
        result.is_ok(),
        "Should successfully transfer CW20 tokens to socket contract. Error: {:?}",
        result.err()
    );

    let socket_msg = andromeda_socket::astroport::ExecuteMsg::CreatePairAndProvideLiquidity {
        pair_type,
        asset_infos,
        init_params: None,
        assets,
        slippage_tolerance: Some(Decimal::percent(10)),
        auto_stake: Some(false),
        receiver: None,
    };

    let result = socket_astroport_contract.execute(
        &socket_msg,
        &[coin(native_amount.u128(), neutron_native_denom)], // Send native tokens directly
    );

    assert!(
        result.is_ok(),
        "Socket contract should successfully create pair and provide liquidity. Error: {:?}",
        result.err()
    );

    println!("✅ Direct socket call succeeded!");
}

#[rstest]
fn test_create_pair_and_provide_liquidity_through_kernel(setup: TestCase) {
    let TestCase {
        daemon,
        app_contract,
        kernel,
        ..
    } = setup;

    let socket_astroport_addr: String = app_contract.get_address("socket-astroport");
    let cw20_token_address: String = app_contract.get_address("cw20");
    let cw20_contract = CW20Contract::new(daemon.clone());
    cw20_contract.set_address(&Addr::unchecked(cw20_token_address.clone()));

    // Get the CW20 token address from the app components
    let neutron_native_denom = "untrn"; // Neutron's native token

    // Create asset infos for the pair (CW20 token + native token)
    let asset_infos = vec![
        AssetInfo::Token {
            contract_addr: Addr::unchecked(cw20_token_address.clone()),
        },
        AssetInfo::NativeToken {
            denom: neutron_native_denom.to_string(),
        },
    ];

    let pair_type = PairType::Xyk {};

    let native_amount = Uint128::new(100000);
    let cw20_amount = Uint128::new(1000000);

    // Create the assets for liquidity provision
    let assets = vec![
        AssetEntry {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked(cw20_token_address.clone()),
            },
            amount: cw20_amount,
        },
        AssetEntry {
            info: AssetInfo::NativeToken {
                denom: neutron_native_denom.to_string(),
            },
            amount: native_amount,
        },
    ];

    // First transfer CW20 tokens to the socket contract
    let cw20_transfer_msg = Cw20ExecuteMsg::Transfer {
        recipient: AndrAddr::from_string(socket_astroport_addr.clone()),
        amount: cw20_amount,
    };

    let result = cw20_contract.execute(&cw20_transfer_msg, &[]);
    assert!(
        result.is_ok(),
        "Should successfully transfer CW20 tokens to socket contract. Error: {:?}",
        result.err()
    );

    // Create the socket message to execute
    let socket_msg = andromeda_socket::astroport::ExecuteMsg::CreatePairAndProvideLiquidity {
        pair_type,
        asset_infos,
        init_params: None,
        assets,
        slippage_tolerance: Some(Decimal::percent(10)),
        auto_stake: Some(false),
        receiver: None,
    };

    // Create an AMPMsg to send through the kernel
    let amp_message = AMPMsg::new(
        AndrAddr::from_string(socket_astroport_addr.clone()),
        to_json_binary(&socket_msg).unwrap(),
        Some(vec![coin(native_amount.u128(), neutron_native_denom)]),
    );

    // Execute through the kernel
    let result = kernel.execute(
        &andromeda_std::os::kernel::ExecuteMsg::Send {
            message: amp_message,
        },
        &[coin(native_amount.u128(), neutron_native_denom)],
    );

    assert!(
        result.is_ok(),
        "Kernel should successfully execute the message. Error: {:?}",
        result.err()
    );
}

#[rstest]
fn test_full_liquidity_cycle_with_fresh_contract(setup: TestCase) {
    let TestCase {
        daemon,
        app_contract,
        kernel,
        ..
    } = setup;

    let socket_astroport_addr: String = app_contract.get_address("socket-astroport");
    let cw20_token_address: String = app_contract.get_address("cw20");
    let cw20_contract = CW20Contract::new(daemon.clone());
    cw20_contract.set_address(&Addr::unchecked(cw20_token_address.clone()));

    let socket_astroport_contract = SocketAstroportContract::new(daemon.clone());
    let upload_res = socket_astroport_contract.upload().unwrap();
    println!(
        "📦 Uploaded fresh contract with code_id: {}",
        upload_res.uploaded_code_id().unwrap()
    );

    // Instantiate the fresh contract
    let fresh_init_msg = InstantiateMsg {
        kernel_address: kernel.address().unwrap().to_string(),
        owner: None,
        swap_router: None,
    };

    socket_astroport_contract
        .instantiate(&fresh_init_msg, None, &[])
        .unwrap();
    let fresh_socket_addr = socket_astroport_contract.address().unwrap().to_string();
    println!("🚀 Using fresh contract at: {}", fresh_socket_addr);

    let neutron_native_denom = "untrn";
    let sender_address = daemon.sender().address().to_string();

    println!("=== Fresh Contract Liquidity Cycle Test ===");
    println!("Socket address: {}", socket_astroport_addr);
    println!("CW20 token address: {}", cw20_token_address);
    println!("Sender address: {}", sender_address);

    // Create asset infos for the pair (CW20 token + native token)
    let asset_infos = vec![
        AssetInfo::Token {
            contract_addr: Addr::unchecked(cw20_token_address.clone()),
        },
        AssetInfo::NativeToken {
            denom: neutron_native_denom.to_string(),
        },
    ];

    let pair_type = PairType::Xyk {};
    let native_amount = Uint128::new(100000);
    let cw20_amount = Uint128::new(1000000);

    // Create the assets for liquidity provision
    let assets = vec![
        AssetEntry {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked(cw20_token_address.clone()),
            },
            amount: cw20_amount,
        },
        AssetEntry {
            info: AssetInfo::NativeToken {
                denom: neutron_native_denom.to_string(),
            },
            amount: native_amount,
        },
    ];

    // Transfer CW20 tokens to the fresh socket contract (not the old app one)
    println!("=== Step 1: Transferring CW20 tokens to fresh socket ===");
    let cw20_transfer_msg = Cw20ExecuteMsg::Transfer {
        recipient: AndrAddr::from_string(fresh_socket_addr.clone()),
        amount: cw20_amount,
    };

    let result = cw20_contract.execute(&cw20_transfer_msg, &[]);
    assert!(
        result.is_ok(),
        "Should successfully transfer CW20 tokens to fresh socket contract. Error: {:?}",
        result.err()
    );

    // Create pair and provide liquidity through kernel using fresh contract
    println!("=== Step 2: Creating pair and providing liquidity with fresh contract ===");
    let socket_msg = andromeda_socket::astroport::ExecuteMsg::CreatePairAndProvideLiquidity {
        pair_type,
        asset_infos,
        init_params: None,
        assets,
        slippage_tolerance: Some(Decimal::percent(10)),
        auto_stake: Some(false),
        receiver: Some(sender_address.clone()),
    };

    let amp_message = AMPMsg::new(
        AndrAddr::from_string(fresh_socket_addr.clone()),
        to_json_binary(&socket_msg).unwrap(),
        Some(vec![coin(native_amount.u128(), neutron_native_denom)]),
    );

    let result = kernel.execute(
        &andromeda_std::os::kernel::ExecuteMsg::Send {
            message: amp_message,
        },
        &[coin(native_amount.u128(), neutron_native_denom)],
    );

    assert!(
        result.is_ok(),
        "Kernel should successfully execute the message. Error: {:?}",
        result.err()
    );

    println!("✅ Liquidity provided successfully through kernel!");

    // Extract LP token denom from the transaction response
    let tx_response = result.unwrap();
    let mut lp_token_denom: Option<String> = None;

    // Look for the create_denom event to find the LP token denom
    for event in &tx_response.events {
        if event.r#type == "create_denom" {
            for attr in &event.attributes {
                if attr.key == "new_token_denom".as_bytes() {
                    lp_token_denom = Some(String::from_utf8_lossy(&attr.value).to_string());
                    break;
                }
            }
        }
    }

    let lp_token_denom =
        lp_token_denom.expect("LP token denom should be found in transaction events");
    println!("🪙 LP Token Denom: {}", lp_token_denom);

    // Wait and get pair address using the NEW query!
    println!("=== Step 3: Testing NEW LpPairAddress query ===");
    std::thread::sleep(std::time::Duration::from_secs(5));

    // This should now work with our fresh contract that has the LpPairAddress query!
    let pair_address_response = socket_astroport_contract.lp_pair_address();
    println!("NEW LP Pair address response: {:?}", pair_address_response);
    assert!(
        pair_address_response.is_ok(),
        "Should be able to query LP pair address with the NEW query. Error: {:?}",
        pair_address_response.err()
    );

    let pair_address = pair_address_response.unwrap().lp_pair_address;
    assert!(
        pair_address.is_some(),
        "LP Pair address should be set after creating pair"
    );

    let pair_address = pair_address.unwrap();
    println!("📍 LP Pair contract address: {}", pair_address);

    let withdraw_msg = SocketAstroportExecuteMsg::WithdrawLiquidity {};
    let funds = vec![coin(20, lp_token_denom.clone())];

    let withdraw_amp_message = AMPMsg::new(
        AndrAddr::from_string(fresh_socket_addr.clone()),
        to_json_binary(&withdraw_msg).unwrap(),
        Some(funds.clone()),
    );

    println!("🔄 Executing withdraw liquidity call...");
    let withdraw_result = kernel.execute(
        &andromeda_std::os::kernel::ExecuteMsg::Send {
            message: withdraw_amp_message,
        },
        &[funds[0].clone()],
    );

    assert!(
        withdraw_result.is_ok(),
        "Withdraw liquidity should succeed. Error: {:?}",
        withdraw_result.err()
    );

    let tx_response = withdraw_result.unwrap();
    let mut funds_received = false;

    // Look for refund_assets or transfer events to confirm we got assets back
    for event in &tx_response.events {
        if event.r#type == "wasm" {
            for attr in &event.attributes {
                if attr.key == "refund_assets".as_bytes() {
                    let refunded_assets = String::from_utf8_lossy(&attr.value);
                    println!("💰 Received refunded assets: {}", refunded_assets);
                    funds_received = true;
                    break;
                }
            }
        }
    }

    assert!(
        funds_received,
        "Should receive refunded assets from liquidity withdrawal"
    );

    println!("✅ Withdrawal completed successfully with funds received back!");
}
