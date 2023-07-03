use andromeda_std::{
    ado_contract::ADOContract, andr_exec, andr_instantiate, andr_query, error::ContractError,
    os::aos_querier::AOSQuerier,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Binary, Deps};

#[cw_serde]
pub struct AppComponent {
    pub name: String,
    pub ado_type: String,
    pub instantiate_msg: Binary,
}

impl AppComponent {
    pub fn new(
        name: impl Into<String>,
        ado_type: impl Into<String>,
        instantiate_msg: Binary,
    ) -> AppComponent {
        AppComponent {
            name: name.into(),
            ado_type: ado_type.into(),
            instantiate_msg,
        }
    }

    pub fn verify(&self, deps: &Deps) -> Result<(), ContractError> {
        if self.name.is_empty() {
            panic!("name cannot be empty");
        }
        if self.ado_type.is_empty() {
            panic!("ado_type cannot be empty");
        }
        if self.instantiate_msg.is_empty() || self.instantiate_msg == Binary::default() {
            panic!("instantiate_msg cannot be empty");
        }
        let adodb_addr = ADOContract::default()
            .get_adodb_address(deps.storage, &deps.querier)
            .unwrap();
        AOSQuerier::code_id_getter(&deps.querier, &adodb_addr, &self.ado_type).unwrap();
        Ok(())
    }
}

#[andr_instantiate("no_modules")]
#[cw_serde]
pub struct InstantiateMsg {
    pub app_components: Vec<AppComponent>,
    pub name: String,
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    AddAppComponent { component: AppComponent },
    ClaimOwnership { name: Option<String> },
    ProxyMessage { name: String, msg: Binary },
    UpdateAddress { name: String, addr: String },
}

#[cw_serde]
pub struct MigrateMsg {}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(String)]
    GetAddress { name: String },
    #[returns(AppComponent)]
    GetComponents {},
    #[returns(bool)]
    ComponentExists { name: String },
    #[returns(Vec<AppComponent>)]
    GetAddressesWithNames {},
    #[returns(ConfigResponse)]
    Config {},
}

#[cw_serde]
pub struct ConfigResponse {
    pub owner: String,
    pub name: String,
}

#[cw_serde]
pub struct ComponentAddress {
    pub name: String,
    pub address: String,
}

#[cfg(test)]
mod tests {
    // use super::*;
}
