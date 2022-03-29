use andromeda_protocol::mission::MissionComponent;
use common::{ado_base::AndromedaMsg, error::ContractError};
use cosmwasm_std::{to_binary, Addr, Coin, CosmosMsg, Order, ReplyOn, Storage, SubMsg, WasmMsg};
use cw_storage_plus::{Bound, Item, Map};

/// Used to store the addresses of each ADO within the mission
pub const ADO_ADDRESSES: Map<&str, Addr> = Map::new("ado_addresses");
/// Stores a record of the describing structs for each ADO
pub const ADO_DESCRIPTORS: Map<&str, MissionComponent> = Map::new("ado_descriptors");
pub const ADO_IDX: Item<u64> = Item::new("ado_idx");
pub const MISSION_NAME: Item<String> = Item::new("mission_name");

// DEV NOTE: Very similar to CW721 module instantiation, possibly merge both implementations?
pub fn add_mission_component(
    storage: &mut dyn Storage,
    component: &MissionComponent,
) -> Result<u64, ContractError> {
    let idx = ADO_IDX.may_load(storage)?.unwrap_or(1u64);
    let idx_str = idx.to_string();
    ADO_DESCRIPTORS.save(storage, &idx_str, component)?;
    ADO_IDX.save(storage, &(idx + 1))?;

    Ok(idx)
}

pub fn load_component_addresses(storage: &dyn Storage) -> Result<Vec<Addr>, ContractError> {
    let min = Some(Bound::Inclusive(1u64.to_le_bytes().to_vec()));
    let addresses: Vec<Addr> = ADO_ADDRESSES
        .range(storage, min, None, Order::Ascending)
        .flatten()
        .map(|(_vec, addr)| addr)
        .collect();

    Ok(addresses)
}

pub fn load_component_descriptors(
    storage: &dyn Storage,
) -> Result<Vec<MissionComponent>, ContractError> {
    let min = Some(Bound::Inclusive(1u64.to_le_bytes().to_vec()));
    let descriptors: Vec<MissionComponent> = ADO_DESCRIPTORS
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
        id: 101,
        reply_on: ReplyOn::Error,
        msg: CosmosMsg::Wasm(WasmMsg::Execute {
            msg,
            funds: Vec::<Coin>::new(),
            contract_addr: addr.to_string(),
        }),
        gas_limit: None,
    })
}

pub fn generate_assign_mission_message(
    addr: &Addr,
    mission_addr: &str,
) -> Result<SubMsg, ContractError> {
    let msg = to_binary(&AndromedaMsg::UpdateMissionContract {
        address: mission_addr.to_string(),
    })?;
    Ok(SubMsg {
        id: 103,
        reply_on: ReplyOn::Error,
        msg: CosmosMsg::Wasm(WasmMsg::Execute {
            msg,
            funds: Vec::<Coin>::new(),
            contract_addr: addr.to_string(),
        }),
        gas_limit: None,
    })
}
