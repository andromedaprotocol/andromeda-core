use andromeda_std::{amp::AndrAddr, andr_exec, andr_instantiate, andr_query, error::ContractError};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{ensure, Addr};

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    pub gate_addresses: Vec<AndrAddr>,
    pub cycle_start_time: CycleStartTime,
    pub time_interval: Option<u64>,
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    UpdateCycleStartTime { cycle_start_time: CycleStartTime },
    UpdateGateAddresses { new_gate_addresses: Vec<AndrAddr> },
    UpdateTimeInterval { time_interval: u64 },
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Addr)]
    GetCurrentAdoPath {},
    #[returns(CycleStartTime)]
    GetCycleStartTime {},
    #[returns(Vec<AndrAddr>)]
    GetGateAddresses {},
    #[returns(String)]
    GetTimeInterval {},
}

#[cw_serde]
pub struct CycleStartTime {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

impl CycleStartTime {
    pub fn validate(&self) -> Result<(), ContractError> {
        ensure!(
            self.year >= 1970,
            ContractError::InvalidParameter {
                error: Some("Year must be bigger than 1969".to_string())
            }
        );
        ensure!(
            self.month <= 12 && self.month >= 1,
            ContractError::InvalidParameter {
                error: Some("Month must be between 1 and 12".to_string())
            }
        );
        let year = self.year;
        let month = self.month;
        let days_in_month_feb_29: [u32; 12] = [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        let days_in_month_feb_28: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

        if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
            ensure!(
                self.day <= days_in_month_feb_29[(month - 1) as usize] && self.day >= 1,
                ContractError::InvalidParameter {
                    error: Some("Wrong days of month".to_string())
                }
            );
        } else {
            ensure!(
                self.day <= days_in_month_feb_28[(month - 1) as usize] && self.day >= 1,
                ContractError::InvalidParameter {
                    error: Some("Wrong days of month".to_string())
                }
            );
        }

        ensure!(
            self.hour <= 23,
            ContractError::InvalidParameter {
                error: Some("Hour must be less than 24".to_string())
            }
        );
        ensure!(
            self.minute <= 59,
            ContractError::InvalidParameter {
                error: Some("Minute must be less than 60".to_string())
            }
        );
        ensure!(
            self.second <= 59,
            ContractError::InvalidParameter {
                error: Some("Second must be less than 60".to_string())
            }
        );

        Ok(())
    }
}
