use andromeda_std::{common::context::ExecuteContext, error::ContractError};
use cosmwasm_std::Response;
use cw_json::JSON;
use serde_json::{from_str, Value};

use crate::state::SCHEMA;

pub fn execute_update_schema(
    ctx: ExecuteContext,
    new_schema_json: String,
) -> Result<Response, ContractError> {
    let sender: cosmwasm_std::Addr = ctx.info.sender;
    let new_schema_json_value: Value =
        from_str(new_schema_json.as_str()).map_err(|_| ContractError::CustomError {
            msg: "Invalid JSON Schema".to_string(),
        })?;
    let new_schema_json = JSON::try_from(new_schema_json_value.to_string().as_str()).unwrap();

    SCHEMA.save(ctx.deps.storage, &new_schema_json)?;

    let response = Response::new()
        .add_attribute("method", "update_schema")
        .add_attribute("sender", sender);

    Ok(response)
}
