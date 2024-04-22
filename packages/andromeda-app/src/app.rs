use andromeda_std::{
    amp::AndrAddr,
    andr_exec, andr_instantiate, andr_query,
    common::reply::ReplyId,
    error::ContractError,
    os::{
        aos_querier::AOSQuerier,
        vfs::{convert_component_name, ExecuteMsg as VFSExecuteMsg},
    },
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    attr, ensure, instantiate2_address, to_json_binary, wasm_execute, Addr, Api, Binary,
    CodeInfoResponse, Deps, Event, QuerierWrapper, SubMsg, WasmMsg,
};
use serde::Serialize;

pub fn get_chain_info(chain_name: String, chain_info: Option<Vec<ChainInfo>>) -> Option<ChainInfo> {
    match chain_info {
        Some(chain_info) => {
            let idx = chain_info
                .iter()
                .position(|info| info.chain_name == chain_name)?;
            Some(chain_info[idx].clone())
        }
        None => None,
    }
}

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

    pub fn symlink(
        name: impl Into<String>,
        ado_type: impl Into<String>,
        symlink: impl Into<String>,
    ) -> AppComponent {
        AppComponent {
            ado_type: ado_type.into(),
            name: name.into(),
            component_type: ComponentType::Symlink(AndrAddr::from_string(symlink.into())),
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

    /// Generates an `Instantiate2` address for the component.
    ///
    /// Returns `None` for `Symlink` and `CrossChain` components.
    pub fn get_new_addr(
        &self,
        api: &dyn Api,
        adodb_addr: &Addr,
        querier: &QuerierWrapper,
        parent_addr: Addr,
    ) -> Result<Option<Addr>, ContractError> {
        if !matches!(self.component_type, ComponentType::New(..)) {
            return Ok(None);
        }

        let code_id = AOSQuerier::code_id_getter(querier, adodb_addr, &self.ado_type)?;
        let CodeInfoResponse { checksum, .. } = querier.query_wasm_code_info(code_id)?;

        let salt = self.get_salt(parent_addr.clone());
        let creator = api.addr_canonicalize(parent_addr.as_str())?;
        let new_addr = instantiate2_address(&checksum, &creator, &salt).unwrap();

        Ok(Some(api.addr_humanize(&new_addr)?))
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

    /// Generates a VFS registration message for the component.
    pub fn generate_vfs_registration(
        &self,
        // New addr is provided to prevent duplicate queries
        new_addr: Option<Addr>,
        _app_addr: &Addr,
        app_name: &str,
        chain_info: Option<Vec<ChainInfo>>,
        _adodb_addr: &Addr,
        vfs_addr: &Addr,
    ) -> Result<Option<SubMsg>, ContractError> {
        if self.name.starts_with('.') {
            return Ok(None);
        }
        match self.component_type.clone() {
            ComponentType::New(_) => {
                let new_addr = new_addr.unwrap();
                let register_msg = wasm_execute(
                    vfs_addr.clone(),
                    &VFSExecuteMsg::AddPath {
                        name: convert_component_name(&self.name),
                        address: new_addr,
                        parent_address: None,
                    },
                    vec![],
                )?;
                let register_submsg =
                    SubMsg::reply_always(register_msg, ReplyId::RegisterPath.repr());

                Ok(Some(register_submsg))
            }
            ComponentType::Symlink(symlink) => {
                let msg = VFSExecuteMsg::AddSymlink {
                    name: self.name.clone(),
                    symlink,
                    parent_address: None,
                };
                let cosmos_msg = wasm_execute(vfs_addr, &msg, vec![])?;
                let sub_msg = SubMsg::reply_on_error(cosmos_msg, ReplyId::RegisterPath.repr());
                Ok(Some(sub_msg))
            }
            ComponentType::CrossChain(CrossChainComponent { chain, .. }) => {
                let curr_chain_info = get_chain_info(chain.clone(), chain_info.clone());
                ensure!(
                    curr_chain_info.is_some(),
                    ContractError::InvalidComponent {
                        name: self.name.clone()
                    }
                );
                let owner_addr = curr_chain_info.unwrap().owner;
                let name = self.name.clone();

                // Register the component as a symlink for the receiving chain
                let new_component = AppComponent {
                    name: name.clone(),
                    ado_type: self.ado_type.clone(),
                    component_type: ComponentType::Symlink(AndrAddr::from_string(format!(
                        "ibc://{chain}/home/{owner_addr}/{app_name}/{name}"
                    ))),
                };
                new_component.generate_vfs_registration(
                    new_addr,
                    _app_addr,
                    app_name,
                    chain_info,
                    _adodb_addr,
                    vfs_addr,
                )
            }
        }
    }

    /// Generates an instantiation message for the component.
    ///
    /// Returns `None` for `Symlink` and `CrossChain` components.
    pub fn generate_instantiation_message(
        &self,
        querier: &QuerierWrapper,
        adodb_addr: &Addr,
        parent_addr: &Addr,
        sender: &str,
        idx: u64,
    ) -> Result<Option<SubMsg>, ContractError> {
        if let ComponentType::New(instantiate_msg) = self.component_type.clone() {
            let code_id = AOSQuerier::code_id_getter(querier, adodb_addr, &self.ado_type)?;
            let salt = self.get_salt(parent_addr.clone());
            let inst_msg = WasmMsg::Instantiate2 {
                admin: Some(sender.to_string()),
                code_id,
                label: format!("Instantiate: {}", self.ado_type),
                msg: instantiate_msg,
                funds: vec![],
                salt,
            };
            Ok(Some(SubMsg::reply_always(inst_msg, idx)))
        } else {
            Ok(None)
        }
    }

    /// Generates an event for the app component.
    ///
    /// Includes the name and type of the component plus the following for each type:
    ///  - `New` - the address of the component
    ///  - `CrossChain` - the receiving chain of the component
    ///  - `Symlink` - the created symlink for the component
    pub fn generate_event(&self, addr: Option<Addr>) -> Event {
        let mut ev = Event::new("add_app_component").add_attributes(vec![
            attr("name", self.name.clone()),
            attr("ado_type", self.ado_type.clone()),
        ]);

        match self.component_type.clone() {
            ComponentType::New(_) => {
                ev = ev.add_attribute("address", addr.unwrap().to_string());
            }
            ComponentType::CrossChain(chain_component) => {
                ev = ev.add_attribute("chain", chain_component.chain);
            }
            ComponentType::Symlink(link) => ev = ev.add_attribute("symlink", link),
        }

        ev
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

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(String)]
    GetAddress { name: String },
    #[returns(AppComponent)]
    GetComponents {},
    #[returns(ComponentExistsResponse)]
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
pub struct ComponentExistsResponse {
    pub component_exists: bool,
}

#[cw_serde]
pub struct ComponentAddress {
    pub name: String,
    pub address: String,
}
