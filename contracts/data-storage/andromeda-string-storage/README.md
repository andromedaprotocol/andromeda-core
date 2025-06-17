# Andromeda String Storage ADO

## Introduction

The Andromeda String Storage ADO is a fundamental data storage contract that provides secure storage and management of string values with configurable access controls and ownership tracking. This contract serves as a simple yet powerful building block for text data storage, configuration management, content storage, and any application requiring persistent string data with controlled access. The string storage system supports three access control modes and includes data ownership tracking, making it ideal for applications requiring secure string data management with flexible permission models.

<b>Ado_type:</b> string-storage

## Why String Storage ADO

The String Storage ADO serves as essential infrastructure for applications requiring:

- **Configuration Storage**: Store application configuration strings and settings
- **Content Management**: Store text content, descriptions, and metadata
- **Data Persistence**: Persistent storage for string data across contract interactions
- **User Data Storage**: Store user-generated text content with ownership tracking
- **Message Storage**: Store messages, notes, or communication data
- **Metadata Management**: Store metadata strings for other contracts or assets
- **Documentation Storage**: Store documentation strings or help text
- **Name/Label Storage**: Store names, labels, or identifiers
- **URL/Link Storage**: Store URLs, links, or references
- **Template Storage**: Store text templates or format strings

The ADO provides secure string storage with ownership tracking and flexible access controls for reliable text data management.

## Key Features

### **String Value Storage**
- **Text data storage**: Store and retrieve string values with type safety
- **Value validation**: Automatic validation of string inputs (non-empty requirement)
- **State persistence**: String values persist until explicitly changed or deleted
- **Atomic operations**: All value operations are atomic and consistent
- **Type conversion**: Seamless conversion between string types

### **Access Control Models**
- **Private mode**: Only contract owner can read and write string values
- **Public mode**: Anyone can read and write string values
- **Restricted mode**: Only data owner can modify values, everyone can read
- **Dynamic restrictions**: Update access control modes as needed
- **Owner privileges**: Contract owner can always modify access control settings

### **Data Ownership**
- **Ownership tracking**: Track who set the current string value
- **Owner queries**: Query the current data owner
- **Ownership transfer**: Ownership changes when value is updated by different address
- **Access validation**: Validate ownership for restricted mode operations
- **Permission inheritance**: Contract owner maintains administrative control

### **Administrative Controls**
- **Restriction updates**: Change access control modes (owner-only)
- **Value deletion**: Remove stored string value and ownership
- **Permission management**: Manage access permissions through standard Andromeda controls
- **Rate limiting**: Support for transaction rate limiting and taxation
- **Tax integration**: Integrated transaction fee handling and refunds

## Access Control Modes

### **Private Mode**
Only contract owner can read and write string values:
```rust
StringStorageRestriction::Private
```

### **Public Mode**
Anyone can read and write string values:
```rust
StringStorageRestriction::Public
```

### **Restricted Mode**
Only data owner can modify values, everyone can read:
```rust
StringStorageRestriction::Restricted
```

## InstantiateMsg

```rust
pub struct InstantiateMsg {
    pub restriction: StringStorageRestriction,
}

pub enum StringStorageRestriction {
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

**Parameters**:
- **restriction**: Access control mode for string operations
  - **Private**: Only contract owner can read and write values
  - **Public**: Anyone can read and write values
  - **Restricted**: Only data owner can modify values, anyone can read

**Access Control Behavior**:
- **Private**: Owner-only access for all operations
- **Public**: Universal read/write access
- **Restricted**: Data owner can modify, everyone can read
- **Owner Override**: Contract owner can always update restrictions

## ExecuteMsg

### SetValue
Sets the string value in storage.

```rust
SetValue {
    value: StringStorage,
}

pub enum StringStorage {
    String(String),
}
```

```json
{
    "set_value": {
        "value": {
            "String": "Hello, Andromeda!"
        }
    }
}
```

**Parameters**:
- **value**: String value to store (wrapped in StringStorage enum)

**Access Control**:
- **Private**: Only contract owner can set values
- **Public**: Anyone can set values
- **Restricted**: Only current data owner can modify, or anyone if no value exists

**Validation**:
- String cannot be empty
- Value must pass StringStorage validation

**Effects**:
- Stores the string value
- Updates data ownership to the sender
- Triggers any configured rate limiting or taxation
- May trigger transaction fee refunds

### DeleteValue
Removes the stored string value and ownership information.

```rust
DeleteValue {}
```

```json
{
    "delete_value": {}
}
```

**Access Control**:
- **Private**: Only contract owner can delete values
- **Public**: Anyone can delete values
- **Restricted**: Only current data owner can delete

**Effects**:
- Removes the string value from storage
- Removes data ownership information
- Returns contract to uninitialized state

### UpdateRestriction
Updates the access control restriction (owner-only).

```rust
UpdateRestriction {
    restriction: StringStorageRestriction,
}
```

```json
{
    "update_restriction": {
        "restriction": "public"
    }
}
```

**Authorization**: Only contract owner can update restrictions
**Effect**: Changes access control for all future operations
**Use Cases**: Evolve access control as application requirements change

## QueryMsg

### GetValue
Returns the current string value.

```rust
#[returns(GetValueResponse)]
GetValue {}

pub struct GetValueResponse {
    pub value: String,
}
```

```json
{
    "get_value": {}
}
```

**Response:**
```json
{
    "value": "Hello, Andromeda!"
}
```

**Access Control**: All restriction modes allow reading the string value

### GetDataOwner
Returns the address that owns the current data.

```rust
#[returns(GetDataOwnerResponse)]
GetDataOwner {}

pub struct GetDataOwnerResponse {
    pub owner: AndrAddr,
}
```

```json
{
    "get_data_owner": {}
}
```

**Response:**
```json
{
    "owner": "andr1data_owner_address..."
}
```

**Usage**: Determine who set the current string value for ownership validation

## Usage Examples

### Configuration Storage
```json
{
    "restriction": "private"
}
```

**Set Configuration:**
```json
{
    "set_value": {
        "value": {
            "String": "max_users=1000,timeout=30,debug=true"
        }
    }
}
```

**Read Configuration:**
```json
{
    "get_value": {}
}
```

### Content Management System
```json
{
    "restriction": "restricted"
}
```

**Store Content:**
```json
{
    "set_value": {
        "value": {
            "String": "Welcome to our platform! This is the main landing page content that users will see when they first visit our application."
        }
    }
}
```

**Update Content:**
```json
{
    "set_value": {
        "value": {
            "String": "Updated welcome message with new features and improved user experience!"
        }
    }
}
```

### Public Message Board
```json
{
    "restriction": "public"
}
```

**Post Message:**
```json
{
    "set_value": {
        "value": {
            "String": "Community announcement: Join us for the upcoming blockchain meetup next Friday!"
        }
    }
}
```

**Read Message:**
```json
{
    "get_value": {}
}
```

**Check Author:**
```json
{
    "get_data_owner": {}
}
```

### Metadata Storage
```json
{
    "restriction": "restricted"
}
```

**Store Metadata:**
```json
{
    "set_value": {
        "value": {
            "String": "{\"name\":\"NFT Collection\",\"description\":\"Exclusive digital art collection\",\"image\":\"https://example.com/image.png\"}"
        }
    }
}
```

## Operational Examples

### Check Current Value
```json
{
    "get_value": {}
}
```

### Update Content
```json
{
    "set_value": {
        "value": {
            "String": "Updated content with new information"
        }
    }
}
```

### Clear Storage
```json
{
    "delete_value": {}
}
```

### Change Access Control
```json
{
    "update_restriction": {
        "restriction": "public"
    }
}
```

### Check Data Owner
```json
{
    "get_data_owner": {}
}
```

## Integration Patterns

### With App Contract
String storage can be integrated for content management:

```json
{
    "components": [
        {
            "name": "app_config",
            "ado_type": "string-storage",
            "component_type": {
                "new": {
                    "restriction": "private"
                }
            }
        },
        {
            "name": "user_content",
            "ado_type": "string-storage",
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
For application configuration storage:

1. **Use private mode** for sensitive configuration settings
2. **Store configuration strings** like API endpoints, feature flags, or settings
3. **Query configuration** from multiple application components
4. **Update configuration** through administrative interfaces
5. **Maintain configuration history** through ownership tracking

### Content Management Systems
For managing application content:

1. **Deploy with restricted mode** for content creator control
2. **Allow content creators** to store and update their content
3. **Enable public reading** for content consumption
4. **Track content ownership** through data owner queries
5. **Manage content lifecycle** with create/update/delete operations

### User Data Storage
For user-generated content:

1. **Use restricted mode** for user data isolation
2. **Allow users to store** their personal text data
3. **Maintain user privacy** through ownership-based access control
4. **Query user data** for personalized application behavior
5. **Enable data portability** through user-controlled updates and deletions

### Metadata and Documentation
For storing metadata and documentation:

1. **Store contract metadata** with descriptive information
2. **Maintain documentation strings** for help systems
3. **Track metadata changes** through ownership records
4. **Enable metadata queries** for discovery and integration
5. **Support metadata evolution** through update mechanisms

## Advanced Features

### **Data Ownership Tracking**
- **Owner identification**: Track who set the current string value
- **Ownership queries**: Query current data owner for validation
- **Access validation**: Use ownership for restricted mode access control
- **Automatic updates**: Ownership automatically updates when value changes

### **String Value Management**
- **Type safety**: Strong typing with StringStorage enum wrapper
- **Validation enforcement**: Automatic validation of string inputs
- **Empty string prevention**: Prevent storage of empty strings
- **Type conversion**: Seamless conversion between string formats

### **Flexible Access Control**
- **Mode switching**: Dynamic switching between access control modes
- **Permission inheritance**: Contract owner maintains administrative control
- **Granular control**: Different access levels for different use cases
- **Migration support**: Seamless transition between access control modes

### **Transaction Management**
- **Rate limiting support**: Integration with Andromeda rate limiting system
- **Tax calculation**: Automatic tax calculation and refund handling
- **Economic controls**: Implement economic controls for value operations
- **Fee optimization**: Efficient handling of transaction fees and refunds

## Security Features

### **Access Control**
- **Restriction enforcement**: Strict enforcement of access control rules
- **Owner validation**: Verify ownership before allowing sensitive operations
- **Permission checking**: Comprehensive permission validation
- **Unauthorized prevention**: Prevent unauthorized access to string data

### **Data Integrity**
- **Type safety**: Strong typing prevents data corruption
- **Validation enforcement**: Automatic validation of all string inputs
- **Empty string prevention**: Prevent storage of invalid empty strings
- **State consistency**: Maintain consistent state across all operations

### **Ownership Protection**
- **Owner tracking**: Accurate tracking of data ownership
- **Access validation**: Verify ownership for restricted operations
- **Privacy protection**: Ensure users can only modify their own data
- **Administrative override**: Contract owner maintains ultimate control

### **Transaction Security**
- **Tax integration**: Secure handling of transaction fees and taxation
- **Refund processing**: Safe refund processing for overpaid transactions
- **Economic controls**: Protection against economic attacks
- **Fee validation**: Comprehensive validation of fee calculations

## Important Notes

- **Single string value**: Contract stores one string value at a time
- **Ownership tracking**: Data ownership automatically updates when value changes
- **Delete behavior**: Deleting value also removes ownership information
- **Access control**: Restriction mode affects all value operations
- **Owner privileges**: Contract owner can always update restrictions
- **Empty string prevention**: Cannot store empty strings
- **Type wrapper**: String values are wrapped in StringStorage enum
- **Rate limiting**: Supports transaction rate limiting and taxation

## Common Workflow

### 1. **Deploy String Storage**
```json
{
    "restriction": "restricted"
}
```

### 2. **Set Initial Value**
```json
{
    "set_value": {
        "value": {
            "String": "Initial content value"
        }
    }
}
```

### 3. **Query Current Value**
```json
{
    "get_value": {}
}
```

### 4. **Check Data Owner**
```json
{
    "get_data_owner": {}
}
```

### 5. **Update Value**
```json
{
    "set_value": {
        "value": {
            "String": "Updated content with new information"
        }
    }
}
```

### 6. **Change Access Control**
```json
{
    "update_restriction": {
        "restriction": "public"
    }
}
```

### 7. **Delete Value**
```json
{
    "delete_value": {}
}
```

### 8. **Verify Deletion**
```json
{
    "get_value": {}
}
```

The String Storage ADO provides essential string data storage infrastructure for the Andromeda ecosystem, enabling secure, flexible, and controlled string data management for a wide range of applications from simple configuration storage to complex content management systems.