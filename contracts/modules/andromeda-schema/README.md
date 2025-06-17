# Andromeda Schema ADO

## Introduction

The Andromeda Schema ADO is a fundamental validation module that provides JSON schema-based data validation services for other contracts and applications within the Andromeda ecosystem. This contract stores and manages JSON schemas and offers validation services to ensure data integrity and consistency across different applications. The schema system supports complex validation rules including type checking, required fields, nested objects, and array validations, making it essential for form validation, data collection, API validation, and any scenario requiring structured data compliance.

<b>Ado_type:</b> schema

## Why Schema ADO

The Schema ADO serves as critical validation infrastructure for applications requiring:

- **Data Validation**: Ensure submitted data meets predefined structural requirements
- **Form Validation**: Validate user inputs in form submissions and applications
- **API Validation**: Validate request/response data in API interactions
- **Configuration Validation**: Ensure configuration data meets expected schemas
- **Data Integrity**: Maintain data consistency across different system components
- **Contract Integration**: Provide validation services to other smart contracts
- **Type Safety**: Enforce type safety for JSON data in decentralized applications
- **Compliance Checking**: Validate data against regulatory or business requirements
- **Quality Assurance**: Ensure data quality before processing or storage
- **Integration Standards**: Maintain consistent data formats across integrations

The ADO provides centralized schema management with comprehensive validation capabilities for reliable data integrity enforcement.

## Key Features

### **JSON Schema Validation**
- **Standard compliance**: Supports standard JSON Schema validation patterns
- **Type validation**: Validates string, number, boolean, array, and object types
- **Required fields**: Enforces required field validation for objects
- **Nested validation**: Supports complex nested object and array validation
- **Property validation**: Validates object properties against defined schemas

### **Dynamic Schema Management**
- **Schema updates**: Update validation schemas through administrative functions
- **Version control**: Manage schema evolution through update mechanisms
- **Centralized storage**: Single source of truth for validation rules
- **Query access**: Retrieve current schema for external validation
- **Administrative control**: Owner-only schema modification capabilities

### **Validation Services**
- **Real-time validation**: Immediate validation feedback for submitted data
- **Detailed error reporting**: Clear error messages for validation failures
- **Type checking**: Comprehensive type validation for all JSON data types
- **Structural validation**: Validate complex object and array structures
- **Cross-contract integration**: Provide validation services to other contracts

### **Advanced Validation Logic**
- **Recursive validation**: Support for deeply nested data structures
- **Array item validation**: Validate each item in arrays against item schemas
- **Optional properties**: Handle optional vs required object properties
- **Type coercion**: Strict type checking without automatic coercion
- **Custom validation**: Support for complex validation patterns

## Validation Engine

### **Supported Types**
The schema ADO validates the following JSON data types:
- **string**: Text data validation
- **number**: Numeric data validation (integers and floats)
- **boolean**: True/false value validation
- **array**: Array structure and item validation
- **object**: Object structure and property validation

### **Validation Process**
1. **Schema loading**: Load the stored JSON schema
2. **Data parsing**: Parse input data as JSON
3. **Type checking**: Validate data types against schema
4. **Required field validation**: Check for required object properties
5. **Recursive validation**: Validate nested structures
6. **Result reporting**: Return validation success or detailed error messages

### **Error Handling**
- **Invalid JSON**: Clear error messages for malformed JSON input
- **Type mismatches**: Specific errors for type validation failures
- **Missing fields**: Detailed messages for missing required fields
- **Structural errors**: Clear feedback for object/array structure violations

## InstantiateMsg

```rust
pub struct InstantiateMsg {
    pub schema_json_string: String,
}
```

```json
{
    "schema_json_string": "{\"type\": \"object\", \"properties\": {\"name\": {\"type\": \"string\"}, \"email\": {\"type\": \"string\"}, \"age\": {\"type\": \"number\"}}, \"required\": [\"name\", \"email\"]}"
}
```

**Parameters**:
- **schema_json_string**: Valid JSON schema definition as a string

**Schema Examples**:

**Simple Object Schema:**
```json
{
    "type": "object",
    "properties": {
        "name": {"type": "string"},
        "age": {"type": "number"},
        "active": {"type": "boolean"}
    },
    "required": ["name", "age"]
}
```

**Array Schema:**
```json
{
    "type": "array",
    "items": {
        "type": "object",
        "properties": {
            "id": {"type": "number"},
            "title": {"type": "string"}
        },
        "required": ["id", "title"]
    }
}
```

**Nested Object Schema:**
```json
{
    "type": "object",
    "properties": {
        "user": {
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "contact": {
                    "type": "object",
                    "properties": {
                        "email": {"type": "string"},
                        "phone": {"type": "string"}
                    },
                    "required": ["email"]
                }
            },
            "required": ["name", "contact"]
        },
        "preferences": {
            "type": "array",
            "items": {"type": "string"}
        }
    },
    "required": ["user"]
}
```

**Validation**:
- Schema must be valid JSON
- Schema must follow JSON Schema format standards
- Schema structure must be properly formatted

## ExecuteMsg

### UpdateSchema
Updates the stored JSON schema (owner-only).

```rust
UpdateSchema {
    new_schema_json_string: String,
}
```

```json
{
    "update_schema": {
        "new_schema_json_string": "{\"type\": \"object\", \"properties\": {\"name\": {\"type\": \"string\"}, \"email\": {\"type\": \"string\"}, \"age\": {\"type\": \"number\"}, \"department\": {\"type\": \"string\"}}, \"required\": [\"name\", \"email\", \"department\"]}"
    }
}
```

**Authorization**: Only contract owner can update the schema
**Effect**: Replaces the current schema with the new schema definition
**Validation**: New schema must be valid JSON and properly formatted

## QueryMsg

### ValidateData
Validates provided data against the stored schema.

```rust
#[returns(ValidateDataResponse)]
ValidateData {
    data: String,
}

pub enum ValidateDataResponse {
    Valid,
    Invalid { msg: String },
}
```

```json
{
    "validate_data": {
        "data": "{\"name\": \"John Doe\", \"email\": \"john@example.com\", \"age\": 30}"
    }
}
```

**Valid Response:**
```json
"Valid"
```

**Invalid Response:**
```json
{
    "Invalid": {
        "msg": "Data structure does not match the basic schema types."
    }
}
```

### GetSchema
Returns the current stored schema.

```rust
#[returns(GetSchemaResponse)]
GetSchema {}

pub struct GetSchemaResponse {
    pub schema: String,
}
```

```json
{
    "get_schema": {}
}
```

**Response:**
```json
{
    "schema": "{\"type\": \"object\", \"properties\": {\"name\": {\"type\": \"string\"}, \"email\": {\"type\": \"string\"}}, \"required\": [\"name\", \"email\"]}"
}
```

## Usage Examples

### User Profile Schema
```json
{
    "schema_json_string": "{\"type\": \"object\", \"properties\": {\"username\": {\"type\": \"string\"}, \"email\": {\"type\": \"string\"}, \"age\": {\"type\": \"number\"}, \"verified\": {\"type\": \"boolean\"}, \"tags\": {\"type\": \"array\", \"items\": {\"type\": \"string\"}}}, \"required\": [\"username\", \"email\"]}"
}
```

### Product Catalog Schema
```json
{
    "schema_json_string": "{\"type\": \"object\", \"properties\": {\"product_id\": {\"type\": \"string\"}, \"name\": {\"type\": \"string\"}, \"price\": {\"type\": \"number\"}, \"in_stock\": {\"type\": \"boolean\"}, \"categories\": {\"type\": \"array\", \"items\": {\"type\": \"string\"}}, \"specifications\": {\"type\": \"object\", \"properties\": {\"weight\": {\"type\": \"number\"}, \"dimensions\": {\"type\": \"string\"}}}}, \"required\": [\"product_id\", \"name\", \"price\"]}"
}
```

### Survey Response Schema
```json
{
    "schema_json_string": "{\"type\": \"object\", \"properties\": {\"respondent_id\": {\"type\": \"string\"}, \"responses\": {\"type\": \"array\", \"items\": {\"type\": \"object\", \"properties\": {\"question_id\": {\"type\": \"number\"}, \"answer\": {\"type\": \"string\"}, \"rating\": {\"type\": \"number\"}}, \"required\": [\"question_id\", \"answer\"]}}, \"completed_at\": {\"type\": \"string\"}}, \"required\": [\"respondent_id\", \"responses\"]}"
}
```

## Operational Examples

### Validate User Data
```json
{
    "validate_data": {
        "data": "{\"username\": \"alice123\", \"email\": \"alice@example.com\", \"age\": 28, \"verified\": true, \"tags\": [\"developer\", \"blockchain\"]}"
    }
}
```
**Result**: `"Valid"`

### Validate Invalid Data (Missing Required Field)
```json
{
    "validate_data": {
        "data": "{\"username\": \"bob456\", \"age\": 35, \"verified\": false}"
    }
}
```
**Result**: `{"Invalid": {"msg": "Data structure does not match the basic schema types."}}`

### Validate Invalid Data (Wrong Type)
```json
{
    "validate_data": {
        "data": "{\"username\": \"charlie789\", \"email\": \"charlie@example.com\", \"age\": \"thirty-two\", \"verified\": true}"
    }
}
```
**Result**: `{"Invalid": {"msg": "Data structure does not match the basic schema types."}}`

### Update Schema
```json
{
    "update_schema": {
        "new_schema_json_string": "{\"type\": \"object\", \"properties\": {\"username\": {\"type\": \"string\"}, \"email\": {\"type\": \"string\"}, \"age\": {\"type\": \"number\"}, \"verified\": {\"type\": \"boolean\"}, \"profile\": {\"type\": \"object\", \"properties\": {\"bio\": {\"type\": \"string\"}, \"location\": {\"type\": \"string\"}}}}, \"required\": [\"username\", \"email\"]}"
    }
}
```

### Get Current Schema
```json
{
    "get_schema": {}
}
```

## Integration Patterns

### With Form ADO
Schema ADO provides validation services for form submissions:

```json
{
    "components": [
        {
            "name": "user_schema",
            "ado_type": "schema",
            "component_type": {
                "new": {
                    "schema_json_string": "{\"type\": \"object\", \"properties\": {\"name\": {\"type\": \"string\"}, \"email\": {\"type\": \"string\"}, \"department\": {\"type\": \"string\"}}, \"required\": [\"name\", \"email\"]}"
                }
            }
        },
        {
            "name": "user_form",
            "ado_type": "form",
            "component_type": {
                "new": {
                    "schema_ado_address": "./user_schema",
                    "form_config": {
                        "allow_multiple_submissions": false,
                        "allow_edit_submission": true
                    }
                }
            }
        }
    ]
}
```

### API Validation Service
For validating API request/response data:

1. **Deploy schema ADO** with API endpoint data schemas
2. **Configure validation rules** for request and response formats
3. **Integrate with applications** for real-time validation
4. **Update schemas** as API requirements evolve
5. **Maintain data consistency** across service integrations

### Configuration Management
For application configuration validation:

1. **Define configuration schemas** with required settings and types
2. **Validate configuration data** before deployment
3. **Ensure type safety** for configuration values
4. **Support schema evolution** through update mechanisms
5. **Maintain configuration integrity** across environments

### Contract Integration
For inter-contract data validation:

1. **Deploy shared schema ADO** for common data formats
2. **Reference schema from multiple contracts** for validation
3. **Ensure data consistency** across contract interactions
4. **Validate message payloads** before processing
5. **Maintain protocol compliance** through schema enforcement

### Data Collection Systems
For structured data collection:

1. **Create collection schemas** for specific data types
2. **Validate submissions** in real-time during collection
3. **Ensure data quality** before storage or processing
4. **Support complex validation** with nested objects and arrays
5. **Maintain data standards** across collection campaigns

## Advanced Features

### **Complex Validation Support**
- **Nested objects**: Validate deeply nested object structures
- **Array validation**: Validate array contents and structure
- **Required fields**: Enforce required property validation
- **Type safety**: Strict type checking for all data types
- **Recursive validation**: Handle complex nested data patterns

### **Schema Evolution**
- **Dynamic updates**: Update schemas without redeployment
- **Version management**: Manage schema changes over time
- **Backward compatibility**: Handle schema evolution gracefully
- **Administrative control**: Owner-only schema modification
- **Change tracking**: Track schema updates through events

### **Validation Services**
- **Real-time validation**: Immediate validation feedback
- **Detailed errors**: Comprehensive error reporting
- **Cross-contract support**: Provide validation to other contracts
- **Query interface**: Access validation services via queries
- **Integration ready**: Easy integration with other ADOs

### **Performance Optimization**
- **Efficient parsing**: Optimized JSON parsing and validation
- **Memory management**: Efficient storage and retrieval
- **Gas optimization**: Minimal gas usage for validation operations
- **Caching strategies**: Efficient schema storage and access
- **Scalable validation**: Handle complex schemas efficiently

## Security Features

### **Schema Integrity**
- **Validation enforcement**: Ensure schema validity before storage
- **Type safety**: Prevent invalid schema definitions
- **Format checking**: Validate JSON Schema format compliance
- **Administrative control**: Owner-only schema modification
- **Change protection**: Secure schema update mechanisms

### **Data Validation Security**
- **Input sanitization**: Safe parsing of validation data
- **Type checking**: Strict type validation without coercion
- **Error handling**: Safe error reporting without information leakage
- **Memory safety**: Secure memory management for validation operations
- **DoS protection**: Prevent denial-of-service through malformed data

### **Access Control**
- **Owner restrictions**: Only contract owner can modify schemas
- **Query permissions**: Public access to validation services
- **Administrative functions**: Secure administrative operations
- **Permission validation**: Verify permissions before schema updates
- **Unauthorized prevention**: Prevent unauthorized schema modifications

### **Validation Reliability**
- **Deterministic results**: Consistent validation across all executions
- **Error consistency**: Reliable error reporting and handling
- **State protection**: Maintain consistent contract state
- **Transaction safety**: Safe handling of all validation operations
- **Data integrity**: Ensure validation data is not corrupted

## Important Notes

- **Schema format**: Must follow standard JSON Schema specification
- **Type support**: Supports string, number, boolean, array, and object types
- **Required fields**: Enforced for object validation
- **Nested validation**: Supports complex nested structures
- **Owner privileges**: Only contract owner can update schemas
- **Public validation**: Anyone can query validation services
- **Error messages**: Validation errors provide basic structural feedback
- **JSON requirement**: All data must be valid JSON for validation

## Common Workflow

### 1. **Deploy Schema ADO**
```json
{
    "schema_json_string": "{\"type\": \"object\", \"properties\": {\"name\": {\"type\": \"string\"}, \"email\": {\"type\": \"string\"}, \"age\": {\"type\": \"number\"}}, \"required\": [\"name\", \"email\"]}"
}
```

### 2. **Validate Data**
```json
{
    "validate_data": {
        "data": "{\"name\": \"John Doe\", \"email\": \"john@example.com\", \"age\": 30}"
    }
}
```

### 3. **Get Current Schema**
```json
{
    "get_schema": {}
}
```

### 4. **Update Schema**
```json
{
    "update_schema": {
        "new_schema_json_string": "{\"type\": \"object\", \"properties\": {\"name\": {\"type\": \"string\"}, \"email\": {\"type\": \"string\"}, \"age\": {\"type\": \"number\"}, \"phone\": {\"type\": \"string\"}}, \"required\": [\"name\", \"email\"]}"
    }
}
```

### 5. **Validate Against Updated Schema**
```json
{
    "validate_data": {
        "data": "{\"name\": \"Jane Smith\", \"email\": \"jane@example.com\", \"age\": 25, \"phone\": \"+1234567890\"}"
    }
}
```

### 6. **Test Invalid Data**
```json
{
    "validate_data": {
        "data": "{\"name\": \"Bob Wilson\", \"age\": \"twenty-five\"}"
    }
}
```

The Schema ADO provides essential validation infrastructure for the Andromeda ecosystem, enabling robust data validation, type safety, and structural integrity enforcement across diverse applications and use cases.