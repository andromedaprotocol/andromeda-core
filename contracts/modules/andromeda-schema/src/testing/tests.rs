use super::mock::{proper_initialization, query_validate_data, update_schema};

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
    "$id": "app-contract",
    "$schema": "http://json-schema.org/draft-07/schema#",
    "additionalProperties": false,
    "adoType": "app-contract",
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

#[test]
fn test_valid_data_against_schema_instantiation_msg_schema() {
    let schema = SCHEMA_ADO_INSTANTIATION_MSG_SCHEMA;
    let (deps, _) = proper_initialization(schema.to_string());

    let valid_data = r#"
    {
        "kernel_address": "0x123abc456",
        "owner": null,
        "schema_json_string": "{\"type\":\"object\", \"properties\": {\"name\": {\"type\":\"string\"}}}"
    }"#;

    let query_res = query_validate_data(deps.as_ref(), valid_data.to_string()).unwrap();

    assert!(query_res.is_valid);
}

#[test]
fn test_invalid_datas_against_schema_instantiation_msg_schema() {
    let schema = SCHEMA_ADO_INSTANTIATION_MSG_SCHEMA;
    let (deps, _) = proper_initialization(schema.to_string());

    // Missing required fields
    let invalid_data = r#"
    {
        "owner": "0x789xyz"
    }"#;
    let query_res = query_validate_data(deps.as_ref(), invalid_data.to_string()).unwrap();
    assert!(!query_res.is_valid);

    // Invalid type for kernel_address
    let invalid_data = r#"
    {
        "kernel_address": ["invalid"],
        "schema_json_string": "{\"type\":\"object\"}"
    }"#;
    let query_res = query_validate_data(deps.as_ref(), invalid_data.to_string()).unwrap();
    assert!(!query_res.is_valid);

    // Extra properties not allowed
    let invalid_data = r#"
    {
        "kernel_address": "0x123abc456",
        "schema_json_string": "{\"type\":\"object\"}",
        "extra_property": "not allowed"
    }"#;
    let query_res = query_validate_data(deps.as_ref(), invalid_data.to_string()).unwrap();
    assert!(!query_res.is_valid);
}

#[test]
fn test_valid_data_against_app_contract_schema() {
    let schema = APP_CONTRACT_INSTANTIATION_MSG_SCHEMA;
    let (deps, _) = proper_initialization(schema.to_string());

    let valid_data = r#"
    {
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
    }"#;

    let query_res = query_validate_data(deps.as_ref(), valid_data.to_string()).unwrap();

    assert!(query_res.is_valid);
}

#[test]
fn test_invalid_data_against_app_contract_schema() {
    let schema = APP_CONTRACT_INSTANTIATION_MSG_SCHEMA;
    let (deps, _) = proper_initialization(schema.to_string());

    // Missing app_components
    let invalid_data = r#"
    {
        "kernel_address": "test_kernel",
        "name": "test_contract"
    }"#;
    let query_res = query_validate_data(deps.as_ref(), invalid_data.to_string()).unwrap();
    assert!(!query_res.is_valid);

    // Invalid type for name
    let invalid_data = r#"
    {
        "app_components": [],
        "kernel_address": "test_kernel",
        "name": 123
    }"#;
    let query_res = query_validate_data(deps.as_ref(), invalid_data.to_string()).unwrap();
    assert!(!query_res.is_valid);

    // Invalid structure for component_type
    let invalid_data = r#"
    {
        "app_components": [
            {
                "ado_type": "test_ado",
                "component_type": {},
                "name": "component_name"
            }
        ],
        "kernel_address": "test_kernel",
        "name": "test_contract"
    }"#;
    let query_res = query_validate_data(deps.as_ref(), invalid_data.to_string()).unwrap();
    assert!(!query_res.is_valid);

    // Additional Properties in component_type
    let invalid_data = r#"
    {
        "app_components": [
            {
                "ado_type": "test_ado",
                "component_type": {
                    "new": "test_binary",
                    "extra_property": "not_allowed"
                },
                "name": "component_name"
            }
        ],
        "kernel_address": "test_kernel",
        "name": "test_contract"
    }"#;
    let query_res = query_validate_data(deps.as_ref(), invalid_data.to_string()).unwrap();
    assert!(!query_res.is_valid);

    // Both new and symlink Present for component type
    let invalid_data = r#"
    {
        "app_components": [
            {
                "ado_type": "test_ado",
                "component_type": {
                    "new": "test_binary",
                    "symlink": "some_address"
                },
                "name": "component_name"
            }
        ],
        "kernel_address": "test_kernel",
        "name": "test_contract"
    }"#;
    let query_res = query_validate_data(deps.as_ref(), invalid_data.to_string()).unwrap();
    assert!(!query_res.is_valid);
}

#[test]
fn test_update_schema() {
    let schema = APP_CONTRACT_INSTANTIATION_MSG_SCHEMA;
    let (mut deps, info) = proper_initialization(schema.to_string());

    update_schema(
        deps.as_mut(),
        info.sender.as_ref(),
        SCHEMA_ADO_INSTANTIATION_MSG_SCHEMA.to_string(),
    )
    .unwrap();

    let invalid_data = r#"
    {
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
    }"#;
    let invalid_query_res = query_validate_data(deps.as_ref(), invalid_data.to_string()).unwrap();
    assert!(!invalid_query_res.is_valid);

    let valid_data = r#"
    {
        "kernel_address": "0x123abc456",
        "owner": null,
        "schema_json_string": "{\"type\":\"object\", \"properties\": {\"name\": {\"type\":\"string\"}}}"
    }"#;
    let valid_query_res = query_validate_data(deps.as_ref(), valid_data.to_string()).unwrap();
    assert!(valid_query_res.is_valid);
}
