use cosmwasm_schema::cw_serde;

#[cw_serde]
pub struct BlockHeightResponse {
    pub block_height: u64,
}
