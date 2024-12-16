use super::mock::{proper_initialization, query_schema, query_validate_data};
use andromeda_modules::schema::ValidateDataResponse;
use test_case::test_case;

// JSON schema definitions for each type and nested structures
pub const SCHEMA_STRING_TYPE: &str = r#"
{
    "type": "string"
}"#;

pub const SCHEMA_NUMBER_TYPE: &str = r#"
{
    "type": "number"
}"#;

pub const SCHEMA_BOOLEAN_TYPE: &str = r#"
{
    "type": "boolean"
}"#;

pub const SCHEMA_ARRAY_OF_STRINGS: &str = r#"
{
    "type": "array",
    "items": { "type": "string" }
}"#;

pub const SCHEMA_ARRAY_OF_OBJECTS: &str = r#"
{
    "type": "array",
    "items": {
        "type": "object",
        "properties": {
            "id": { "type": "number" },
            "name": { "type": "string" }
        },
        "required": ["id", "name"]
    }
}"#;

pub const SCHEMA_SIMPLE_OBJECT: &str = r#"
{
    "type": "object",
    "properties": {
        "title": { "type": "string" },
        "count": { "type": "number" }
    },
    "required": ["title", "count"]
}"#;

pub const SCHEMA_NESTED_OBJECT: &str = r#"
{
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
}"#;

#[test_case(
    SCHEMA_STRING_TYPE,
    r#""Hello World""#,
    ValidateDataResponse::Valid ;
    "valid string data"
)]
#[test_case(
    SCHEMA_STRING_TYPE,
    r#"123"#,
    ValidateDataResponse::Invalid { msg: "Data structure does not match the basic schema types.".to_string() } ;
    "invalid non-string for string schema"
)]
#[test_case(
    SCHEMA_NUMBER_TYPE,
    r#"42"#,
    ValidateDataResponse::Valid ;
    "valid number data"
)]
#[test_case(
    SCHEMA_NUMBER_TYPE,
    r#""forty-two""#,
    ValidateDataResponse::Invalid { msg: "Data structure does not match the basic schema types.".to_string() } ;
    "invalid non-number for number schema"
)]
#[test_case(
    SCHEMA_BOOLEAN_TYPE,
    r#"true"#,
    ValidateDataResponse::Valid ;
    "valid boolean data"
)]
#[test_case(
    SCHEMA_BOOLEAN_TYPE,
    r#""true""#,
    ValidateDataResponse::Invalid { msg: "Data structure does not match the basic schema types.".to_string() } ;
    "invalid non-boolean for boolean schema"
)]
#[test_case(
    SCHEMA_ARRAY_OF_STRINGS,
    r#"["apple", "banana", "cherry"]"#,
    ValidateDataResponse::Valid ;
    "valid array of strings"
)]
#[test_case(
    SCHEMA_ARRAY_OF_STRINGS,
    r#"[1, 2, 3]"#,
    ValidateDataResponse::Invalid { msg: "Data structure does not match the basic schema types.".to_string() } ;
    "invalid array of non-strings for string array schema"
)]
#[test_case(
    SCHEMA_ARRAY_OF_OBJECTS,
    r#"[{"id": 1, "name": "Alice"}, {"id": 2, "name": "Bob"}]"#,
    ValidateDataResponse::Valid ;
    "valid array of objects with required fields"
)]
#[test_case(
    SCHEMA_ARRAY_OF_OBJECTS,
    r#"[{"id": "one", "name": "Alice"}, {"id": 2, "name": "Bob"}]"#,
    ValidateDataResponse::Invalid { msg: "Data structure does not match the basic schema types.".to_string() } ;
    "invalid array of objects with wrong type for id field"
)]
#[test_case(
    SCHEMA_SIMPLE_OBJECT,
    r#"{"title": "Introduction", "count": 10}"#,
    ValidateDataResponse::Valid ;
    "valid simple object with required fields"
)]
#[test_case(
    SCHEMA_SIMPLE_OBJECT,
    r#"{"title": "Introduction"}"#,
    ValidateDataResponse::Invalid { msg: "Data structure does not match the basic schema types.".to_string() } ;
    "missing required field in simple object"
)]
#[test_case(
    SCHEMA_SIMPLE_OBJECT,
    r#"{"title": "Introduction", "count": "not_a_number"}"#,
    ValidateDataResponse::Invalid { msg: "Data structure does not match the basic schema types.".to_string() } ;
    "invalid type for count in simple object"
)]
#[test_case(
    SCHEMA_NESTED_OBJECT,
    r#"{
        "user": {
            "name": "Charlie",
            "age": 25
        },
        "roles": ["admin", "user"]
    }"#,
    ValidateDataResponse::Valid ;
    "valid nested object with required properties"
)]
#[test_case(
    SCHEMA_NESTED_OBJECT,
    r#"{
        "user": {
            "name": "Charlie"
        },
        "roles": ["admin", "user"]
    }"#,
    ValidateDataResponse::Invalid { msg: "Data structure does not match the basic schema types.".to_string() } ;
    "missing required field in nested object"
)]
#[test_case(
    SCHEMA_NESTED_OBJECT,
    r#"{
        "user": {
            "name": "Charlie",
            "age": "twenty-five"
        },
        "roles": ["admin", "user"]
    }"#,
    ValidateDataResponse::Invalid { msg: "Data structure does not match the basic schema types.".to_string() } ;
    "invalid type for age in nested object"
)]
#[test_case(
    SCHEMA_NESTED_OBJECT,
    r#"{
        "user": {
            "name": "Charlie",
            "age": 25
        },
        "roles": ["admin", 123]
    }"#,
    ValidateDataResponse::Invalid { msg: "Data structure does not match the basic schema types.".to_string() } ;
    "invalid type in roles array within nested object"
)]
fn test_basic_type_matches_cases(schema: &str, data: &str, expected_res: ValidateDataResponse) {
    let (deps, _) = proper_initialization(schema.to_string());
    let query_res = query_validate_data(deps.as_ref(), data.to_string()).unwrap();
    assert_eq!(query_res, expected_res);
}

pub const SCHEMA_INITIAL: &str = r#"
{
    "type": "object",
    "properties": {
        "name": { "type": "string" },
        "age": { "type": "number" }
    },
    "required": ["name", "age"]
}"#;

#[test]
fn test_query_schema() {
    let (deps, _) = proper_initialization(SCHEMA_INITIAL.to_string());
    let query_res = query_schema(deps.as_ref()).unwrap();
    let schema = query_res.schema;
    assert_eq!(
        schema,
        "{\"properties\":{\"age\":{\"type\":\"number\"},\"name\":{\"type\":\"string\"}},\"required\":[\"name\",\"age\"],\"type\":\"object\"}".to_string()
    );
}
