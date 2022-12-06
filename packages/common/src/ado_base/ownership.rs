use cosmwasm_schema::cw_serde;

#[cw_serde]
pub struct ContractOwnerResponse {
    pub owner: String,
}

#[cw_serde]
pub struct PublisherResponse {
    pub original_publisher: String,
}
