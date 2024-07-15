use andromeda_modules::curve::{CurveRestriction, CurveType, CurveId};
use cw_storage_plus::Item;

pub const CURVE_TYPE: Item<CurveType> = Item::new("curve_type");
pub const CURVE_ID: Item<CurveId> = Item::new("curve_id");
pub const RESTRICTION: Item<CurveRestriction> = Item::new("curve_restriction");
pub const BASE_VALUE: Item<u64> = Item::new("base_value");
pub const MULTIPLE_VARIABLE_VALUE: Item<u64> = Item::new("multiple_variable_value");
pub const CONSTANT_VALUE: Item<u64> = Item::new("constant_value");
pub const IS_CONFIGURED_EXP: Item<bool> = Item::new("is_configured_exp");

pub const DEFAULT_MULTIPLE_VARIABLE_VALUE: u64 = 1;
pub const DEFAULT_CONSTANT_VALUE: u64 = 1;



