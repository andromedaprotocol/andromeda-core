use andromeda_modules::schema::ValidateDataResponse;
use andromeda_std::error::ContractError;
use cosmwasm_std::Storage;
use cw_json::JSON;
use jsonschema::validator_for;
use serde_json::json;

use crate::state::SCHEMA;

pub fn validate_data(
    storage: &dyn Storage,
    data: String,
) -> Result<ValidateDataResponse, ContractError> {
    let schema = json!(SCHEMA.load(storage)?);
    let data_instance = json!(JSON::from(data.as_str()));

    match validator_for(&schema).is_ok() {
        false => {
            return Err(ContractError::CustomError {
                msg: "Schema Validation Error".to_string(),
            })
        }
        true => {
            let validator = jsonschema::validator_for(&schema).unwrap();
            let is_valid = validator.is_valid(&data_instance);

            Ok(ValidateDataResponse { is_valid })
        }
    }
}
