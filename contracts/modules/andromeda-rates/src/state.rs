use andromeda_std::ado_base::rates::LocalRate;
use cw_storage_plus::Map;

// Mapping of action to LocalRate
pub const RATES: Map<&str, LocalRate> = Map::new("rates");
