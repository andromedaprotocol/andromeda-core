use andromeda_std::ado_base::rates::Rate;
use cw_storage_plus::Map;

pub const RATES: Map<&str, Rate> = Map::new("rates");
