use andromeda_automation::oracle::TypeOfResponse;
use cosmwasm_std::Binary;
use cw_storage_plus::Item;

// The target ADO we want to querry
pub const TARGET_ADO_ADDRESS: Item<String> = Item::new("target_ado_address");

// Query message of the target ADO, converted into binary and supplied by the frontend
pub const QUERY_MSG: Item<Binary> = Item::new("query_message");

// The query's expected response type, either (u64, bool...) or (CountResponse, PriceResponse...)
pub const EXPECTED_TYPE_RESPONSE: Item<TypeOfResponse> = Item::new("expected_type_of_response");

// Desired response element, like "price" in a PriceResponse struct
pub const RESPONSE: Item<String> = Item::new("desired_response");
