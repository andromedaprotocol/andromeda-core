use andromeda_modules::schema::ValidateDataResponse;
use andromeda_std::error::ContractError;
use cosmwasm_std::Storage;
use jsonschema_valid::{schemas, Config};
use serde_json::{from_str, json};

use crate::state::SCHEMA;

pub fn validate_data(
    storage: &dyn Storage,
    data: String,
) -> Result<ValidateDataResponse, ContractError> {
    let schema = json!(SCHEMA.load(storage)?);
    let data_instance = from_str(data.as_str()).map_err(|_| ContractError::CustomError {
        msg: "Invalid data JSON".to_string(),
    })?;

    let config = Config::from_schema(&schema, Some(schemas::Draft::Draft7)).map_err(|_| {
        ContractError::CustomError {
            msg: "Validation Error".to_string(),
        }
    })?;

    let validate_res = config.validate(&data_instance);
    let is_valid = validate_res.is_ok();

    Ok(ValidateDataResponse { is_valid })
}
