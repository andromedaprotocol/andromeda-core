use andromeda_std::{
    ado_base::MigrateMsg,
    deploy::ADOMetadata,
    os::kernel::{ExecuteMsg, InstantiateMsg, QueryMsg},
};
use cw_orch::{interface, prelude::*};

pub const CONTRACT_ID: &str = "kernel";

#[interface(InstantiateMsg, ExecuteMsg, QueryMsg, MigrateMsg, id = CONTRACT_ID)]
pub struct KernelContract;

impl<Chain> Uploadable for KernelContract<Chain> {
    fn wrapper() -> Box<dyn MockContract<Empty>> {
        Box::new(
            ContractWrapper::new_with_empty(
                crate::contract::execute,
                crate::contract::instantiate,
                crate::contract::query,
            )
            .with_reply(crate::contract::reply)
            .with_migrate(crate::contract::migrate)
            .with_ibc(
                crate::ibc::ibc_channel_open,
                crate::ibc::ibc_channel_connect,
                crate::ibc::ibc_channel_close,
                crate::ibc::ibc_packet_receive,
                crate::ibc::ibc_packet_ack,
                crate::ibc::ibc_packet_timeout,
            ),
        )
    }

    fn wasm(_chain: &ChainInfoOwned) -> WasmPath {
        artifacts_dir_from_workspace!()
            .find_wasm_path("andromeda_kernel.wasm")
            .unwrap()
    }
}

impl<Chain> ADOMetadata for KernelContract<Chain> {
    fn name() -> String {
        CONTRACT_ID.to_string()
    }

    fn version() -> String {
        let version = env!("CARGO_PKG_VERSION");
        version.to_string()
    }
}
