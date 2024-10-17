use crate::contract::{execute, instantiate, query, reply};
use andromeda_std::os::economics::{ExecuteMsg, InstantiateMsg, QueryMsg};
use cw_orch::{interface, prelude::*};
pub const CONTRACT_ID: &str = "economics_contract";

#[interface(InstantiateMsg, ExecuteMsg, QueryMsg, Empty id = CONTRACT_ID)]
pub struct EconomicsContract<Chain: CwEnv>;

// Implement the Uploadable trait so it can be uploaded to the mock.
impl<Chain> Uploadable for EconomicsContract<Chain> {
    fn wrapper() -> Box<dyn MockContract<Empty>> {
        Box::new(
            ContractWrapper::new_with_empty(execute, instantiate, query)
                .with_reply(reply)
                .with_reply(crate::contract::reply),
        )
    }
}
