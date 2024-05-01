use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;

use crate::common::{expiration::Expiry, MillisecondsExpiration};

#[cw_serde]
pub struct ContractOwnerResponse {
    pub owner: String,
}

#[cw_serde]
pub struct ContractPotentialOwnerResponse {
    pub potential_owner: Option<Addr>,
    pub expiration: Option<MillisecondsExpiration>,
}

#[cw_serde]
pub struct PublisherResponse {
    pub original_publisher: String,
}

#[cw_serde]
pub enum OwnershipMessage {
    UpdateOwner {
        new_owner: Addr,
        expiration: Option<Expiry>,
    },
    RevokeOwnershipOffer,
    AcceptOwnership,
    Disown,
}
