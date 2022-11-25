use andromeda_automation::oracle::TypeOfResponse;
use cosmwasm_std::Binary;
use cw_storage_plus::Item;

// The target ADO we want to querry
pub const TARGET_ADO_ADDRESS: Item<String> = Item::new("target_ado_address");

// Query message of the target ADO, converted into binary and supplied by the frontend
pub const QUERY_MSG: Item<Binary> = Item::new("query_message");

// Expected response from query
pub const QUERY_RESPONSE: Item<TypeOfResponse> = Item::new("type_of_response");

// The specific element from the expected response
pub const RESPONSE_ELEMENT: Item<String> = Item::new("response_element");
