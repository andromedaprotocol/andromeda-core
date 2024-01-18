use cosmwasm_schema::cw_serde;

#[cw_serde]
pub struct IsOperatorResponse {
    pub is_operator: bool,
}

#[cw_serde]
pub struct OperatorsResponse {
    pub operators: Vec<String>,
}
