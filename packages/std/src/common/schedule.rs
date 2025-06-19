use crate::{
    common::{expiration::Expiry, Milliseconds},
    error::ContractError,
};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{ensure, BlockInfo};

#[cw_serde]
#[derive(Default)]
pub struct Schedule {
    pub start: Option<Expiry>,
    pub end: Option<Expiry>,
}

impl Schedule {
    pub fn validate(
        &self,
        block: &BlockInfo,
    ) -> Result<(Milliseconds, Option<Milliseconds>), ContractError> {
        let start_time = match &self.start {
            Some(s) => {
                // Check that the start time is in the future
                s.validate(block)?.get_time(block)
            }
            // Set start time to current time if not provided
            None => Expiry::FromNow(Milliseconds::zero()).get_time(block),
        };

        let end_time = match &self.end {
            Some(limit) => {
                let end_time = limit.get_end_time(start_time);
                // Start time has already been validated, so no need to check if the end time is in the past
                if let Some(end_time) = end_time {
                    ensure!(
                        end_time > start_time,
                        ContractError::InvalidSchedule {
                            msg: "End time must be after start time".to_string(),
                        }
                    );
                }
                end_time
            }
            None => None,
        };

        Ok((start_time, end_time))
    }
    pub fn new(start: Option<Expiry>, end: Option<Expiry>) -> Self {
        Self { start, end }
    }
}
