use andromeda_std::{amp::AndrAddr, andr_exec, andr_instantiate, andr_query, error::ContractError};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{instantiate2_address, to_json_binary, Addr, Api, Binary, Deps, HexBinary};
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
    #[serde(skip)]
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
        ComponentType::New(to_json_binary(&msg).unwrap())
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

    pub fn verify(&self, _deps: &Deps) -> Result<(), ContractError> {
        if self.name.is_empty() {
            panic!("name cannot be empty");
        }
        if self.ado_type.is_empty() {
            panic!("ado_type cannot be empty");
        }
        self.component_type.verify()?;
        Ok(())
    }

    #[inline]
    pub fn get_salt(&self, _parent_addr: Addr) -> Binary {
        Binary::from(self.name.as_bytes())
    }

    pub fn get_new_addr(
        &self,
        checksum: HexBinary,
        parent_addr: Addr,
        api: &dyn Api,
    ) -> Result<Addr, ContractError> {
        let salt = self.get_salt(parent_addr.clone());
        let creator = api.addr_canonicalize(parent_addr.as_str())?;
        let new_addr = instantiate2_address(&checksum, &creator, &salt).unwrap();

        Ok(api.addr_humanize(&new_addr)?)
    }

    #[inline]
    pub fn get_msg_binary(&self) -> Result<Binary, ContractError> {
        match self.component_type.clone() {
            ComponentType::New(msg) => Ok(msg),
            _ => Err(ContractError::InvalidComponent {
                name: self.name.clone(),
            }),
        }
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
        new_owner: Option<AndrAddr>,
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
    use andromeda_std::testing::mock_querier::MOCK_APP_CONTRACT;
    use cw_multi_test::MockApiBech32;

    use super::*;

    #[test]
    fn test_get_new_addr() {
        let api = MockApiBech32::new("andr");
        let component = AppComponent {
            name: "test".to_string(),
            ado_type: "app-contract".to_string(),
            component_type: ComponentType::New(Binary::from("0".as_bytes())),
        };
        let checksum =
            HexBinary::from_hex("9af782a3a1bcbcd22dbb6a45c751551d9af782a3a1bcbcd22dbb6a45c751551d")
                .unwrap();

        let new_addr = component
            .get_new_addr(checksum, api.addr_make(MOCK_APP_CONTRACT), &api)
            .unwrap();

        println!("{:?}", new_addr);
    }
}
