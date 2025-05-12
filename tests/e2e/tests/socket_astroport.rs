use std::str::FromStr;

use andromeda_app::app::AppComponent;
use andromeda_app_contract::AppContract;
use andromeda_finance::splitter::AddressPercent;
use andromeda_socket::astroport::{ExecuteMsgFns, InstantiateMsg};

use andromeda_std::{
    amp::{AndrAddr, Recipient},
    common::denom::Asset,
};
use cosmwasm_std::{coin, to_json_binary, Decimal, Uint128};
use cw_orch::prelude::*;
use cw_orch_daemon::{Daemon, DaemonBase, TxSender, Wallet};
use e2e::constants::{PION_1, RECIPIENT_MNEMONIC_1, RECIPIENT_MNEMONIC_2};

use andromeda_socket_astroport::SocketAstroportContract;

use rstest::{fixture, rstest};

struct TestCase {
    daemon: DaemonBase<Wallet>,
    app_contract: AppContract<DaemonBase<Wallet>>,
}

const TEST_MNEMONIC: &str = "cereal gossip fox peace youth leader engage move brass sell gas trap issue simple dance source develop black hurt pulp burst predict patient onion";

#[fixture]
fn setup(
    #[default(11766)] app_code_id: u64,
    #[default("neutron1zlwfu3wurn98zv3qe4cln0p4crwvfvjkn703vhhcajh6h3v00zzsdadsd8")]
    kernel_address: String,
) -> TestCase {
    let socket_astroport_type = "soekct-astroport";
    let socket_astroport_component_name = "socket-astroport";
    let app_name = "socket astroport with recipient";

    let daemon = Daemon::builder(PION_1)
        .mnemonic(TEST_MNEMONIC)
        .build()
        .unwrap();
    let app_contract = AppContract::new(daemon.clone());
    app_contract.set_code_id(app_code_id);

    // Prepare app components
    let socket_astroport_init_msg = InstantiateMsg {
        kernel_address: kernel_address.to_string(),
        owner: None,
        swap_router: None,
    };

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
        "splitter".to_string(),
        to_json_binary(&splitter_init_msg).unwrap(),
    );

    let app_components = vec![
        splitter_component.clone(),
        socket_astroport_component.clone(),
    ];

    app_contract
        .instantiate(
            &andromeda_app::app::InstantiateMsg {
                app_components,
                name: app_name.to_string(),
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
    }
}

#[rstest]
fn test_onchain_native(setup: TestCase) {
    let TestCase {
        daemon,
        app_contract,
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
    } = setup;

    let app_name = "socket astroport with recipient";
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
    } = setup;

    let app_name = "socket astroport with recipient";
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
