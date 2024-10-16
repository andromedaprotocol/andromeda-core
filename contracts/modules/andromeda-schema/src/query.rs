use andromeda_modules::schema::ValidateDataResponse;
use andromeda_std::error::ContractError;
use cosmwasm_std::Storage;
use cw_json::JSON;
use jsonschema_valid::{schemas, Config};
use serde_json::json;

use crate::state::SCHEMA;

pub fn validate_data(
    storage: &dyn Storage,
    data: String,
) -> Result<ValidateDataResponse, ContractError> {
    let schema = json!(SCHEMA.load(storage)?);
    let data_instance = json!(JSON::from(data.as_str()));

    match Config::from_schema(&schema, Some(schemas::Draft::Draft7)).is_ok() {
        false => Err(ContractError::CustomError {
            msg: "Validation Error".to_string(),
        }),
        true => {
            let cfg = Config::from_schema(&schema, Some(schemas::Draft::Draft7)).unwrap();

            if cfg.validate(&data_instance).is_ok() {
                Ok(ValidateDataResponse { is_valid: true })
            } else {
                Ok(ValidateDataResponse { is_valid: false })
            }
        }
    }
}
