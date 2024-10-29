use super::mock::{proper_initialization, query_schema, query_validate_data, update_schema};
use andromeda_modules::schema::ValidateDataResponse;
use test_case::test_case;

pub const SCHEMA_ADO_INSTANTIATION_MSG_SCHEMA: &str = r#"
{
    "$schema": "http://json-schema.org/draft-07/schema#",
    "title": "InstantiateMsg",
    "type": "object",
    "required": [
        "kernel_address",
        "schema_json_string"
    ],
    "properties": {
        "kernel_address": {
        "type": "string"
        },
        "owner": {
        "type": [
            "string",
            "null"
        ]
        },
        "schema_json_string": {
        "type": "string"
        }
    },
    "additionalProperties": false
}"#;

pub const APP_CONTRACT_INSTANTIATION_MSG_SCHEMA: &str = r#"
{
    "$schema": "http://json-schema.org/draft-07/schema#",
    "$id": "http://json-schema.org/draft-07/schema#",
    "additionalProperties": false,
    "adoType": "app-contract",
    "contractName": "my-app-contract",
    "class": "baseADO",
    "classifier": "",
    "description": "The App Contract component is an ADO that bundles a collection of other ADOs together for streamlined coordinated use.",
    "properties": {
        "app_components": {
            "items": {
                "$original_type": "AppComponent",
                "additionalProperties": false,
                "properties": {
                    "ado_type": {
                        "type": "string"
                    },
                    "component_type": {
                        "$original_type": "ComponentType",
                        "oneOf": [
                            {
                                "additionalProperties": false,
                                "properties": {
                                    "new": {
                                        "$original_type": "Binary",
                                        "description": "Binary Msg",
                                        "type": "string"
                                    }
                                },
                                "required": [
                                    "new"
                                ],
                                "title": "New",
                                "type": "object"
                            },
                            {
                                "additionalProperties": false,
                                "properties": {
                                    "symlink": {
                                        "$original_type": "AndrAddr",
                                        "description": "Andr Address, can be VFS`",
                                        "pattern": "",
                                        "type": "string"
                                    }
                                },
                                "required": [
                                    "symlink"
                                ],
                                "title": "Symlink",
                                "type": "object"
                            }
                        ]
                    },
                    "name": {
                        "type": "string"
                    }
                },
                "required": [
                    "ado_type",
                    "component_type",
                    "name"
                ],
                "title": "Ado type",
                "type": "object"
            },
            "title": "App components",
            "type": "array"
        },
        "chain_info": {
            "items": {
                "$original_type": "ChainInfo",
                "additionalProperties": false,
                "properties": {
                    "chain_name": {
                        "type": "string"
                    },
                    "owner": {
                        "type": "string"
                    }
                },
                "required": [
                    "chain_name",
                    "owner"
                ],
                "title": "Chain name",
                "type": "object"
            },
            "title": "Chain info",
            "type": [
                "array",
                "null"
            ]
        },
        "kernel_address": {
            "default": "",
            "title": "Kernel address",
            "type": "string"
        },
        "name": {
            "title": "Name",
            "type": "string"
        },
        "owner": {
            "title": "Owner",
            "type": [
                "string",
                "null"
            ]
        }
    },
    "required": [
        "app_components",
        "kernel_address",
        "name"
    ],
    "title": "App contract",
    "type": "object",
    "version": "1.0.0"
}"#;

#[test_case(
    SCHEMA_ADO_INSTANTIATION_MSG_SCHEMA,
    r#"{
        "kernel_address": "0x123abc456",
        "owner": null,
        "schema_json_string": "{\"type\":\"object\", \"properties\": {\"name\": {\"type\":\"string\"}}}"
    }"#,
    ValidateDataResponse::Valid ;
    "valid instantiation schema"
)]
#[test_case(
    SCHEMA_ADO_INSTANTIATION_MSG_SCHEMA,
    r#"{
        "owner": "0x789xyz"
    }"#,
    ValidateDataResponse::Invalid { msg: "Required properties \"kernel_address\", \"schema_json_string\" are missing\nAt instance path /:\n  {\n    \"owner\": \"0x789xyz\"\n  }\nAt schema path /required:\n  [\n    \"kernel_address\",\n    \"schema_json_string\"\n  ]\n".to_string() } ;
    "missing required fields"
)]
#[test_case(
    SCHEMA_ADO_INSTANTIATION_MSG_SCHEMA,
    r#"{
        "kernel_address": ["invalid"],
        "schema_json_string": "{\"type\":\"object\"}"
    }"#,
    ValidateDataResponse::Invalid { msg: "Invalid type.\nAt instance path /kernel_address:\n  [\n    \"invalid\"\n  ]\nAt schema path /properties/kernel_address/type:\n  {\n    \"type\": \"string\"\n  }\n".to_string() } ;
    "invalid type for kernel_address"
)]
#[test_case(
    SCHEMA_ADO_INSTANTIATION_MSG_SCHEMA,
    r#"{
        "kernel_address": "0x123abc456",
        "schema_json_string": "{\"type\":\"object\"}",
        "extra_property": "not allowed"
    }"#,
    ValidateDataResponse::Invalid { msg: "Additional properties are not allowed. Found \"extra_property\".\nAt instance path /:\n  {\n    \"extra_property\": \"not allowed\",\n    \"kernel_address\": \"0x123abc456\",\n    \"schema_json_string\": \"{\\\"type\\\":\\\"object\\\"}\"\n  }\nAt schema path /additionalProperties:\n  {\n    \"$schema\": \"http://json-schema.org/draft-07/schema#\",\n    \"additionalProperties\": false,\n    \"properties\": {\n      \"kernel_address\": {\n        \"type\": \"string\"\n      },\n      \"owner\": {\n        \"type\": [\n          \"string\",\n          \"null\"\n        ]\n      },\n      \"schema_json_string\": {\n        \"type\": \"string\"\n      }\n    },\n    \"required\": [\n      \"kernel_address\",\n      \"schema_json_string\"\n    ],\n    \"title\": \"InstantiateMsg\",\n    \"type\": \"object\"\n  }\n".to_string() } ;
    "extra properties not allowed"
)]
fn test_instantiation_schema_validation_with_errors(
    schema: &str,
    data: &str,
    expected_res: ValidateDataResponse,
) {
    let (deps, _) = proper_initialization(schema.to_string());
    let query_res = query_validate_data(deps.as_ref(), data.to_string()).unwrap();
    assert_eq!(query_res, expected_res)
}

#[test_case(
    APP_CONTRACT_INSTANTIATION_MSG_SCHEMA,
    r#"{
        "app_components": [
            {
                "ado_type": "test_ado",
                "component_type": {
                    "new": "test_binary"
                },
                "name": "component_name"
            }
        ],
        "kernel_address": "test_kernel",
        "name": "test_contract"
    }"# => true;
    "valid app contract"
)]
#[test_case(
    APP_CONTRACT_INSTANTIATION_MSG_SCHEMA,
    r#"{
        "kernel_address": "test_kernel",
        "name": "test_contract"
    }"# => false;
    "missing app_components"
)]
#[test_case(
    APP_CONTRACT_INSTANTIATION_MSG_SCHEMA,
    r#"{
        "app_components": [],
        "kernel_address": "test_kernel",
        "name": 123
    }"# => false;
    "invalid type for name"
)]
#[test_case(
    APP_CONTRACT_INSTANTIATION_MSG_SCHEMA,
    r#"{
        "app_components": [
            {
                "ado_type": "test_ado",
                "component_type": {},
                "name": "component_name"
            }
        ],
        "kernel_address": "test_kernel",
        "name": "test_contract"
    }"# => false;
    "invalid structure for component_type"
)]
fn test_app_contract_schema_validation(schema: &str, data: &str) -> bool {
    let (deps, _) = proper_initialization(schema.to_string());
    let query_res: ValidateDataResponse =
        query_validate_data(deps.as_ref(), data.to_string()).unwrap();
    match query_res {
        ValidateDataResponse::Valid => true,
        ValidateDataResponse::Invalid { msg: _ } => false,
    }
}

#[test_case(
    APP_CONTRACT_INSTANTIATION_MSG_SCHEMA,
    SCHEMA_ADO_INSTANTIATION_MSG_SCHEMA,
    r#"{
        "app_components": [
            {
                "ado_type": "test_ado",
                "component_type": {
                    "new": "test_binary"
                },
                "name": "component_name"
            }
        ],
        "kernel_address": "test_kernel",
        "name": "test_contract"
    }"# => false;
    "invalid data after schema update"
)]
#[test_case(
    APP_CONTRACT_INSTANTIATION_MSG_SCHEMA,
    SCHEMA_ADO_INSTANTIATION_MSG_SCHEMA,
    r#"{
        "kernel_address": "0x123abc456",
        "owner": null,
        "schema_json_string": "{\"type\":\"object\", \"properties\": {\"name\": {\"type\":\"string\"}}}"
    }"# => true;
    "valid data after schema update"
)]
fn test_update_schema(app_schema: &str, ado_schema: &str, data: &str) -> bool {
    let (mut deps, info) = proper_initialization(app_schema.to_string());

    // Update schema
    update_schema(deps.as_mut(), info.sender.as_ref(), ado_schema.to_string()).unwrap();

    // Test the data after schema update
    let query_res = query_validate_data(deps.as_ref(), data.to_string()).unwrap();
    match query_res {
        ValidateDataResponse::Valid => true,
        ValidateDataResponse::Invalid { msg: _ } => false,
    }
}

#[test]
fn test_query_schema() {
    let (deps, _) = proper_initialization(SCHEMA_ADO_INSTANTIATION_MSG_SCHEMA.to_string());
    let query_res = query_schema(deps.as_ref()).unwrap();
    let schema = query_res.schema;
    assert_eq!(
        schema,
        "{\"$schema\":\"http://json-schema.org/draft-07/schema#\",\"additionalProperties\":false,\"properties\":{\"kernel_address\":{\"type\":\"string\"},\"owner\":{\"type\":[\"string\",\"null\"]},\"schema_json_string\":{\"type\":\"string\"}},\"required\":[\"kernel_address\",\"schema_json_string\"],\"title\":\"InstantiateMsg\",\"type\":\"object\"}".to_string()
    );
}
