use andromeda_std::{amp::recipient::Recipient, andr_exec, andr_instantiate, andr_query};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use cw_utils::Expiration;

#[cw_serde]
pub struct AddressWeight {
    pub recipient: Recipient,
    pub weight: Uint128,
}

#[cw_serde]
/// A config struct for a `Splitter` contract.
pub struct Splitter {
    /// The vector of recipients for the contract. Anytime a `Send` execute message is sent the amount sent will be divided amongst these recipients depending on their assigned weight.
    pub recipients: Vec<AddressWeight>,
    /// Whether or not the contract is currently locked. This restricts updating any config related fields.
    pub lock: Expiration,
}

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    /// The vector of recipients for the contract. Anytime a `Send` execute message is
    /// sent the amount sent will be divided amongst these recipients depending on their assigned weight.
    pub recipients: Vec<AddressWeight>,
    pub lock_time: Option<u64>,
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    /// Update the recipients list. Only executable by the contract owner when the contract is not locked.
    UpdateRecipients { recipients: Vec<AddressWeight> },
    /// Update a specific recipient's weight. Only executable by the contract owner when the contract is not locked.
    UpdateRecipientWeight { recipient: AddressWeight },
    /// Add a single recipient to the recipient list. Only executable by the contract owner when the contract is not locked.
    AddRecipient { recipient: AddressWeight },
    /// Remove a single recipient from the recipient list. Only executable by the contract owner when the contract is not locked.
    RemoveRecipient { recipient: Recipient },
    /// Used to lock/unlock the contract allowing the config to be updated.
    UpdateLock { lock_time: u64 },
    /// Divides any attached funds to the message amongst the recipients list.
    Send {},
}

#[cw_serde]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// The current config of the Splitter contract
    #[returns(GetSplitterConfigResponse)]
    GetSplitterConfig {},
    /// Gets user's allocated weight
    #[returns(GetUserWeightResponse)]
    GetUserWeight { user: Recipient },
}

#[cw_serde]
pub struct GetSplitterConfigResponse {
    pub config: Splitter,
}
/// In addition to returning a specific recipient's weight, this function also returns the total weight of all recipients.
/// This serves to put the user's weight into perspective.
#[cw_serde]
pub struct GetUserWeightResponse {
    pub weight: Uint128,
    pub total_weight: Uint128,
}
