use andromeda_splitter::SplitterContract;
use andromeda_std::deploy::ADOMetadata;
use andromeda_validator_staking::ValidatorStakingContract;
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
    ]
}
