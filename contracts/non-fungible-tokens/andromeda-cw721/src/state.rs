use andromeda_non_fungible_tokens::cw721::TransferAgreement;
use common::{app::AndrAddress, error::ContractError};
use cosmwasm_std::Storage;
use cw_storage_plus::{Item, Map};

// Key must not be "minter" as that is reserved by cw721_base contract.
pub const ANDR_MINTER: Item<AndrAddress> = Item::new("andr_minter");

pub const TRANSFER_AGREEMENTS: Map<String, TransferAgreement> = Map::new("transfer_agreements");
pub const ARCHIVED: Map<String, bool> = Map::new("archived_tokens");

pub fn is_archived(storage: &dyn Storage, token_id: &str) -> Result<bool, ContractError> {
    let archived_opt = ARCHIVED
        .may_load(storage, token_id.to_string())?
        .unwrap_or(false);
    Ok(archived_opt)
}
