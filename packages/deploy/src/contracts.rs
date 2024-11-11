use andromeda_counter::CounterContract;
use andromeda_cw20::CW20Contract;
use andromeda_cw20_staking::CW20StakingContract;
use andromeda_cw721::CW721Contract;
use andromeda_primitive::PrimitiveContract;
use andromeda_splitter::SplitterContract;
use andromeda_std::deploy::ADOMetadata;
use andromeda_timelock::TimelockContract;
use andromeda_validator_staking::ValidatorStakingContract;
use andromeda_vesting::VestingContract;

use cw_orch::prelude::*;
use cw_orch_daemon::{DaemonBase, Wallet};

type UploadFn = Box<dyn FnOnce(&DaemonBase<Wallet>) -> Result<u64, CwOrchError>>;
pub type DeployableContract = (String, String, UploadFn);

/// Macro to create a tuple of (name, version, uploadFn) for a given contract.
macro_rules! deployable {
    ($contract_struct:ident) => {
        (
            $contract_struct::<DaemonBase<Wallet>>::name(),
            $contract_struct::<DaemonBase<Wallet>>::version(),
            Box::new(|chain: &DaemonBase<Wallet>| {
                let contract = $contract_struct::<DaemonBase<Wallet>>::new(chain.clone());
                contract.upload()?;
                Ok(contract.code_id().unwrap())
            }),
        )
    };
}

pub fn all_contracts() -> Vec<DeployableContract> {
    vec![
        deployable!(SplitterContract),
        deployable!(ValidatorStakingContract),
        deployable!(VestingContract),
        deployable!(TimelockContract),
        deployable!(CounterContract),
        deployable!(PrimitiveContract),
        deployable!(CW20Contract),
        deployable!(CW20StakingContract),
        deployable!(CW721Contract),
    ]
}
