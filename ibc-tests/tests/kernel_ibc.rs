use andromeda_app::app::InstantiateMsg as AppInstantiateMsg;
use andromeda_std::amp::messages::AMPMsg;
use andromeda_std::os::kernel::*;
use andromeda_testing_e2e::chains::LOCAL_OSMO;
use andromeda_testing_e2e::chains::LOCAL_TERRA;
use andromeda_testing_e2e::mock::setup_interchain_env;
use andromeda_testing_e2e::mock::MockAndromeda;
use cosmwasm_std::to_json_binary;
use cw_orch::prelude::*;
use cw_orch_interchain::prelude::*;
use ibc_tests::config::Config;

#[test]
fn test_basic_ibc() {
    env_logger::init();
    let env = setup_interchain_env();
    let osmo = env.get_chain(LOCAL_OSMO.chain_id).unwrap();
    let terra = env.get_chain(LOCAL_TERRA.chain_id).unwrap();
    let config = Config::load();

    let osmo_aos = MockAndromeda::new(
        &osmo,
        config.get_installation(LOCAL_OSMO.network_info.chain_name),
    );
    let MockAndromeda {
        kernel_contract, ..
    } = osmo_aos;

    let terra_aos = MockAndromeda::new(
        &terra,
        config.get_installation(LOCAL_TERRA.network_info.chain_name),
    );
    let MockAndromeda {
        kernel_contract: terra_kernel_contract,
        ..
    } = terra_aos;
    kernel_contract
        .execute(
            &ExecuteMsg::AssignChannels {
                ics20_channel_id: Some("channel-0".to_string()),
                direct_channel_id: Some("channel-1".to_string()),
                chain: "terra".to_string(),
                kernel_address: terra_kernel_contract.address().unwrap().to_string(),
            },
            None,
        )
        .unwrap();

    terra_kernel_contract
        .execute(
            &ExecuteMsg::AssignChannels {
                ics20_channel_id: Some("channel-0".to_string()),
                direct_channel_id: Some("channel-3".to_string()),
                chain: "osmo".to_string(),
                kernel_address: kernel_contract.address().unwrap().to_string(),
            },
            None,
        )
        .unwrap();

    let res = kernel_contract
        .execute(
            &ExecuteMsg::Send {
                message: AMPMsg::new(
                    format!("ibc://terra/{}", terra_kernel_contract.address().unwrap()),
                    to_json_binary(&ExecuteMsg::Create {
                        ado_type: "app-contract".to_string(),
                        owner: None,
                        chain: None,
                        msg: to_json_binary(&AppInstantiateMsg {
                            app_components: vec![],
                            name: "ibc-app".to_string(),
                            chain_info: None,
                            kernel_address: terra_kernel_contract.address().unwrap().to_string(),
                            owner: None,
                        })
                        .unwrap(),
                    })
                    .unwrap(),
                    None,
                ),
            },
            None,
        )
        .unwrap();
    // let res = kernel_contract
    //     .execute(
    //         &ExecuteMsg::Send {
    //             message: AMPMsg::new(
    //                 format!("ibc://terra/{}", terra_kernel_contract.address().unwrap()),
    //                 to_json_binary(&ExecuteMsg::Create {
    //                     ado_type: "app-contract".to_string(),
    //                     owner: None,
    //                     chain: None,
    //                     msg: Binary::default(),
    //                 })
    //                 .unwrap(),
    //                 None,
    //             ),
    //         },
    //         Some(&[coin(100, "uosmo")]),
    //     )
    //     .unwrap();
    env.await_and_check_packets(&osmo.chain_id(), res).unwrap();
}
