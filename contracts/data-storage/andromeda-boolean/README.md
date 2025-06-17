# Andromeda Boolean ADO

## Introduction

The Andromeda Boolean ADO is a fundamental data storage contract that provides secure storage and management of boolean (true/false) values with configurable access controls. This contract serves as a basic building block for conditional logic, feature flags, voting mechanisms, and state management in blockchain applications. The boolean storage system supports three access control modes and includes data ownership tracking, making it ideal for applications requiring simple but secure boolean state management.

<b>Ado_type:</b> boolean

## Why Boolean ADO

The Boolean ADO serves as essential infrastructure for applications requiring:

- **Feature Flags**: Control feature availability through boolean toggles
- **Configuration Management**: Store boolean configuration settings and preferences
- **Conditional Logic**: Implement conditional workflows based on boolean states
- **Voting Mechanisms**: Simple yes/no voting systems with state persistence
- **Permission Flags**: Manage permission states and access controls
- **Status Tracking**: Track boolean status indicators (active/inactive, enabled/disabled)
- **Circuit Breakers**: Implement circuit breaker patterns for system protection
- **Game State**: Boolean game states like power-ups, achievements, or conditions
- **Protocol States**: Track protocol states like paused/unpaused, emergency modes
- **User Preferences**: Store user preference settings as boolean values

The ADO provides secure boolean storage with ownership tracking and flexible access controls for reliable state management.

## Key Features

### **Boolean Value Storage**
- **True/false values**: Store and retrieve boolean values with type safety
- **State persistence**: Boolean values persist until explicitly changed or deleted
- **Atomic operations**: All value operations are atomic and consistent
- **Type safety**: Strong typing prevents invalid value storage
- **Value validation**: Automatic validation of boolean inputs

### **Access Control Models**
- **Private mode**: Only contract owner can read and write the boolean value
- **Public mode**: Anyone can read and write the boolean value
- **Restricted mode**: Only data owner can modify the value, everyone can read
- **Dynamic restrictions**: Update access control modes as needed
- **Owner privileges**: Contract owner can always modify access control settings

### **Data Ownership**
- **Ownership tracking**: Track who set the current boolean value
- **Owner queries**: Query the current data owner
- **Ownership transfer**: Ownership changes when value is updated by different address
- **Access validation**: Validate ownership for restricted mode operations
- **Permission inheritance**: Contract owner maintains administrative control

### **Administrative Controls**
- **Restriction updates**: Change access control modes (owner-only)
- **Value deletion**: Remove stored boolean value and ownership
- **Permission management**: Manage access permissions through standard Andromeda controls
- **Rate limiting**: Support for transaction rate limiting and taxation

## Access Control Modes

### **Private Mode**
Only contract owner can read and write the boolean value:
```rust
BooleanRestriction::Private
```

### **Public Mode**
Anyone can read and write the boolean value:
```rust
BooleanRestriction::Public
```

### **Restricted Mode**
Only data owner can modify the value, everyone can read:
```rust
BooleanRestriction::Restricted
```

## InstantiateMsg

```rust
pub struct InstantiateMsg {
    pub restriction: BooleanRestriction,
}

pub enum BooleanRestriction {
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
- **restriction**: Access control mode for boolean operations
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
Sets the boolean value in storage.

```rust
SetValue {
    value: bool,
}
```

```json
{
    "set_value": {
        "value": true
    }
}
```

**Parameters**:
- **value**: Boolean value to store (true or false)

**Access Control**:
- **Private**: Only contract owner can set values
- **Public**: Anyone can set values
- **Restricted**: Only current data owner can modify, or anyone if no value exists

**Effects**:
- Stores the boolean value
- Updates data ownership to the sender
- Triggers any configured rate limiting or taxation

### DeleteValue
Removes the stored boolean value and ownership information.

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
- Removes the boolean value from storage
- Removes data ownership information
- Returns contract to uninitialized state

### UpdateRestriction
Updates the access control restriction (owner-only).

```rust
UpdateRestriction {
    restriction: BooleanRestriction,
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
Returns the current boolean value.

```rust
#[returns(GetValueResponse)]
GetValue {}

pub struct GetValueResponse {
    pub value: bool,
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
    "value": true
}
```

**Access Control**: All restriction modes allow reading the boolean value

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

**Usage**: Determine who set the current boolean value for ownership validation

## Usage Examples

### Feature Flag System
```json
{
    "restriction": "private"
}
```

**Enable Feature:**
```json
{
    "set_value": {
        "value": true
    }
}
```

**Disable Feature:**
```json
{
    "set_value": {
        "value": false
    }
}
```

**Check Feature Status:**
```json
{
    "get_value": {}
}
```

### Voting System
```json
{
    "restriction": "public"
}
```

**Cast Yes Vote:**
```json
{
    "set_value": {
        "value": true
    }
}
```

**Cast No Vote:**
```json
{
    "set_value": {
        "value": false
    }
}
```

**Check Vote Result:**
```json
{
    "get_value": {}
}
```

### User Preference Storage
```json
{
    "restriction": "restricted"
}
```

**Set Notification Preference:**
```json
{
    "set_value": {
        "value": true
    }
}
```

**Query Preference:**
```json
{
    "get_value": {}
}
```

**Check Who Set Preference:**
```json
{
    "get_data_owner": {}
}
```

### Circuit Breaker Pattern
```json
{
    "restriction": "private"
}
```

**Activate Circuit Breaker:**
```json
{
    "set_value": {
        "value": true
    }
}
```

**Deactivate Circuit Breaker:**
```json
{
    "set_value": {
        "value": false
    }
}
```

**Reset Circuit Breaker:**
```json
{
    "delete_value": {}
}
```

## Operational Examples

### Check Current State
```json
{
    "get_value": {}
}
```

### Toggle State
```json
{
    "set_value": {
        "value": false
    }
}
```

### Clear State
```json
{
    "delete_value": {}
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

### Check Data Owner
```json
{
    "get_data_owner": {}
}
```

## Integration Patterns

### With App Contract
Boolean storage can be integrated for state management:

```json
{
    "components": [
        {
            "name": "feature_flags",
            "ado_type": "boolean",
            "component_type": {
                "new": {
                    "restriction": "private"
                }
            }
        },
        {
            "name": "user_preferences",
            "ado_type": "boolean",
            "component_type": {
                "new": {
                    "restriction": "restricted"
                }
            }
        }
    ]
}
```

### Feature Flag Management
For controlling application features:

1. **Deploy with private restriction** for administrative control
2. **Set feature flags** based on development and deployment needs
3. **Query flags from applications** to control feature availability
4. **Update flags dynamically** without redeploying applications

### Configuration Management
For application configuration:

1. **Use private mode** for sensitive configuration settings
2. **Store boolean settings** like debug modes, maintenance windows
3. **Query settings** from multiple application components
4. **Update configuration** through administrative interfaces

### User Preference Systems
For managing user preferences:

1. **Deploy with restricted mode** for user data isolation
2. **Allow users to set preferences** through application interfaces
3. **Maintain user privacy** through ownership-based access control
4. **Query preferences** for personalized application behavior

### Voting and Governance
For simple voting mechanisms:

1. **Use public mode** for open voting scenarios
2. **Track vote ownership** through data owner queries
3. **Implement simple yes/no voting** with boolean values
4. **Reset votes** between voting rounds

## Advanced Features

### **Data Ownership Tracking**
- **Owner identification**: Track who set the current boolean value
- **Ownership queries**: Query current data owner for validation
- **Access validation**: Use ownership for restricted mode access control
- **Automatic updates**: Ownership automatically updates when value changes

### **Flexible Access Control**
- **Mode switching**: Dynamic switching between access control modes
- **Permission inheritance**: Contract owner maintains administrative control
- **Granular control**: Different access levels for different use cases
- **Migration support**: Seamless transition between access control modes

### **State Management**
- **Persistent storage**: Boolean values persist until explicitly changed
- **Atomic operations**: All state changes are atomic and consistent
- **Type safety**: Strong typing prevents invalid state transitions
- **State queries**: Easy access to current state information

### **Administrative Controls**
- **Owner privileges**: Contract owner can always modify settings
- **Restriction updates**: Dynamic access control management
- **Value deletion**: Complete state reset capabilities
- **Permission management**: Integration with Andromeda permission system

## Security Features

### **Access Control**
- **Restriction enforcement**: Strict enforcement of access control rules
- **Owner validation**: Verify ownership before allowing sensitive operations
- **Permission checking**: Comprehensive permission validation
- **Unauthorized prevention**: Prevent unauthorized access to boolean data

### **Data Integrity**
- **Type safety**: Strong typing prevents data corruption
- **Atomic operations**: All operations are atomic to prevent partial failures
- **State consistency**: Maintain consistent state across all operations
- **Input validation**: Validate all inputs before storage

### **Ownership Protection**
- **Owner tracking**: Accurate tracking of data ownership
- **Access validation**: Verify ownership for restricted operations
- **Privacy protection**: Ensure users can only modify their own data
- **Administrative override**: Contract owner maintains ultimate control

### **Rate Limiting Support**
- **Transaction taxation**: Support for transaction fees and rate limiting
- **Tax calculation**: Automatic tax calculation and refund handling
- **Economic controls**: Implement economic controls for value operations
- **Resource management**: Manage contract resource usage

## Important Notes

- **Single boolean value**: Contract stores one boolean value at a time
- **Ownership tracking**: Data ownership automatically updates when value changes
- **Delete behavior**: Deleting value also removes ownership information
- **Access control**: Restriction mode affects all value operations
- **Owner privileges**: Contract owner can always update restrictions
- **Type safety**: Only boolean values (true/false) can be stored
- **State persistence**: Values persist until explicitly changed or deleted
- **Empty state**: Contract can exist without a stored value

## Common Workflow

### 1. **Deploy Boolean Storage**
```json
{
    "restriction": "restricted"
}
```

### 2. **Set Initial Value**
```json
{
    "set_value": {
        "value": true
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
        "value": false
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

The Boolean ADO provides essential boolean storage infrastructure for the Andromeda ecosystem, enabling secure, flexible, and controlled boolean state management for a wide range of applications from simple feature flags to complex conditional logic systems.