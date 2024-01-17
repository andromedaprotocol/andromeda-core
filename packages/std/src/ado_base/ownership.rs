use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;

#[cw_serde]
pub struct ContractOwnerResponse {
    pub owner: String,
}

#[cw_serde]
pub struct PublisherResponse {
    pub original_publisher: String,
}

#[cw_serde]
pub enum OwnershipMessage {
    UpdateOwner { new_owner: Addr },
    AcceptOwnership,
    Disown,
    UpdateOperators { new_operators: Vec<Addr> },
}
