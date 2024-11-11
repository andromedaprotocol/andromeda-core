use andromeda_splitter::SplitterContract;
use andromeda_std::deploy::ADOMetadata;
use andromeda_validator_staking::ValidatorStakingContract;
use cw_orch::{anyhow::Chain, prelude::Uploadable};

pub enum AndromedaUploadable<Chain> {
    Splitter(SplitterContract<Chain>),
    ValidatorStaking(ValidatorStakingContract<Chain>),
}

impl<Chain> Uploadable for AndromedaUploadable<Chain> {}
impl<Chain> ADOMetadata for AndromedaUploadable<Chain> {
    fn name(&self) -> String {
        match self {
            AndromedaUploadable::Splitter(contract) => contract.name(),
            AndromedaUploadable::ValidatorStaking(contract) => contract.name(),
        }
    }

    fn version(&self) -> String {
        match self {
            AndromedaUploadable::Splitter(contract) => contract.version(),
            AndromedaUploadable::ValidatorStaking(contract) => contract.version(),
        }
    }
}

pub fn all_contracts(chain: Chain) -> Vec<AndromedaUploadable<Chain>> {
    vec![
        AndromedaUploadable::Splitter(SplitterContract::new(chain.clone())),
        AndromedaUploadable::ValidatorStaking(ValidatorStakingContract::new(chain)),
    ]
}
