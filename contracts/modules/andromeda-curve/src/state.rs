use andromeda_modules::curve::{CurveConfig, CurveRestriction};
use cw_storage_plus::Item;

pub const CURVE_CONFIG: Item<CurveConfig> = Item::new("curve_config");
pub const RESTRICTION: Item<CurveRestriction> = Item::new("curve_restriction");

pub const DEFAULT_MULTIPLE_VARIABLE_VALUE: u64 = 1;
pub const DEFAULT_CONSTANT_VALUE: u64 = 1;
