use andromeda_data_storage::form::SubmissionInfo;
use andromeda_std::{amp::AndrAddr, common::MillisecondsExpiration};
use cosmwasm_std::{Addr, Uint64};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex};

pub const SCHEMA_ADO_ADDRESS: Item<AndrAddr> = Item::new("schema_ado_address");
pub const START_TIME: Item<Option<MillisecondsExpiration>> = Item::new("start_time");
pub const END_TIME: Item<Option<MillisecondsExpiration>> = Item::new("end_time");

pub const ALLOW_MULTIPLE_SUBMISSIONS: Item<bool> = Item::new("allow_multiple_submissions");
pub const ALLOW_EDIT_SUBMISSION: Item<bool> = Item::new("allow_edit_submission");

pub const SUBMISSION_ID: Item<Uint64> = Item::new("submission_id");

pub struct SubmissionIndexes<'a> {
    /// PK: submission_id + wallet_address
    /// Secondary key: submission_id
    pub submission_id: MultiIndex<'a, u64, SubmissionInfo, (u64, Addr)>,

    /// PK: submission_id + wallet_address
    /// Secondary key: wallet_address
    pub wallet_address: MultiIndex<'a, Addr, SubmissionInfo, (u64, Addr)>,
}

impl<'a> IndexList<SubmissionInfo> for SubmissionIndexes<'a> {
    fn get_indexes(
        &'_ self,
    ) -> Box<dyn Iterator<Item = &'_ dyn cw_storage_plus::Index<SubmissionInfo>> + '_> {
        let v: Vec<&dyn Index<SubmissionInfo>> = vec![&self.submission_id, &self.wallet_address];
        Box::new(v.into_iter())
    }
}

pub fn submissions<'a>() -> IndexedMap<'a, &'a (u64, Addr), SubmissionInfo, SubmissionIndexes<'a>> {
    let indexes = SubmissionIndexes {
        submission_id: MultiIndex::new(
            |_pk: &[u8], r| r.submission_id,
            "submission",
            "submission_id_index",
        ),
        wallet_address: MultiIndex::new(
            |_pk: &[u8], r| r.wallet_address.clone(),
            "submission",
            "wallet_address_index",
        ),
    };
    IndexedMap::new("submission", indexes)
}