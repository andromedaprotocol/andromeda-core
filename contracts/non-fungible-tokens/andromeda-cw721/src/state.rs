use andromeda_std::amp::AndrAddr;
use cw_storage_plus::Item;

pub const ANDR_MINTER: Item<AndrAddr> = Item::new("minter");
// pub const TRANSFER_AGREEMENTS: Map<&str, TransferAgreement> = Map::new("transfer_agreements");
// pub const ARCHIVED: Map<&str, bool> = Map::new("archived_tokens");

// pub fn is_archived(
//     storage: &dyn Storage,
//     token_id: &str,
// ) -> Result<IsArchivedResponse, ContractError> {
//     let archived_opt = ARCHIVED.may_load(storage, token_id)?.unwrap_or(false);
//     Ok(IsArchivedResponse {
//         is_archived: archived_opt,
//     })
// }
