use crate::messages::AMPPkt;
use common::ado_base::AndromedaQuery;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary, ReplyOn};

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    /// Receives an AMP Packet for relaying
    AMPReceive(AMPPkt),
    /// Creates an original AMP packet
    AMPDirect {
        recipient: String,
        message: Binary,
        reply_on: Option<ReplyOn>,
        exit_at_error: Option<bool>,
        gas_limit: Option<u64>,
    },
    /// Upserts a key address to the kernel, restricted to the owner of the kernel
    UpsertKeyAddress { key: String, value: String },
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(AndromedaQuery)]
    AndrQuery(AndromedaQuery),
    #[returns(Addr)]
    KeyAddress { key: String },
    #[returns(bool)]
    VerifyAddress { address: String },
}

// turns ibc://juno/path into /path
pub fn adjust_recipient_with_protocol(recipient: &str) -> String {
    let mut count_slashes = 0;
    let mut last_slash_index = 0;

    // Iterate through each character in the input string
    for (i, c) in recipient.chars().enumerate() {
        // If the current character is a slash
        if c == '/' {
            count_slashes += 1;
            last_slash_index = i;

            // If we've found the third slash, exit the loop
            if count_slashes == 3 {
                break;
            }
        }
    }

    // Return the substring starting from the last slash index
    recipient[last_slash_index..].to_owned()
}
