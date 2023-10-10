use andromeda_std::{
    ado_contract::ADOContract, amp::AndrAddr, andr_exec, andr_instantiate, andr_query,
    error::ContractError, os::aos_querier::AOSQuerier,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_binary, Addr, Binary, Deps};
use serde::Serialize;

#[cw_serde]
pub struct CrossChainComponent {
    pub instantiate_msg: Binary,
    pub chain: String,
}

#[cw_serde]
pub enum ComponentType {
    New(Binary),
    Symlink(AndrAddr),
    CrossChain(CrossChainComponent),
}

impl Default for ComponentType {
    fn default() -> Self {
        ComponentType::New(Binary::default())
    }
}

impl ComponentType {
    pub fn verify(&self) -> Result<(), ContractError> {
        match self {
            ComponentType::New(msg) => {
                if msg.is_empty() {
                    panic!("instantiate_msg cannot be empty");
                }
            }
            ComponentType::Symlink(_) => {}
            ComponentType::CrossChain(cross_chain) => {
                if cross_chain.chain.is_empty() {
                    panic!("chain cannot be empty");
                }
                if cross_chain.instantiate_msg.is_empty() {
                    panic!("instantiate_msg cannot be empty");
                }
            }
        }
        Ok(())
    }

    pub fn new(msg: impl Serialize) -> ComponentType {
        ComponentType::New(to_binary(&msg).unwrap())
    }
}

#[cw_serde]
pub struct AppComponent {
    pub name: String,
    pub ado_type: String,
    pub component_type: ComponentType,
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
            component_type: ComponentType::New(instantiate_msg),
        }
    }

    pub fn verify(&self, deps: &Deps) -> Result<(), ContractError> {
        if self.name.is_empty() {
            panic!("name cannot be empty");
        }
        if self.ado_type.is_empty() {
            panic!("ado_type cannot be empty");
        }
        self.component_type.verify()?;
        let adodb_addr = ADOContract::default()
            .get_adodb_address(deps.storage, &deps.querier)
            .unwrap();
        AOSQuerier::code_id_getter(&deps.querier, &adodb_addr, &self.ado_type).unwrap();
        Ok(())
    }
}

#[cw_serde]
pub struct ChainInfo {
    pub chain_name: String,
    pub owner: String,
}

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    pub app_components: Vec<AppComponent>,
    pub name: String,
    pub chain_info: Option<Vec<ChainInfo>>,
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    AddAppComponent {
        component: AppComponent,
    },
    ClaimOwnership {
        name: Option<String>,
        new_owner: Option<Addr>,
    },
    ProxyMessage {
        name: String,
        msg: Binary,
    },
    UpdateAddress {
        name: String,
        addr: String,
    },
    // Only available to the app contract itself
    AssignAppToComponents {},
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
