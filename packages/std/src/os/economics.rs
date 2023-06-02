use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

use crate::amp::AndrAddr;

#[cw_serde]
pub struct InstantiateMsg {
    /// Address of the Kernel contract on chain
    pub kernel_address: String,
    pub owner: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    Deposit { address: Option<AndrAddr> },
    PayFee { payee: Addr, action: String },
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}

#[cfg(test)]
mod test {
    
}
