use andromeda_non_fungible_tokens::cw721::TransferAgreement;
use andromeda_std::error::ContractError;
use cosmwasm_std::Storage;
use cw_storage_plus::Map;

pub const TRANSFER_AGREEMENTS: Map<&str, TransferAgreement> = Map::new("transfer_agreements");
pub const ARCHIVED: Map<&str, bool> = Map::new("archived_tokens");

pub const MINT_ACTION: &str = "can_mint";

pub fn is_archived(storage: &dyn Storage, token_id: &str) -> Result<bool, ContractError> {
    let archived_opt = ARCHIVED.may_load(storage, token_id)?.unwrap_or(false);
    Ok(archived_opt)
}
