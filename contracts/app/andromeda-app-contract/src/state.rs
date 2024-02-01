use andromeda_app::app::{
    AppComponent, ChainInfo, ComponentAddress, ComponentType, CrossChainComponent, InstantiateMsg,
};
use andromeda_std::{
    ado_base::AndromedaMsg, ado_contract::ADOContract, amp::AndrAddr, error::ContractError,
    os::aos_querier::AOSQuerier, os::kernel::ExecuteMsg as KernelExecuteMsg,
};
use cosmwasm_std::{
    to_binary, Addr, Coin, CosmosMsg, DepsMut, Order, ReplyOn, Storage, SubMsg, WasmMsg,
};
use cw_storage_plus::{Bound, Item, Map};

use crate::reply::ReplyId;

/// Used to store the addresses of each ADO within the app
pub const ADO_ADDRESSES: Map<&str, Addr> = Map::new("ado_addresses");
/// Stores a record of the describing structs for each ADO
pub const ADO_DESCRIPTORS: Map<&str, AppComponent> = Map::new("ado_descriptors");
pub const ADO_IDX: Item<u64> = Item::new("ado_idx");
pub const APP_NAME: Item<String> = Item::new("app_name");
// Used to keep track of which component indices have had the app assigned
pub const ASSIGNED_IDX: Item<u64> = Item::new("assigned_idx");

// DEV NOTE: Very similar to CW721 module instantiation, possibly merge both implementations?
pub fn add_app_component(
    storage: &mut dyn Storage,
    component: &AppComponent,
) -> Result<u64, ContractError> {
    let idx = ADO_IDX.may_load(storage)?.unwrap_or(1u64);
    ADO_DESCRIPTORS.save(storage, &idx.to_string(), component)?;
    ADO_IDX.save(storage, &(idx + 1))?;

    Ok(idx)
}

pub fn load_component_addresses(
    storage: &dyn Storage,
    min: Option<&str>,
) -> Result<Vec<Addr>, ContractError> {
    let min = Some(Bound::inclusive(min.unwrap_or("1")));
    let addresses: Vec<Addr> = ADO_ADDRESSES
        .range(storage, min, None, Order::Ascending)
        .flatten()
        .map(|(_vec, addr)| addr)
        .collect();

    Ok(addresses)
}

pub fn load_component_addresses_with_name(
    storage: &dyn Storage,
) -> Result<Vec<ComponentAddress>, ContractError> {
    let min = Some(Bound::inclusive("1"));
    let addresses: Vec<ComponentAddress> = ADO_ADDRESSES
        .range(storage, min, None, Order::Ascending)
        .flatten()
        .map(|(name, addr)| ComponentAddress {
            name,
            address: addr.to_string(),
        })
        .collect();

    Ok(addresses)
}

pub fn load_component_descriptors(
    storage: &dyn Storage,
) -> Result<Vec<AppComponent>, ContractError> {
    let min = Some(Bound::inclusive("1"));
    let descriptors: Vec<AppComponent> = ADO_DESCRIPTORS
        .range(storage, min, None, Order::Ascending)
        .flatten()
        .map(|(_vec, component)| component)
        .collect();

    Ok(descriptors)
}

pub fn generate_ownership_message(addr: Addr, owner: &str) -> Result<SubMsg, ContractError> {
    let msg = to_binary(&AndromedaMsg::UpdateOwner {
        address: owner.to_string(),
    })?;
    Ok(SubMsg {
        id: ReplyId::ClaimOwnership.repr(),
        reply_on: ReplyOn::Error,
        msg: CosmosMsg::Wasm(WasmMsg::Execute {
            msg,
            funds: Vec::<Coin>::new(),
            contract_addr: addr.to_string(),
        }),
        gas_limit: None,
    })
}

pub fn generate_assign_app_message(addr: &Addr, app_addr: &str) -> Result<SubMsg, ContractError> {
    let msg = to_binary(&AndromedaMsg::UpdateAppContract {
        address: app_addr.to_string(),
    })?;
    Ok(SubMsg {
        id: ReplyId::AssignApp.repr(),
        reply_on: ReplyOn::Error,
        msg: CosmosMsg::Wasm(WasmMsg::Execute {
            msg,
            funds: Vec::<Coin>::new(),
            contract_addr: addr.to_string(),
        }),
        gas_limit: None,
    })
}

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

/// Creates a sub message to create a recpliant app on the target chain
/// Apps are altered to be symlinks or instantiations depending on if they are for the target chain
pub fn create_cross_chain_message(
    deps: &DepsMut,
    app_name: String,
    owner: String,
    components: Vec<AppComponent>,
    chain_info: ChainInfo,
) -> Result<SubMsg, ContractError> {
    let kernel_address = ADOContract::default().get_kernel_address(deps.storage)?;
    let curr_chain = AOSQuerier::get_current_chain(&deps.querier, &kernel_address)?;
    let channel_info = AOSQuerier::get_chain_info(
        &deps.querier,
        &kernel_address,
        chain_info.chain_name.as_str(),
    )?;
    let mut new_components: Vec<AppComponent> = Vec::new();
    for component in components {
        let name = component.name;
        let new_component = match component.component_type {
            ComponentType::CrossChain(CrossChainComponent {
                chain,
                instantiate_msg,
            }) => {
                // If component for target chain instantiate component
                if chain == chain_info.chain_name {
                    AppComponent {
                        name,
                        ado_type: component.ado_type,
                        component_type: ComponentType::New(instantiate_msg),
                    }
                } else {
                    // Otherwise use a symlink to the component
                    AppComponent {
                        name: name.clone(),
                        ado_type: component.ado_type,
                        component_type: ComponentType::Symlink(AndrAddr::from_string(format!(
                            "ibc://{chain}/home/{owner}/{app_name}/{name}"
                        ))),
                    }
                }
            }
            // Must be some form of local component (symlink or new) so create symlink references
            _ => AppComponent {
                name: name.clone(),
                ado_type: component.ado_type,
                component_type: ComponentType::Symlink(AndrAddr::from_string(format!(
                    "ibc://{curr_chain}/home/{owner}/{app_name}/{name}"
                ))),
            },
        };
        new_components.push(new_component);
    }
    let msg = InstantiateMsg {
        owner: Some(chain_info.owner.clone()),
        app_components: new_components,
        name: app_name,
        chain_info: None,
        kernel_address: channel_info.kernel_address,
    };

    let kernel_msg = KernelExecuteMsg::Create {
        ado_type: "app-contract".to_string(),
        msg: to_binary(&msg)?,
        owner: Some(AndrAddr::from_string(chain_info.owner)),
        chain: Some(chain_info.chain_name),
    };

    let cosmos_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: kernel_address.to_string(),
        msg: to_binary(&kernel_msg)?,
        funds: vec![],
    });
    let sub_msg = SubMsg {
        id: ReplyId::CrossChainCreate.repr(),
        reply_on: ReplyOn::Error,
        msg: cosmos_msg,
        gas_limit: None,
    };

    Ok(sub_msg)
}
