use std::str::FromStr;

use andromeda_app::app::AppComponent;
use andromeda_app_contract::AppContract;
use andromeda_finance::splitter::AddressPercent;
use andromeda_socket::osmosis::{
    ExecuteMsgFns, InstantiateMsg, QueryMsgFns, Slippage, SwapAmountInRoute,
};

use andromeda_std::amp::Recipient;
use cosmwasm_std::{coin, to_json_binary, Decimal, Uint128};
use cw_orch::prelude::*;
use cw_orch_daemon::{Daemon, DaemonBase, TxSender, Wallet};
use e2e::constants::{OSMO_5, RECIPIENT_MNEMONIC_1, RECIPIENT_MNEMONIC_2};

use andromeda_socket_osmosis::SocketOsmosisContract;

use rstest::{fixture, rstest};
use std::time::{SystemTime, UNIX_EPOCH};

struct TestCase {
    daemon: DaemonBase<Wallet>,
    app_contract: AppContract<DaemonBase<Wallet>>,
    app_name: String,
}

const TEST_MNEMONIC: &str = "cereal gossip fox peace youth leader engage move brass sell gas trap issue simple dance source develop black hurt pulp burst predict patient onion";

#[fixture]
fn setup(
    #[default(12441)] app_code_id: u64,
    #[default("osmo17gxc6ec2cz2h6662tt8wajqaq57kwvdlzl63ceq9keeqm470ywyqrp9qux")]
    kernel_address: String,
) -> TestCase {
    let socket_osmosis_type = "socket-osmosis@0.1.2-b.1";
    let socket_osmosis_component_name = "socket-osmosis";
    let app_name = format!(
        "socket osmosis with recipient {}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Check system time")
            .as_millis()
    );

    let daemon = Daemon::builder(OSMO_5)
        .mnemonic(TEST_MNEMONIC)
        .build()
        .unwrap();
    let app_contract = AppContract::new(daemon.clone());
    app_contract.set_code_id(app_code_id);

    // Prepare app components
    let socket_osmosis_init_msg = InstantiateMsg {
        kernel_address: kernel_address.to_string(),
        owner: None,
        swap_router: None,
    };

    let socket_osmosis_component = AppComponent::new(
        socket_osmosis_component_name,
        socket_osmosis_type,
        to_json_binary(&socket_osmosis_init_msg).unwrap(),
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
        recipients: Some(recipients),
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

    let app_components = vec![splitter_component.clone(), socket_osmosis_component.clone()];

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
    TestCase {
        daemon,
        app_contract,
        app_name,
    }
}

#[rstest]
fn test_onchain_native(setup: TestCase) {
    let TestCase {
        daemon,
        app_contract,
        app_name,
    } = setup;
    let app_name_parsed = app_name.replace(' ', "_");

    let socket_osmosis_addr: String = app_contract.get_address("socket-osmosis");

    let socket_osmosis_contract = SocketOsmosisContract::new(daemon.clone());
    socket_osmosis_contract.set_address(&Addr::unchecked(socket_osmosis_addr));

    // execute swap operation
    let slippage = Slippage::MinOutputAmount(Uint128::one());
    let atom_denom =
        "ibc/A8C2D23A1E6F95DA4E48BA349667E322BD7A6C996D8A4AAE8BA72E190F3D1477".to_string();
    let _res = socket_osmosis_contract.get_route("uosmo", atom_denom.clone());
    let forward_msg =
        to_json_binary(&andromeda_finance::splitter::ExecuteMsg::Send { config: None }).unwrap();
    let forward_addr = Recipient::new(
        format!(
            "/home/{}/{}/{}",
            daemon.sender().address(),
            app_name_parsed,
            "splitter"
        ),
        Some(forward_msg),
    );

    socket_osmosis_contract
        .swap_and_forward(
            slippage,
            atom_denom.clone(),
            Some(forward_addr),
            Some(vec![SwapAmountInRoute {
                pool_id: "94".to_string(),
                token_out_denom: atom_denom.to_string(),
            }]),
            &[coin(1000000, OSMO_5.gas_denom)],
        )
        .unwrap();
}
