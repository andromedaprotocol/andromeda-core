#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub mirror_mint_contract: String,
    pub mirror_staking_contract: String,
    pub mirror_gov_contract: String,
    pub mirror_lock_contract: String,
    pub operators: Option<Vec<String>>,
}
