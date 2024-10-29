use andromeda_modules::schema::{GetSchemaResponse, ValidateDataResponse};
use andromeda_std::error::ContractError;
use cosmwasm_std::Storage;
use jsonschema_valid::{schemas, Config};
use serde_json::{from_str, json};

use crate::state::SCHEMA;

pub fn get_schema(storage: &dyn Storage) -> Result<GetSchemaResponse, ContractError> {
    let schema = SCHEMA.load(storage)?.into();
    Ok(GetSchemaResponse { schema })
}

pub fn validate_data(
    storage: &dyn Storage,
    data: String,
) -> Result<ValidateDataResponse, ContractError> {
    let schema = json!(SCHEMA.load(storage)?);
    let data_instance = from_str(data.as_str()).map_err(|e| ContractError::CustomError {
        msg: format!("Invalid data JSON: {}", e),
    })?;

    let config = Config::from_schema(&schema, Some(schemas::Draft::Draft7)).map_err(|e| {
        ContractError::CustomError {
            msg: format!("Validation Error: {}", e),
        }
    })?;

    let validate_res = config.validate(&data_instance).map_err(|errors| {
        let error_messages: Vec<String> = errors
            .map(|err| format!("{}", err)) // Assuming ValidationError implements Display or Debug
            .collect(); // Collect errors into a Vec<String>

        error_messages.join(", ")
    });

    match validate_res.is_ok() {
        true => Ok(ValidateDataResponse::Valid),
        false => Ok(ValidateDataResponse::Invalid {
            msg: validate_res.unwrap_err(),
        }),
    }
}
