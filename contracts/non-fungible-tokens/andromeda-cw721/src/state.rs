use andromeda_non_fungible_tokens::cw721::{IsArchivedResponse, TransferAgreement};
use andromeda_std::{amp::AndrAddr, error::ContractError};
use cosmwasm_std::Storage;
use cw_storage_plus::{Item, Map};

pub const ANDR_MINTER: Item<AndrAddr> = Item::new("minter");
pub const TRANSFER_AGREEMENTS: Map<&str, TransferAgreement> = Map::new("transfer_agreements");
pub const ARCHIVED: Map<&str, bool> = Map::new("archived_tokens");

pub const MINT_ACTION: &str = "can_mint";
pub const BATCH_MINT_ACTION: &str = "can_batch_mint";

pub fn is_archived(
    storage: &dyn Storage,
    token_id: &str,
) -> Result<IsArchivedResponse, ContractError> {
    let archived_opt = ARCHIVED.may_load(storage, token_id)?.unwrap_or(false);
    Ok(IsArchivedResponse {
        is_archived: archived_opt,
    })
}
