use cw_orch::prelude::CwOrchError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DeployError {
    #[error("{0}")]
    CwOrchError(#[from] CwOrchError),
}
