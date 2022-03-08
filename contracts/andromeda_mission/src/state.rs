use andromeda_protocol::{
    communication::AndromedaMsg, error::ContractError, mission::MissionComponent,
};
use cosmwasm_std::{to_binary, Addr, Coin, CosmosMsg, Order, ReplyOn, Storage, SubMsg, WasmMsg};
use cw_storage_plus::{Bound, Item, Map};

/// Used to store the addresses of each ADO within the mission
pub const ADO_ADDRESSES: Map<&str, Addr> = Map::new("ado_addresses");
/// Stores a record of the describing structs for each ADO
pub const ADO_DESCRIPTORS: Map<&str, MissionComponent> = Map::new("ado_descriptors");
pub const ADO_IDX: Item<u64> = Item::new("ado_idx");

// DEV NOTE: Very similar to CW721 module instantiation, possibly merge both implementations?
pub fn add_mission_component(
    storage: &mut dyn Storage,
    component: &MissionComponent,
) -> Result<u64, ContractError> {
    let idx = match ADO_IDX.load(storage) {
        Ok(index) => index,
        Err(..) => 1u64,
    };
    let idx_str = idx.to_string();
    ADO_DESCRIPTORS.save(storage, &idx_str, &component)?;
    ADO_IDX.save(storage, &(idx + 1))?;

    Ok(idx)
}

pub fn load_component_addresses(storage: &dyn Storage) -> Result<Vec<Addr>, ContractError> {
    let min = Some(Bound::Inclusive(1u64.to_le_bytes().to_vec()));
    // let max = Some(Bound::Inclusive(1u64.to_le_bytes().to_vec()));
    let addresses: Vec<Addr> = ADO_ADDRESSES
        .range(storage, min, None, Order::Ascending)
        .flatten()
        .map(|(_vec, module)| module)
        .collect();

    Ok(addresses)
}

pub fn generate_ownership_message(addr: Addr, owner: &str) -> Result<SubMsg, ContractError> {
    let msg = to_binary(&AndromedaMsg::UpdateOwner {
        address: owner.to_string(),
    })?;
    Ok(SubMsg {
        id: 101,
        reply_on: ReplyOn::Error,
        msg: CosmosMsg::Wasm(WasmMsg::Execute {
            msg: msg.clone(),
            funds: Vec::<Coin>::new(),
            contract_addr: addr.to_string(),
        }),
        gas_limit: None,
    })
}
