use crate::contract::{execute, instantiate, query};
use andromeda_finance::splitter::{ExecuteMsg, InstantiateMsg, QueryMsg};
use cw_orch::{interface, prelude::*};
pub const CONTRACT_ID: &str = "splitter_contract";

#[interface(InstantiateMsg, ExecuteMsg, QueryMsg, Empty, id = CONTRACT_ID)]
pub struct SplitterContract<Chain: CwEnv>;

// Implement the Uploadable trait so it can be uploaded to the mock.
impl<Chain> Uploadable for SplitterContract<Chain> {
    fn wrapper() -> Box<dyn MockContract<Empty>> {
        Box::new(ContractWrapper::new_with_empty(execute, instantiate, query))
    }
}
