# Andromeda Primitive ADO

## Introduction

The Andromeda Primitive ADO is a versatile key-value data storage contract that provides type-safe storage for common blockchain data types. This contract acts as a decentralized database, enabling applications to store and retrieve structured data with configurable access controls. The primitive storage system supports multiple data types including numbers, addresses, tokens, and binary data, making it ideal for configuration management, state persistence, and inter-contract data sharing.

<b>Ado_type:</b> primitive

## Why Primitive ADO

The Primitive ADO serves as essential data infrastructure for applications requiring:

- **Configuration Management**: Store application settings and parameters with type safety
- **State Persistence**: Maintain long-term state across multiple contract interactions
- **Data Sharing**: Enable data sharing between multiple contracts and applications
- **Dynamic Parameters**: Store and update dynamic parameters without contract upgrades
- **User Preferences**: Store user-specific settings and preferences
- **Registry Services**: Create registries for addresses, names, and other identifiers
- **Oracle Data**: Store off-chain data feeds and oracle information
- **Game State**: Maintain game state, scores, and player information
- **Metadata Storage**: Store NFT metadata, token information, and asset details
- **Inter-Contract Communication**: Facilitate data exchange between different ADOs

The ADO provides flexible access control models to ensure data security and appropriate access permissions.

## Key Features

### **Type-Safe Storage**
- **Multiple data types**: Support for Uint128, Decimal, Coin, Addr, String, Bool, and Binary
- **Type validation**: Automatic validation of data types during storage
- **Type inference**: Easy type retrieval and casting for stored values
- **Safe conversion**: Built-in methods for safe type conversion and extraction
- **Comprehensive validation**: Input validation prevents invalid data storage

### **Access Control Models**
- **Private mode**: Only contract owner can read and write data
- **Public mode**: Anyone can read and write data
- **Restricted mode**: Only key owners can modify their specific keys
- **Dynamic restrictions**: Update access control modes as needed
- **Owner privileges**: Contract owner always has full access in all modes

### **Key Management**
- **Custom keys**: Use custom string keys for organized data storage
- **Default key**: Automatic default key when no specific key is provided
- **Key enumeration**: List all keys or keys owned by specific addresses
- **Key ownership**: Track ownership of keys in restricted mode
- **Flexible naming**: Support for hierarchical and namespace-like key structures

### **Data Operations**
- **Set values**: Store new values or update existing ones
- **Get values**: Retrieve stored values with type information
- **Delete values**: Remove values from storage
- **Type queries**: Query the type of stored values without retrieving data
- **Bulk operations**: Efficient handling of multiple data operations

## Data Types

### **Supported Primitives**
The contract supports storage of the following data types:

1. **Uint128**: Large unsigned integers for amounts, IDs, and counters
2. **Decimal**: Precise decimal numbers for rates, percentages, and calculations
3. **Coin**: Native blockchain tokens with denomination and amount
4. **Addr**: Validated blockchain addresses for contracts and users
5. **String**: Text data for names, descriptions, and identifiers
6. **Bool**: Boolean values for flags and settings
7. **Binary**: Raw binary data for complex data structures

### **Type Safety**
Each primitive type includes:
- **Validation**: Automatic validation during storage
- **Extraction**: Type-safe extraction methods
- **Conversion**: Safe conversion between compatible types
- **Error handling**: Clear error messages for type mismatches

### **Usage Examples by Type**

**Uint128**: Counters, amounts, IDs
```json
{
    "value": {
        "uint128": "1000000000000"
    }
}
```

**Decimal**: Rates, percentages, precise calculations
```json
{
    "value": {
        "decimal": "0.05"
    }
}
```

**Coin**: Token amounts with denominations
```json
{
    "value": {
        "coin": {
            "denom": "uandr",
            "amount": "1000000"
        }
    }
}
```

**Addr**: Validated addresses
```json
{
    "value": {
        "addr": "andr1user_address..."
    }
}
```

**String**: Text data
```json
{
    "value": {
        "string": "configuration_value"
    }
}
```

**Bool**: Boolean flags
```json
{
    "value": {
        "bool": true
    }
}
```

**Binary**: Base64-encoded binary data
```json
{
    "value": {
        "binary": "aGVsbG8gd29ybGQ="
    }
}
```

## InstantiateMsg

```rust
pub struct InstantiateMsg {
    pub restriction: PrimitiveRestriction,
}

pub enum PrimitiveRestriction {
    Private,
    Public,
    Restricted,
}
```

```json
{
    "restriction": "restricted"
}
```

**Restriction Types**:

**Private**: Only contract owner can read and write
```json
{
    "restriction": "private"
}
```

**Public**: Anyone can read and write
```json
{
    "restriction": "public"
}
```

**Restricted**: Only key owners can modify their keys
```json
{
    "restriction": "restricted"
}
```

**Access Control Behavior**:
- **Private**: Owner-only access for all operations
- **Public**: Universal read/write access
- **Restricted**: Read access for all, write access only for key owners
- **Owner Override**: Contract owner always has full access regardless of restriction

## ExecuteMsg

### SetValue
Stores a value with an optional key.

```rust
SetValue {
    key: Option<String>,
    value: Primitive,
}
```

```json
{
    "set_value": {
        "key": "user_config",
        "value": {
            "string": "configuration_data"
        }
    }
}
```

**Parameters**:
- **key**: Optional custom key (uses default key if not provided)
- **value**: Primitive value to store

**Access Control**:
- **Private**: Only contract owner can set values
- **Public**: Anyone can set values
- **Restricted**: Only key owner can modify existing keys, anyone can create new keys

**Validation**:
- All primitive types are validated during storage
- Empty strings, binaries, and invalid addresses are rejected
- Coins must have valid denominations
- Key ownership is tracked in restricted mode

### DeleteValue
Removes a stored value.

```rust
DeleteValue { key: Option<String> }
```

```json
{
    "delete_value": {
        "key": "old_config"
    }
}
```

**Parameters**:
- **key**: Optional key to delete (deletes default key if not provided)

**Access Control**:
- **Private**: Only contract owner can delete values
- **Public**: Anyone can delete any values
- **Restricted**: Only key owner can delete their keys

**Behavior**:
- Removes the key-value pair from storage
- No error if key doesn't exist
- Owner can always delete any key

### UpdateRestriction
Updates the access control restriction (owner-only).

```rust
UpdateRestriction { restriction: PrimitiveRestriction }
```

```json
{
    "update_restriction": {
        "restriction": "public"
    }
}
```

**Authorization**: Only contract owner can update restrictions
**Effect**: Changes apply immediately to all future operations
**Use Cases**: Evolve access control as application requirements change

## QueryMsg

### GetValue
Retrieves a stored value with its key.

```rust
#[returns(GetValueResponse)]
GetValue { key: Option<String> }

pub struct GetValueResponse {
    pub key: String,
    pub value: Primitive,
}
```

```json
{
    "get_value": {
        "key": "user_config"
    }
}
```

**Response:**
```json
{
    "key": "user_config",
    "value": {
        "string": "configuration_data"
    }
}
```

**Access Control**:
- All restriction modes allow reading values
- Returns key and value with full type information

### GetType
Returns the type of a stored value without the actual data.

```rust
#[returns(GetTypeResponse)]
GetType { key: Option<String> }

pub struct GetTypeResponse {
    pub value_type: String,
}
```

```json
{
    "get_type": {
        "key": "amount_config"
    }
}
```

**Response:**
```json
{
    "value_type": "Uint128"
}
```

**Usage**: Efficient type checking without data transfer for large values

### AllKeys
Returns all stored keys in the contract.

```rust
#[returns(Vec<String>)]
AllKeys {}
```

```json
{
    "all_keys": {}
}
```

**Response:**
```json
[
    "user_config",
    "system_setting",
    "rate_limit",
    "default_key"
]
```

**Usage**: Discover all available keys for enumeration and administration

### OwnerKeys
Returns all keys owned by a specific address (restricted mode only).

```rust
#[returns(Vec<String>)]
OwnerKeys { owner: AndrAddr }
```

```json
{
    "owner_keys": {
        "owner": "andr1user_address..."
    }
}
```

**Response:**
```json
[
    "user_preference_1",
    "user_setting_2",
    "user_data_3"
]
```

**Usage**: In restricted mode, find all keys owned by a specific address

## Usage Examples

### Configuration Management
```json
{
    "restriction": "private"
}
```

**Set Application Config:**
```json
{
    "set_value": {
        "key": "max_users",
        "value": {
            "uint128": "10000"
        }
    }
}
```

**Set Fee Rate:**
```json
{
    "set_value": {
        "key": "fee_rate",
        "value": {
            "decimal": "0.025"
        }
    }
}
```

**Set Admin Address:**
```json
{
    "set_value": {
        "key": "admin",
        "value": {
            "addr": "andr1admin_address..."
        }
    }
}
```

### User Preferences (Restricted Mode)
```json
{
    "restriction": "restricted"
}
```

**User Sets Preference:**
```json
{
    "set_value": {
        "key": "notification_enabled",
        "value": {
            "bool": true
        }
    }
}
```

**User Sets Theme:**
```json
{
    "set_value": {
        "key": "theme",
        "value": {
            "string": "dark"
        }
    }
}
```

### Public Registry
```json
{
    "restriction": "public"
}
```

**Register Address:**
```json
{
    "set_value": {
        "key": "alice.andr",
        "value": {
            "addr": "andr1alice_address..."
        }
    }
}
```

**Store Metadata:**
```json
{
    "set_value": {
        "key": "project_info",
        "value": {
            "binary": "eyJuYW1lIjoiUHJvamVjdCIsImRlc2NyaXB0aW9uIjoiLi4uIn0="
        }
    }
}
```

### Token Information
```json
{
    "set_value": {
        "key": "reward_token",
        "value": {
            "coin": {
                "denom": "ureward",
                "amount": "1000000000"
            }
        }
    }
}
```

## Operational Examples

### Read Configuration
```json
{
    "get_value": {
        "key": "max_users"
    }
}
```

### Check Data Type
```json
{
    "get_type": {
        "key": "fee_rate"
    }
}
```

### List All Settings
```json
{
    "all_keys": {}
}
```

### Find User's Keys
```json
{
    "owner_keys": {
        "owner": "andr1user_address..."
    }
}
```

### Update Access Control
```json
{
    "update_restriction": {
        "restriction": "public"
    }
}
```

### Remove Old Config
```json
{
    "delete_value": {
        "key": "deprecated_setting"
    }
}
```

## Integration Patterns

### With App Contract
Primitive storage can be integrated for application state management:

```json
{
    "components": [
        {
            "name": "app_config",
            "ado_type": "primitive",
            "component_type": {
                "new": {
                    "restriction": "private"
                }
            }
        },
        {
            "name": "user_data",
            "ado_type": "primitive", 
            "component_type": {
                "new": {
                    "restriction": "restricted"
                }
            }
        }
    ]
}
```

### Configuration Management
For application configuration:

1. **Deploy with private restriction** for administrative control
2. **Store configuration parameters** using descriptive keys
3. **Query configuration values** from other contracts
4. **Update parameters** as application evolves

### User Data Storage
For user-specific data:

1. **Deploy with restricted mode** for user data isolation
2. **Allow users to set preferences** using their own keys
3. **Enable user data queries** for personalization
4. **Maintain user privacy** through ownership controls

### Registry Services
For public registries:

1. **Deploy with public restriction** for open access
2. **Create naming standards** for consistent key formats
3. **Enable community contributions** to the registry
4. **Provide discovery mechanisms** through key enumeration

## Advanced Features

### **Type Extraction**
The Primitive enum provides safe type extraction methods:

```rust
// Safe extraction methods
primitive.try_get_uint128()    // Result<Uint128, ContractError>
primitive.try_get_string()     // Result<String, ContractError>
primitive.try_get_bool()       // Result<bool, ContractError>
primitive.try_get_decimal()    // Result<Decimal, ContractError>
primitive.try_get_coin()       // Result<Coin, ContractError>
primitive.try_get_addr()       // Result<Addr, ContractError>
primitive.try_get_binary()     // Result<Binary, ContractError>
```

### **Key Management**
- **Hierarchical keys**: Use dot notation for organized data (e.g., "user.alice.preferences")
- **Namespace separation**: Separate different data categories with prefixes
- **Default key handling**: Automatic default key when no key specified
- **Key ownership tracking**: Automatic ownership assignment in restricted mode

### **Access Control Evolution**
- **Dynamic restrictions**: Change access control as applications mature
- **Owner override**: Contract owner maintains access in all modes
- **Granular control**: Different restriction levels for different use cases
- **Migration support**: Seamless transition between access control modes

### **Data Validation**
- **Input validation**: Comprehensive validation for all primitive types
- **Address verification**: Blockchain address validation
- **Denomination checking**: Valid coin denomination enforcement
- **Empty value prevention**: Reject empty strings and binary data

## Security Features

### **Access Control**
- **Restriction enforcement**: Strict enforcement of access control rules
- **Owner privileges**: Contract owner always has administrative access
- **Key ownership**: Tracked ownership for restricted mode operations
- **Permission validation**: Validate permissions before all write operations

### **Data Integrity**
- **Type safety**: Strong typing prevents data corruption
- **Input validation**: Comprehensive validation of all inputs
- **Atomic operations**: All operations are atomic to prevent partial failures
- **State consistency**: Maintain consistent state across all operations

### **Privacy Protection**
- **Restricted mode**: Ensure users can only modify their own data
- **Owner isolation**: Separate owner data from user data when needed
- **Read permissions**: Consistent read access policies across modes
- **Data ownership**: Clear ownership semantics for all stored data

## Important Notes

- **Default key**: Uses "default" as key when no key is specified
- **Owner privileges**: Contract owner can always read/write/delete any key
- **Type safety**: All primitive types are validated during storage
- **Key ownership**: In restricted mode, first setter becomes the key owner
- **Persistent storage**: Data persists until explicitly deleted
- **Gas efficiency**: Optimized for efficient storage and retrieval operations
- **Binary encoding**: Binary data must be base64-encoded
- **Address validation**: All addresses are validated before storage

## Common Workflow

### 1. **Deploy Primitive Storage**
```json
{
    "restriction": "restricted"
}
```

### 2. **Store Configuration Data**
```json
{
    "set_value": {
        "key": "app_version",
        "value": {
            "string": "1.0.0"
        }
    }
}
```

### 3. **Store Numeric Settings**
```json
{
    "set_value": {
        "key": "max_supply",
        "value": {
            "uint128": "1000000000000"
        }
    }
}
```

### 4. **Store Address References**
```json
{
    "set_value": {
        "key": "treasury",
        "value": {
            "addr": "andr1treasury_address..."
        }
    }
}
```

### 5. **Query Stored Data**
```json
{
    "get_value": {
        "key": "app_version"
    }
}
```

### 6. **Check Data Type**
```json
{
    "get_type": {
        "key": "max_supply"
    }
}
```

### 7. **List All Keys**
```json
{
    "all_keys": {}
}
```

### 8. **Update Access Control**
```json
{
    "update_restriction": {
        "restriction": "public"
    }
}
```

The Primitive ADO provides essential data storage infrastructure for the Andromeda ecosystem, enabling type-safe, flexible, and secure storage solutions for a wide range of applications from simple configuration management to complex multi-user data systems.