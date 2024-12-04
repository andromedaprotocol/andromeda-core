use andromeda_math::matrix::Matrix;
use cosmwasm_std::Addr;
use cw_storage_plus::Map;

pub const DEFAULT_KEY: &str = "default";

pub const MATRIX: Map<&str, Matrix> = Map::new("matrix");
pub const KEY_OWNER: Map<&str, Addr> = Map::new("key_owner");
