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

    let config_from_schema = Config::from_schema(&schema, Some(schemas::Draft::Draft7));

    match config_from_schema.is_ok() {
        false => Err(ContractError::CustomError {
            msg: "Validation Error".to_string(),
        }),
        true => {
            let cfg = config_from_schema.unwrap();

            if cfg.validate(&data_instance).is_ok() {
                Ok(ValidateDataResponse { is_valid: true })
            } else {
                Ok(ValidateDataResponse { is_valid: false })
            }
        }
    }
}
