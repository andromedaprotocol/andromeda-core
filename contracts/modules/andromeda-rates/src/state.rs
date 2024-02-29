use andromeda_std::ado_base::rates::LocalRate;
use cw_storage_plus::Map;

pub const RATES: Map<&str, LocalRate> = Map::new("rates");
