use andromeda_modules::schema::{GetSchemaResponse, ValidateDataResponse};
use andromeda_std::error::ContractError;
use cosmwasm_std::Storage;
use serde_json::{from_str, json, Value};

use crate::state::SCHEMA;

pub fn get_schema(storage: &dyn Storage) -> Result<GetSchemaResponse, ContractError> {
    let schema = SCHEMA.load(storage)?.to_string();
    Ok(GetSchemaResponse { schema })
}

pub fn validate_data(
    storage: &dyn Storage,
    data: String,
) -> Result<ValidateDataResponse, ContractError> {
    // Load the schema from storage
    let schema: Value = json!(SCHEMA.load(storage)?);
    let data_instance: Value = from_str(&data).map_err(|e| ContractError::CustomError {
        msg: format!("Invalid data JSON: {}", e),
    })?;

    // Perform basic validation for types: string, array, and object
    if basic_type_matches(&schema, &data_instance) {
        Ok(ValidateDataResponse::Valid)
    } else {
        Ok(ValidateDataResponse::Invalid {
            msg: "Data structure does not match the basic schema types.".to_string(),
        })
    }
}

fn basic_type_matches(schema: &Value, data: &Value) -> bool {
    match schema.get("type") {
        Some(Value::String(schema_type)) => match schema_type.as_str() {
            "string" => data.is_string(),
            "number" => data.is_number(),
            "boolean" => data.is_boolean(),
            "array" => {
                if let Some(items_schema) = schema.get("items") {
                    if let Value::Array(data_array) = data {
                        data_array
                            .iter()
                            .all(|item| basic_type_matches(items_schema, item))
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            "object" => {
                if let Some(Value::Object(schema_props)) = schema.get("properties") {
                    if let Value::Object(data_obj) = data {
                        // Check for required properties
                        let required_fields = schema.get("required").and_then(|r| r.as_array());
                        if let Some(required_fields) = required_fields {
                            if !required_fields.iter().all(|field| {
                                field.as_str().is_some_and(|f| data_obj.contains_key(f))
                            }) {
                                return false;
                            }
                        }
                        // Check each property
                        schema_props.iter().all(|(key, prop_schema)| {
                            if let Some(data_value) = data_obj.get(key) {
                                basic_type_matches(prop_schema, data_value)
                            } else {
                                true // Property not present, acceptable if not required
                            }
                        })
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            _ => false, // Unsupported type
        },
        _ => false, // Type not specified in schema
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_basic_type_matches_string() {
        let schema = json!({ "type": "string" });
        let data = json!("Hello World");
        assert!(basic_type_matches(&schema, &data));

        let data = json!(123);
        assert!(!basic_type_matches(&schema, &data));
    }

    #[test]
    fn test_basic_type_matches_number() {
        let schema = json!({ "type": "number" });
        let data = json!(42);
        assert!(basic_type_matches(&schema, &data));

        let data = json!("42");
        assert!(!basic_type_matches(&schema, &data));
    }

    #[test]
    fn test_basic_type_matches_boolean() {
        let schema = json!({ "type": "boolean" });
        let data = json!(true);
        assert!(basic_type_matches(&schema, &data));

        let data = json!("true");
        assert!(!basic_type_matches(&schema, &data));
    }

    #[test]
    fn test_basic_type_matches_array_of_strings() {
        let schema = json!({
            "type": "array",
            "items": { "type": "string" }
        });
        let data = json!(["apple", "banana", "cherry"]);
        assert!(basic_type_matches(&schema, &data));

        let data = json!(["apple", 123, "cherry"]);
        assert!(!basic_type_matches(&schema, &data));
    }

    #[test]
    fn test_basic_type_matches_array_of_objects() {
        let schema = json!({
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "id": { "type": "number" },
                    "name": { "type": "string" }
                },
                "required": ["id", "name"]
            }
        });
        let data = json!([
            { "id": 1, "name": "Alice" },
            { "id": 2, "name": "Bob" }
        ]);
        assert!(basic_type_matches(&schema, &data));

        let data = json!([
            { "id": "one", "name": "Alice" },
            { "id": 2, "name": "Bob" }
        ]);
        assert!(!basic_type_matches(&schema, &data));
    }

    #[test]
    fn test_basic_type_matches_object() {
        let schema = json!({
            "type": "object",
            "properties": {
                "title": { "type": "string" },
                "count": { "type": "number" }
            },
            "required": ["title", "count"]
        });
        let data = json!({ "title": "Introduction", "count": 10 });
        assert!(basic_type_matches(&schema, &data));

        let data = json!({ "title": "Introduction" });
        assert!(!basic_type_matches(&schema, &data));

        let data = json!({ "title": "Introduction", "count": "ten" });
        assert!(!basic_type_matches(&schema, &data));
    }

    #[test]
    fn test_basic_type_matches_nested_object() {
        let schema = json!({
            "type": "object",
            "properties": {
                "user": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" },
                        "age": { "type": "number" }
                    },
                    "required": ["name", "age"]
                },
                "roles": {
                    "type": "array",
                    "items": { "type": "string" }
                }
            },
            "required": ["user", "roles"]
        });
        let data = json!({
            "user": { "name": "Charlie", "age": 25 },
            "roles": ["admin", "editor"]
        });
        assert!(basic_type_matches(&schema, &data));

        // Missing required property "age"
        let data = json!({
            "user": { "name": "Charlie" },
            "roles": ["admin", "editor"]
        });
        assert!(!basic_type_matches(&schema, &data));

        // Incorrect type for "age"
        let data = json!({
            "user": { "name": "Charlie", "age": "twenty-five" },
            "roles": ["admin", "editor"]
        });
        assert!(!basic_type_matches(&schema, &data));

        // Incorrect type in "roles" array
        let data = json!({
            "user": { "name": "Charlie", "age": 25 },
            "roles": ["admin", 123]
        });
        assert!(!basic_type_matches(&schema, &data));
    }
}
