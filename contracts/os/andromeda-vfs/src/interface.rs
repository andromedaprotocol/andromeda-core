use crate::contract::{execute, instantiate, query};
use andromeda_std::os::vfs::{ExecuteMsg, InstantiateMsg, QueryMsg};
use cw_orch::{interface, prelude::*};
pub const CONTRACT_ID: &str = "vfs_contract";

#[interface(InstantiateMsg, ExecuteMsg, QueryMsg, Empty, id = CONTRACT_ID)]
pub struct VFSContract<Chain: CwEnv>;

// Implement the Uploadable trait so it can be uploaded to the mock.
impl<Chain> Uploadable for VFSContract<Chain> {
    fn wrapper() -> Box<dyn MockContract<Empty>> {
        Box::new(ContractWrapper::new_with_empty(execute, instantiate, query))
    }
}
