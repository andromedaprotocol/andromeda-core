use andromeda_std::ado_base::MigrateMsg;
use andromeda_std::os::kernel;
use andromeda_std::os::kernel::ExecuteMsg;
use andromeda_testing_e2e::chains::LOCAL_OSMO;
use andromeda_testing_e2e::mock::mock_app;
use andromeda_testing_e2e::mock::MockAndromeda;
use cw_orch::interface;
use cw_orch::prelude::*;
use ibc_tests::config::Config;
use ibc_tests::constants::TESTNET_MNEMONIC;
use ibc_tests::contract_interface;

const CHAIN: ChainInfo = LOCAL_OSMO;

contract_interface!(
    KernelContract,
    andromeda_kernel,
    kernel,
    "kernel",
    "andromeda_kernel@1.1.1"
);

#[test]
fn test_basic_ibc() {
    env_logger::init();
    let daemon = mock_app(CHAIN, TESTNET_MNEMONIC);
    let config = Config::load();

    let mock_andr = MockAndromeda::new(
        &daemon,
        config.get_installation(CHAIN.network_info.chain_name),
    );

    let MockAndromeda {
        kernel_contract, ..
    } = mock_andr;

    kernel_contract
        .execute(
            &ExecuteMsg::AssignChannels {
                ics20_channel_id: Some("channel-0".to_string()),
                direct_channel_id: Some("channel-1".to_string()),
                chain: "andr".to_string(),
                kernel_address: "andr123gcwlskafnxw4lvkjeacmd5tqzl27gd8xxxnrghe2qhdxtluskq8mrva5"
                    .to_string(),
            },
            None,
        )
        .unwrap();
}
