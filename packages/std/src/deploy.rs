#[allow(clippy::crate_in_macro_def)]
#[macro_export]
macro_rules! contract_interface {
    ($contract_name:ident, $contract_id:expr, $wasm_path:expr) => {
        use cw_orch::interface;
        use cw_orch::prelude::*;

        #[interface(InstantiateMsg, ExecuteMsg,QueryMsg, MigrateMsg, id = $contract_id)]
        pub struct $contract_name;

        impl<Chain> Uploadable for $contract_name<Chain> {
            fn wrapper() -> Box<dyn MockContract<Empty>> {
                Box::new(
                    ContractWrapper::new_with_empty(
                        crate::contract::execute,
                        crate::contract::instantiate,
                        crate::contract::query,
                    )
                    .with_reply(crate::contract::reply)
                    .with_migrate(crate::contract::migrate),
                )
            }

            fn wasm(_chain: &ChainInfoOwned) -> WasmPath {
                artifacts_dir_from_workspace!()
                    .find_wasm_path($wasm_path)
                    .unwrap()
            }
        }

        impl<Chain> ADOMetadata for $contract_name<Chain> {
            fn name() -> String {
                $contract_id.to_string()
            }

            fn version() -> String {
                let version = env!("CARGO_PKG_VERSION");
                version.to_string()
            }
        }
    };
}

pub trait ADOMetadata {
    fn name() -> String;
    fn version() -> String;
}
