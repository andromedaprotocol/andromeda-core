use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;

#[cw_serde]
pub struct AppContractResponse {
    pub app_contract: Addr,
}
