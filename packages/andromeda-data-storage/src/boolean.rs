use andromeda_std::{amp::AndrAddr, andr_exec, andr_instantiate, andr_query};
use cosmwasm_schema::{cw_serde, QueryResponses};

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    pub restriction: BooleanRestriction,
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    SetValue { value: bool },
    DeleteValue {},
    UpdateRestriction { restriction: BooleanRestriction },
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GetValueResponse)]
    GetValue {},
    #[returns(GetDataOwnerResponse)]
    GetDataOwner {},
}

#[cw_serde]
pub enum BooleanRestriction {
    Private,
    Public,
    Restricted,
}

#[cw_serde]
pub struct GetValueResponse {
    pub value: bool,
}

#[cw_serde]
pub struct GetDataOwnerResponse {
    pub owner: AndrAddr,
}
