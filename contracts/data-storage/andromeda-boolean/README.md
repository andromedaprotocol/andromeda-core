# Andromeda Boolean ADO

## Introduction

The Andromeda Boolean ADO is a simple but essential data storage contract that provides secure boolean value storage with configurable access control. It allows applications to store, retrieve, and manage boolean state with fine-grained permission controls, making it ideal for feature flags, voting mechanisms, approval systems, and any application requiring persistent boolean data.

<b>Ado_type:</b> boolean

## Why Boolean ADO

The Boolean ADO serves as a fundamental building block for applications requiring:

- **Feature Flags**: Enable/disable application features dynamically
- **Voting Systems**: Store individual vote choices (yes/no, approve/reject)
- **Approval Workflows**: Track approval states in multi-step processes
- **Settings Management**: Store user preferences and configuration options
- **Access Control**: Maintain permission flags and access states
- **Status Tracking**: Monitor on/off states for various system components
- **Conditional Logic**: Store boolean conditions for smart contract logic
- **User Preferences**: Save user choices and settings
- **Game State**: Track boolean game states (unlocked/locked, active/inactive)

The ADO supports three access control modes: **Private** (owner only), **Public** (anyone can set), and **Restricted** (configurable permissions).

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
    "restriction": "Private"
}
```

- **restriction**: Controls who can modify the boolean value
  - `"Private"`: Only contract owner can set the value
  - `"Public"`: Anyone can set the value
  - `"Restricted"`: Uses advanced permission system for access control

## ExecuteMsg

### SetValue
Sets the boolean value stored in the contract.

_**Note:** Permission requirements depend on the restriction setting configured during instantiation._

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

### DeleteValue
Removes the stored boolean value from the contract.

_**Note:** Permission requirements depend on the restriction setting._

```rust
DeleteValue {}
```

```json
{
    "delete_value": {}
}
```

### UpdateRestriction
Updates the access restriction for the boolean value operations.

_**Note:** Only contract owner can execute this operation._

```rust
UpdateRestriction {
    restriction: BooleanRestriction,
}
```

```json
{
    "update_restriction": {
        "restriction": "Public"
    }
}
```

## QueryMsg

### GetValue
Returns the current boolean value stored in the contract.

```rust
pub enum QueryMsg {
    #[returns(GetValueResponse)]
    GetValue {},
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

### GetDataOwner
Returns the address that currently owns the data (who set the value).

```rust
pub enum QueryMsg {
    #[returns(GetDataOwnerResponse)]
    GetDataOwner {},
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
    "owner": "andr1..."
}
```

## Access Control Modes

### Private Mode
- **Who can set value**: Only contract owner
- **Use case**: Sensitive settings, admin flags, owner-controlled features
- **Example**: Admin approval flags, system maintenance modes

```json
{
    "restriction": "Private"
}
```

### Public Mode
- **Who can set value**: Anyone
- **Use case**: Community voting, public polls, collaborative settings
- **Example**: Community feature requests, public voting systems

```json
{
    "restriction": "Public"
}
```

### Restricted Mode
- **Who can set value**: Uses Andromeda's permission system
- **Use case**: Complex permission schemes, role-based access
- **Example**: Multi-role approval systems, delegated permissions

```json
{
    "restriction": "Restricted"
}
```

## Usage Examples

### Feature Flag System
```json
{
    "restriction": "Private"
}
```
Owner can toggle features on/off:
```json
{
    "set_value": {
        "value": true
    }
}
```

### Public Voting
```json
{
    "restriction": "Public"
}
```
Anyone can set their vote:
```json
{
    "set_value": {
        "value": false
    }
}
```

### Approval Workflow
```json
{
    "restriction": "Restricted"
}
```
Only authorized approvers can set approval status:
```json
{
    "set_value": {
        "value": true
    }
}
```

### User Preference
```json
{
    "restriction": "Private"
}
```
User controls their own preference:
```json
{
    "set_value": {
        "value": true
    }
}
```

## Integration Patterns

### With App Contract
The Boolean ADO can be integrated into App contracts for state management:

```json
{
    "components": [
        {
            "name": "feature_flag",
            "ado_type": "boolean",
            "component_type": {
                "new": {
                    "restriction": "Private"
                }
            }
        },
        {
            "name": "user_preference",
            "ado_type": "boolean",
            "component_type": {
                "new": {
                    "restriction": "Public"
                }
            }
        }
    ]
}
```

### Voting Systems
For democratic decision-making:

1. **Deploy Boolean ADO** for each vote/proposal
2. **Set to Public mode** to allow all voters to participate
3. **Query final state** after voting period ends
4. **Track data owner** to see who cast the final deciding vote

### Feature Management
For application feature control:

1. **Deploy Boolean ADOs** for each feature flag
2. **Set to Private mode** for admin-controlled features
3. **Query feature states** before executing feature logic
4. **Update restrictions** as needed for different deployment stages

### Multi-Step Approvals
For workflow management:

1. **Deploy Boolean ADO** for each approval step
2. **Set appropriate restrictions** based on role requirements
3. **Query all approval states** before proceeding
4. **Track approval ownership** for audit purposes

## State Management

### Value Lifecycle
1. **Unset**: Initial state, no value stored
2. **Set**: Boolean value stored (true or false)
3. **Deleted**: Value removed, returns to unset state

### Data Ownership
- **Data Owner**: Address that most recently set the value
- **Contract Owner**: Address that owns the contract (different from data owner)
- **Permissions**: Based on restriction mode and contract ownership

## Error Handling

### Common Errors
- **Unauthorized**: Attempting to set value without proper permissions
- **No Value Set**: Querying value when none has been set
- **Invalid Restriction**: Using unsupported restriction type

### Permission Checks
The contract validates permissions based on:
1. **Restriction mode** set during instantiation
2. **Contract ownership** for restricted operations
3. **Andromeda permission system** for advanced access control

## Important Notes

- **Simple State**: Stores only a single boolean value per contract instance
- **Data Ownership Tracking**: Tracks who set the current value
- **Flexible Permissions**: Three different access control modes
- **Value Deletion**: Supports removing stored values
- **Audit Trail**: Can track data ownership changes
- **No History**: Only current value is stored, no historical data

## Example Workflows

### Feature Flag Toggle
```bash
# Deploy with private restriction
# Owner enables feature
{"set_value": {"value": true}}

# Check if feature is enabled
{"get_value": {}}
# Response: {"value": true}

# Owner disables feature
{"set_value": {"value": false}}
```

### Public Poll
```bash
# Deploy with public restriction
# User A votes yes
{"set_value": {"value": true}}

# User B votes no (overwrites A's vote)
{"set_value": {"value": false}}

# Check final result
{"get_value": {}}
# Response: {"value": false}

# Check who voted last
{"get_data_owner": {}}
# Response: {"owner": "andr1...user_b_address"}
```

The Boolean ADO provides a simple, secure way to store and manage boolean state in blockchain applications, with flexible access controls that can adapt to various use cases from private settings to public voting systems.