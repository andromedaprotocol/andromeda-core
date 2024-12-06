use crate::contract::{execute, instantiate, query};
use andromeda_fungible_tokens::ics20::{ExecuteMsg, InitMsg, QueryMsg};
use cw_orch::{interface, prelude::*};
pub const CONTRACT_ID: &str = "ics20_contract";

#[interface(InitMsg, ExecuteMsg, QueryMsg, Empty id = CONTRACT_ID)]
pub struct ICS20Contract<Chain: CwEnv>;

// Implement the Uploadable trait so it can be uploaded to the mock.
impl<Chain> Uploadable for ICS20Contract<Chain> {
    fn wrapper() -> Box<dyn MockContract<Empty>> {
        Box::new(
            ContractWrapper::new_with_empty(execute, instantiate, query).with_ibc(
                crate::ibc::ibc_channel_open,
                crate::ibc::ibc_channel_connect,
                crate::ibc::ibc_channel_close,
                crate::ibc::ibc_packet_receive,
                crate::ibc::ibc_packet_ack,
                crate::ibc::ibc_packet_timeout,
            ),
        )
    }
}
