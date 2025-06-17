# Andromeda Schema ADO

## Introduction

The Andromeda Schema ADO is a flexible data validation contract that stores JSON schemas and validates data structures against them. This ADO enables applications to enforce data consistency, validate user inputs, and ensure API contract compliance through standardized JSON schema validation with support for complex nested structures, required fields, and multiple data types.

<b>Ado_type:</b> schema

## Why Schema ADO

The Schema ADO serves as a critical data validation tool for applications requiring:

- **Form Validation**: Validate user form submissions against predefined schemas
- **API Data Contracts**: Enforce consistent data structures across API endpoints
- **Configuration Management**: Validate configuration files and settings
- **Data Integrity**: Ensure incoming data meets application requirements
- **Type Safety**: Provide runtime type checking for dynamic data
- **Integration Validation**: Validate data exchanges between different systems
- **User Input Sanitization**: Ensure user-provided data conforms to expected formats
- **Database Schema Enforcement**: Validate data before storage operations
- **Microservice Communication**: Validate inter-service data exchanges
- **Content Management**: Validate structured content and metadata

The ADO provides comprehensive JSON schema validation with support for primitive types, arrays, objects, nested structures, and required field validation for robust data quality assurance.

## Key Features

### **JSON Schema Storage**
- **Schema persistence**: Store complete JSON schemas in contract state
- **Schema updates**: Owner-controlled schema modification capability
- **Schema retrieval**: Query stored schemas for inspection and documentation
- **Flexible schemas**: Support for any valid JSON schema structure

### **Data Validation**
- **Type validation**: Support for string, number, boolean, array, and object types
- **Structure validation**: Validate complex nested data structures
- **Required fields**: Enforce required property validation for objects
- **Array validation**: Validate array elements against item schemas
- **Nested validation**: Recursive validation for deeply nested structures

### **Validation Engine**
- **Basic type matching**: Core validation logic for fundamental data types
- **Property validation**: Object property type and structure checking
- **Array item validation**: Element-by-element validation for arrays
- **Required field checking**: Automatic enforcement of required properties
- **Error reporting**: Detailed validation failure messages

### **Access Control**
- **Owner restrictions**: Schema updates restricted to contract owner
- **Public validation**: Anyone can validate data against stored schemas
- **Schema visibility**: Public access to stored schemas for transparency
- **Controlled evolution**: Prevent unauthorized schema modifications

## Validation Logic

### **Type Validation**
The Schema ADO validates data against the following JSON schema types:

#### **Primitive Types**
- **string**: Validates string values
- **number**: Validates numeric values (integers and floats)
- **boolean**: Validates boolean true/false values

#### **Complex Types**
- **array**: Validates arrays with optional item schema validation
- **object**: Validates objects with property and required field validation

#### **Nested Validation**
- **Deep validation**: Recursively validates nested objects and arrays
- **Property matching**: Validates each object property against its schema
- **Item validation**: Validates each array element against item schema

## InstantiateMsg

```rust
pub struct InstantiateMsg {
    pub schema_json_string: String,
}
```

```json
{
    "schema_json_string": "{\"type\":\"object\",\"properties\":{\"name\":{\"type\":\"string\"},\"age\":{\"type\":\"number\"},\"email\":{\"type\":\"string\"}},\"required\":[\"name\",\"age\"]}"
}
```

**Parameters**:
- **schema_json_string**: Complete JSON schema as a string
  - Must be valid JSON format
  - Can include any JSON schema specification
  - Supports nested objects, arrays, and type definitions
  - Required fields can be specified in "required" array

## ExecuteMsg

### UpdateSchema
Updates the stored JSON schema (owner-only operation).

_**Note:** Only the contract owner can update the schema._

```rust
UpdateSchema {
    new_schema_json_string: String,
}
```

```json
{
    "update_schema": {
        "new_schema_json_string": "{\"type\":\"object\",\"properties\":{\"name\":{\"type\":\"string\"},\"age\":{\"type\":\"number\"},\"email\":{\"type\":\"string\"},\"phone\":{\"type\":\"string\"}},\"required\":[\"name\",\"age\",\"email\"]}"
    }
}
```

**Parameters**:
- **new_schema_json_string**: Updated JSON schema string
  - Must be valid JSON format
  - Completely replaces the existing schema
  - All future validations will use the new schema

## QueryMsg

### ValidateData
Validates provided data against the stored schema.

```rust
pub enum QueryMsg {
    #[returns(ValidateDataResponse)]
    ValidateData { data: String },
}
```

```json
{
    "validate_data": {
        "data": "{\"name\":\"Alice Johnson\",\"age\":30,\"email\":\"alice@example.com\"}"
    }
}
```

**Response:**
```json
{
    "valid": {}
}
```

Or for invalid data:
```json
{
    "invalid": {
        "msg": "Data structure does not match the basic schema types."
    }
}
```

### GetSchema
Returns the currently stored JSON schema.

```rust
pub enum QueryMsg {
    #[returns(GetSchemaResponse)]
    GetSchema {},
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
    "schema": "{\"type\":\"object\",\"properties\":{\"name\":{\"type\":\"string\"},\"age\":{\"type\":\"number\"},\"email\":{\"type\":\"string\"}},\"required\":[\"name\",\"age\"]}"
}
```

## Schema Examples

### User Profile Schema
```json
{
    "type": "object",
    "properties": {
        "name": {"type": "string"},
        "age": {"type": "number"},
        "email": {"type": "string"},
        "active": {"type": "boolean"},
        "tags": {
            "type": "array",
            "items": {"type": "string"}
        }
    },
    "required": ["name", "age", "email"]
}
```

**Valid Data:**
```json
{
    "name": "Alice Johnson",
    "age": 30,
    "email": "alice@example.com",
    "active": true,
    "tags": ["developer", "blockchain"]
}
```

### Product Schema
```json
{
    "type": "object",
    "properties": {
        "id": {"type": "number"},
        "title": {"type": "string"},
        "price": {"type": "number"},
        "categories": {
            "type": "array",
            "items": {"type": "string"}
        },
        "metadata": {
            "type": "object",
            "properties": {
                "weight": {"type": "number"},
                "dimensions": {"type": "string"}
            },
            "required": ["weight"]
        }
    },
    "required": ["id", "title", "price"]
}
```

**Valid Data:**
```json
{
    "id": 12345,
    "title": "Wireless Headphones",
    "price": 99.99,
    "categories": ["electronics", "audio"],
    "metadata": {
        "weight": 0.25,
        "dimensions": "20x15x8 cm"
    }
}
```

### Event Schema
```json
{
    "type": "object",
    "properties": {
        "event_id": {"type": "string"},
        "timestamp": {"type": "number"},
        "event_type": {"type": "string"},
        "participants": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "address": {"type": "string"},
                    "role": {"type": "string"}
                },
                "required": ["address", "role"]
            }
        },
        "data": {"type": "object"}
    },
    "required": ["event_id", "timestamp", "event_type"]
}
```

### Configuration Schema
```json
{
    "type": "object",
    "properties": {
        "app_name": {"type": "string"},
        "version": {"type": "string"},
        "features": {
            "type": "object",
            "properties": {
                "authentication": {"type": "boolean"},
                "logging": {"type": "boolean"},
                "rate_limiting": {"type": "boolean"}
            },
            "required": ["authentication"]
        },
        "endpoints": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "method": {"type": "string"},
                    "timeout": {"type": "number"}
                },
                "required": ["path", "method"]
            }
        }
    },
    "required": ["app_name", "version", "features"]
}
```

## Usage Examples

### Form Validation Application
```json
{
    "schema_json_string": "{\"type\":\"object\",\"properties\":{\"first_name\":{\"type\":\"string\"},\"last_name\":{\"type\":\"string\"},\"email\":{\"type\":\"string\"},\"age\":{\"type\":\"number\"},\"newsletter\":{\"type\":\"boolean\"}},\"required\":[\"first_name\",\"last_name\",\"email\"]}"
}
```

### API Contract Validation
```json
{
    "schema_json_string": "{\"type\":\"object\",\"properties\":{\"request_id\":{\"type\":\"string\"},\"method\":{\"type\":\"string\"},\"params\":{\"type\":\"object\"},\"timestamp\":{\"type\":\"number\"}},\"required\":[\"request_id\",\"method\"]}"
}
```

### Configuration Management
```json
{
    "schema_json_string": "{\"type\":\"object\",\"properties\":{\"database\":{\"type\":\"object\",\"properties\":{\"host\":{\"type\":\"string\"},\"port\":{\"type\":\"number\"},\"ssl\":{\"type\":\"boolean\"}},\"required\":[\"host\",\"port\"]},\"cache\":{\"type\":\"object\",\"properties\":{\"ttl\":{\"type\":\"number\"},\"max_size\":{\"type\":\"number\"}}}},\"required\":[\"database\"]}"
}
```

### Inventory Management
```json
{
    "schema_json_string": "{\"type\":\"object\",\"properties\":{\"sku\":{\"type\":\"string\"},\"name\":{\"type\":\"string\"},\"quantity\":{\"type\":\"number\"},\"supplier\":{\"type\":\"object\",\"properties\":{\"name\":{\"type\":\"string\"},\"contact\":{\"type\":\"string\"}},\"required\":[\"name\"]},\"tags\":{\"type\":\"array\",\"items\":{\"type\":\"string\"}}},\"required\":[\"sku\",\"name\",\"quantity\"]}"
}
```

## Integration Patterns

### With App Contract
The Schema ADO can be integrated into App contracts for data validation:

```json
{
    "components": [
        {
            "name": "user_validation_schema",
            "ado_type": "schema",
            "component_type": {
                "new": {
                    "schema_json_string": "{\"type\":\"object\",\"properties\":{\"username\":{\"type\":\"string\"},\"role\":{\"type\":\"string\"}},\"required\":[\"username\",\"role\"]}"
                }
            }
        }
    ]
}
```

### Form Processing Pipeline
For data validation in form processing:

1. **Deploy schema contract** with form validation rules
2. **Submit form data** through your application
3. **Validate data** using the ValidateData query
4. **Process valid data** or return validation errors
5. **Update schema** as requirements evolve

### API Gateway Integration
For API request/response validation:

1. **Store API schemas** for different endpoints
2. **Validate incoming requests** against stored schemas
3. **Ensure response consistency** through validation
4. **Maintain API contracts** with schema updates

### Configuration Validation
For application configuration management:

1. **Define configuration schema** at deployment
2. **Validate config updates** before applying changes
3. **Ensure system stability** through schema compliance
4. **Track configuration evolution** through schema history

## Security Features

### **Owner-Only Updates**
- **Restricted schema modification**: Only contract owner can update schemas
- **Controlled evolution**: Prevents unauthorized schema changes
- **Ownership verification**: Automatic sender validation for updates
- **State protection**: Schema integrity maintained through access control

### **Input Validation**
- **JSON format validation**: Ensures valid JSON input for schemas and data
- **Schema structure validation**: Validates schema format during updates
- **Type safety**: Prevents invalid data types from passing validation
- **Error handling**: Graceful handling of malformed inputs

### **Data Integrity**
- **Consistent validation**: All data validated against same stored schema
- **Atomic operations**: Schema updates are atomic transactions
- **State consistency**: Schema and validation state always synchronized
- **Deterministic results**: Same data always produces same validation result

## Validation Response Types

### ValidateDataResponse::Valid
Returned when data successfully passes all validation checks:
- All required fields are present
- All field types match schema definitions
- All nested structures validate correctly
- Array items conform to item schema

### ValidateDataResponse::Invalid
Returned when data fails validation with error message:
- Missing required fields
- Type mismatches (string vs number, etc.)
- Invalid nested structure
- Array item validation failures

## Important Notes

- **JSON format required**: Both schemas and data must be valid JSON strings
- **Owner-only updates**: Schema modifications restricted to contract owner
- **Public validation**: Anyone can validate data against stored schemas
- **Basic type support**: Currently supports string, number, boolean, array, and object types
- **Nested validation**: Supports recursive validation of complex nested structures
- **Required fields**: Enforces required property validation for objects
- **Case sensitive**: Property names and values are case-sensitive
- **Schema replacement**: UpdateSchema completely replaces existing schema

## Common Workflow

### 1. **Deploy with Schema**
```json
{
    "schema_json_string": "{\"type\":\"object\",\"properties\":{\"name\":{\"type\":\"string\"},\"value\":{\"type\":\"number\"}},\"required\":[\"name\"]}"
}
```

### 2. **Validate Data**
```json
{
    "validate_data": {
        "data": "{\"name\":\"test\",\"value\":42}"
    }
}
```

### 3. **Get Current Schema**
```json
{
    "get_schema": {}
}
```

### 4. **Update Schema (Owner)**
```json
{
    "update_schema": {
        "new_schema_json_string": "{\"type\":\"object\",\"properties\":{\"name\":{\"type\":\"string\"},\"value\":{\"type\":\"number\"},\"category\":{\"type\":\"string\"}},\"required\":[\"name\",\"category\"]}"
    }
}
```

The Schema ADO provides a robust foundation for data validation and schema management, enabling applications to maintain data integrity, enforce API contracts, and provide consistent user experiences through comprehensive JSON schema validation capabilities.