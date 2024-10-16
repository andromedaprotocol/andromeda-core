use super::mock::{proper_initialization, query_validate_data, update_schema};
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
    }"# => true;
    "valid instantiation schema"
)]
#[test_case(
    SCHEMA_ADO_INSTANTIATION_MSG_SCHEMA,
    r#"{
        "owner": "0x789xyz"
    }"# => false;
    "missing required fields"
)]
#[test_case(
    SCHEMA_ADO_INSTANTIATION_MSG_SCHEMA,
    r#"{
        "kernel_address": ["invalid"],
        "schema_json_string": "{\"type\":\"object\"}"
    }"# => false;
    "invalid type for kernel_address"
)]
#[test_case(
    SCHEMA_ADO_INSTANTIATION_MSG_SCHEMA,
    r#"{
        "kernel_address": "0x123abc456",
        "schema_json_string": "{\"type\":\"object\"}",
        "extra_property": "not allowed"
    }"# => false;
    "extra properties not allowed"
)]
fn test_instantiation_schema_validation(schema: &str, data: &str) -> bool {
    let (deps, _) = proper_initialization(schema.to_string());
    let query_res = query_validate_data(deps.as_ref(), data.to_string()).unwrap();
    query_res.is_valid
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
    let query_res = query_validate_data(deps.as_ref(), data.to_string()).unwrap();
    query_res.is_valid
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
    query_res.is_valid
}
