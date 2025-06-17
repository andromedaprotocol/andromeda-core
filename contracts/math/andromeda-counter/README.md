# Andromeda Counter ADO

## Introduction

The Andromeda Counter ADO is a fundamental mathematical contract that provides a simple yet powerful counting mechanism with configurable increment and decrement operations. This contract maintains a numerical counter with customizable step sizes, access controls, and reset functionality. The counter serves as a building block for various applications requiring sequential numbering, voting systems, usage tracking, and state management with precise numerical controls.

<b>Ado_type:</b> counter

## Why Counter ADO

The Counter ADO serves as essential infrastructure for applications requiring:

- **Sequential Numbering**: Generate unique sequential IDs for NFTs, tickets, and records
- **Usage Tracking**: Track usage counts, access attempts, and interaction metrics
- **Voting Systems**: Implement simple voting mechanisms with customizable vote weights
- **State Management**: Maintain numerical state across smart contract interactions
- **Rate Limiting**: Track and limit the number of operations or requests
- **Progress Tracking**: Monitor progress through numerical milestones and achievements
- **Event Counting**: Count occurrences of specific events or conditions
- **Resource Management**: Track available resources, quotas, and allocations
- **Game Mechanics**: Implement score tracking, level progression, and achievement systems
- **Statistical Collection**: Gather numerical statistics and metrics

The ADO provides precise mathematical operations, overflow protection, and configurable access controls for reliable counting operations.

## Key Features

### **Flexible Counting Operations**
- **Increment operations**: Increase counter by configurable amounts
- **Decrement operations**: Decrease counter with underflow protection
- **Reset functionality**: Return counter to initial value
- **Customizable steps**: Set different increment and decrement amounts
- **Overflow protection**: Safe arithmetic operations prevent overflow errors

### **Access Control System**
- **Private mode**: Only contract owner and operators can modify counter
- **Public mode**: Anyone can perform counter operations
- **Permission validation**: Comprehensive access control enforcement
- **Owner privileges**: Contract owner can always modify settings
- **Operator support**: Designated operators can perform counter operations

### **State Management**
- **Initial value tracking**: Remember original counter starting value
- **Current value maintenance**: Track real-time counter value
- **Configuration persistence**: Store increment/decrement amounts
- **State queries**: Query all counter parameters and current state
- **Atomic operations**: All counter operations are atomic and consistent

### **Administrative Controls**
- **Dynamic configuration**: Update increment/decrement amounts
- **Access control updates**: Change between private and public modes
- **Reset capability**: Return to initial state when needed
- **Parameter queries**: Inspect all counter configuration and state

## Access Control Modes

### **Private Mode**
Only contract owner and operators can modify the counter:
```rust
CounterRestriction::Private
```

### **Public Mode**
Anyone can increment, decrement, or reset the counter:
```rust
CounterRestriction::Public
```

## InstantiateMsg

```rust
pub struct InstantiateMsg {
    pub restriction: CounterRestriction,
    pub initial_state: State,
}

pub struct State {
    pub initial_amount: Option<u64>,
    pub increase_amount: Option<u64>,
    pub decrease_amount: Option<u64>,
}

pub enum CounterRestriction {
    Private,
    Public,
}
```

```json
{
    "restriction": "private",
    "initial_state": {
        "initial_amount": 100,
        "increase_amount": 5,
        "decrease_amount": 3
    }
}
```

**Parameters**:
- **restriction**: Access control mode for counter operations
  - **Private**: Only owner/operators can modify counter
  - **Public**: Anyone can perform counter operations
- **initial_state**: Initial counter configuration
  - **initial_amount**: Starting counter value (default: 0)
  - **increase_amount**: Amount to add per increment (default: 1)
  - **decrease_amount**: Amount to subtract per decrement (default: 1)

**Default Values**:
- Initial amount: 0
- Increase amount: 1
- Decrease amount: 1

## ExecuteMsg

### Increment
Increases the counter by the configured increment amount.

```rust
Increment {}
```

```json
{
    "increment": {}
}
```

**Operation**: `current_amount = current_amount + increase_amount`
**Access Control**: Depends on restriction mode (private/public)
**Overflow Protection**: Returns error if operation would cause overflow

### Decrement
Decreases the counter by the configured decrement amount.

```rust
Decrement {}
```

```json
{
    "decrement": {}
}
```

**Operation**: `current_amount = max(0, current_amount - decrease_amount)`
**Access Control**: Depends on restriction mode (private/public)
**Underflow Protection**: Uses saturating subtraction (minimum value is 0)

### Reset
Resets the counter to its initial value.

```rust
Reset {}
```

```json
{
    "reset": {}
}
```

**Operation**: `current_amount = initial_amount`
**Access Control**: Depends on restriction mode (private/public)
**Effect**: Returns counter to the original starting value

### UpdateRestriction
Updates the access control restriction (owner-only).

```rust
UpdateRestriction {
    restriction: CounterRestriction,
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
**Effect**: Changes access control for counter operations
**Use Cases**: Evolve access control as application requirements change

### SetIncreaseAmount
Sets the amount added per increment operation (owner-only).

```rust
SetIncreaseAmount {
    increase_amount: u64,
}
```

```json
{
    "set_increase_amount": {
        "increase_amount": 10
    }
}
```

**Authorization**: Only contract owner can update increment amounts
**Effect**: Changes the step size for increment operations
**Validation**: Must be a valid u64 value

### SetDecreaseAmount
Sets the amount subtracted per decrement operation (owner-only).

```rust
SetDecreaseAmount {
    decrease_amount: u64,
}
```

```json
{
    "set_decrease_amount": {
        "decrease_amount": 2
    }
}
```

**Authorization**: Only contract owner can update decrement amounts
**Effect**: Changes the step size for decrement operations
**Validation**: Must be a valid u64 value

## QueryMsg

### GetCurrentAmount
Returns the current counter value.

```rust
#[returns(GetCurrentAmountResponse)]
GetCurrentAmount {}

pub struct GetCurrentAmountResponse {
    pub current_amount: u64,
}
```

```json
{
    "get_current_amount": {}
}
```

**Response:**
```json
{
    "current_amount": 157
}
```

### GetInitialAmount
Returns the initial counter value set at instantiation.

```rust
#[returns(GetInitialAmountResponse)]
GetInitialAmount {}

pub struct GetInitialAmountResponse {
    pub initial_amount: u64,
}
```

```json
{
    "get_initial_amount": {}
}
```

**Response:**
```json
{
    "initial_amount": 100
}
```

### GetIncreaseAmount
Returns the amount added per increment operation.

```rust
#[returns(GetIncreaseAmountResponse)]
GetIncreaseAmount {}

pub struct GetIncreaseAmountResponse {
    pub increase_amount: u64,
}
```

```json
{
    "get_increase_amount": {}
}
```

**Response:**
```json
{
    "increase_amount": 5
}
```

### GetDecreaseAmount
Returns the amount subtracted per decrement operation.

```rust
#[returns(GetDecreaseAmountResponse)]
GetDecreaseAmount {}

pub struct GetDecreaseAmountResponse {
    pub decrease_amount: u64,
}
```

```json
{
    "get_decrease_amount": {}
}
```

**Response:**
```json
{
    "decrease_amount": 3
}
```

### GetRestriction
Returns the current access control restriction.

```rust
#[returns(GetRestrictionResponse)]
GetRestriction {}

pub struct GetRestrictionResponse {
    pub restriction: CounterRestriction,
}
```

```json
{
    "get_restriction": {}
}
```

**Response:**
```json
{
    "restriction": "private"
}
```

## Usage Examples

### Simple Sequential Counter
```json
{
    "restriction": "public",
    "initial_state": {
        "initial_amount": 1,
        "increase_amount": 1,
        "decrease_amount": 1
    }
}
```

### Voting System Counter
```json
{
    "restriction": "public",
    "initial_state": {
        "initial_amount": 0,
        "increase_amount": 1,
        "decrease_amount": 1
    }
}
```

### Score Tracking System
```json
{
    "restriction": "private",
    "initial_state": {
        "initial_amount": 0,
        "increase_amount": 10,
        "decrease_amount": 5
    }
}
```

### Resource Quota Tracker
```json
{
    "restriction": "private",
    "initial_state": {
        "initial_amount": 1000,
        "increase_amount": 100,
        "decrease_amount": 50
    }
}
```

## Operational Examples

### Increment Counter
```json
{
    "increment": {}
}
```

### Decrement Counter
```json
{
    "decrement": {}
}
```

### Reset to Initial Value
```json
{
    "reset": {}
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

### Modify Increment Step
```json
{
    "set_increase_amount": {
        "increase_amount": 25
    }
}
```

### Modify Decrement Step
```json
{
    "set_decrease_amount": {
        "decrease_amount": 10
    }
}
```

## Query Examples

### Check Current Value
```json
{
    "get_current_amount": {}
}
```

### Check Initial Value
```json
{
    "get_initial_amount": {}
}
```

### Check Step Sizes
```json
{
    "get_increase_amount": {}
}
```

```json
{
    "get_decrease_amount": {}
}
```

### Check Access Control
```json
{
    "get_restriction": {}
}
```

## Integration Patterns

### With App Contract
Counter can be integrated for various tracking purposes:

```json
{
    "components": [
        {
            "name": "usage_counter",
            "ado_type": "counter",
            "component_type": {
                "new": {
                    "restriction": "private",
                    "initial_state": {
                        "initial_amount": 0,
                        "increase_amount": 1,
                        "decrease_amount": 1
                    }
                }
            }
        },
        {
            "name": "score_tracker",
            "ado_type": "counter",
            "component_type": {
                "new": {
                    "restriction": "public",
                    "initial_state": {
                        "initial_amount": 0,
                        "increase_amount": 10,
                        "decrease_amount": 5
                    }
                }
            }
        }
    ]
}
```

### NFT Sequential Numbering
For generating sequential NFT token IDs:

1. **Deploy counter** with initial value 1 and increment of 1
2. **Set private restriction** for controlled minting
3. **Increment on each mint** to generate unique sequential IDs
4. **Query current value** to get next available token ID

### Voting System Implementation
For simple voting mechanisms:

1. **Set public restriction** to allow open voting
2. **Use increment for yes votes** and decrement for no votes
3. **Track vote counts** through current amount queries
4. **Reset between voting rounds** when needed

### Usage Rate Limiting
For tracking and limiting API or contract usage:

1. **Initialize with quota amount** as initial value
2. **Decrement on each usage** to track remaining quota
3. **Check current value** before allowing operations
4. **Reset periodically** to refresh quotas

### Game Score Management
For tracking player scores and achievements:

1. **Set score-appropriate step sizes** for point awards
2. **Use increment for achievements** and decrement for penalties
3. **Track high scores** through maximum value tracking
4. **Reset for new game sessions** when appropriate

## Advanced Features

### **Mathematical Safety**
- **Overflow protection**: Increment operations check for potential overflow
- **Underflow protection**: Decrement operations use saturating subtraction
- **Safe arithmetic**: All mathematical operations are checked for safety
- **Boundary conditions**: Proper handling of edge cases and limits

### **Flexible Step Sizes**
- **Configurable increments**: Set custom amounts for increment operations
- **Configurable decrements**: Set custom amounts for decrement operations
- **Dynamic updates**: Change step sizes without redeploying contract
- **Application-specific scaling**: Tailor step sizes to application needs

### **Access Control Management**
- **Dual mode system**: Switch between private and public access
- **Owner privileges**: Contract owner can always modify configuration
- **Operator support**: Multiple authorized operators for private mode
- **Permission validation**: Comprehensive access control enforcement

### **State Persistence**
- **Initial value tracking**: Maintain reference to original starting value
- **Current state management**: Accurate tracking of real-time counter value
- **Configuration storage**: Persistent storage of all counter parameters
- **Query accessibility**: Easy access to all state information

## Security Features

### **Access Control**
- **Permission validation**: Verify access rights before all operations
- **Owner restrictions**: Restrict configuration changes to contract owner
- **Mode enforcement**: Strict enforcement of private/public access modes
- **Operator validation**: Verify operator permissions in private mode

### **Mathematical Security**
- **Overflow prevention**: Prevent integer overflow in increment operations
- **Underflow protection**: Safe subtraction with minimum value enforcement
- **Safe arithmetic**: All operations use checked mathematical functions
- **Boundary validation**: Proper handling of numerical limits

### **State Integrity**
- **Atomic operations**: All counter operations are atomic and consistent
- **State validation**: Comprehensive validation of all state changes
- **Data persistence**: Reliable storage and retrieval of counter state
- **Error handling**: Graceful handling of edge cases and errors

### **Configuration Protection**
- **Owner-only updates**: Restrict configuration changes to authorized users
- **Validation checks**: Validate all configuration parameters
- **State consistency**: Maintain consistent state across configuration changes
- **Access logging**: Track all configuration and operational changes

## Important Notes

- **Integer arithmetic**: Counter uses u64 integers with safe arithmetic operations
- **Underflow protection**: Decrement operations cannot reduce value below 0
- **Overflow protection**: Increment operations check for potential overflow
- **Access control**: Restriction mode affects all counter operations (increment, decrement, reset)
- **Configuration changes**: Only contract owner can modify step sizes and restrictions
- **State persistence**: All counter state persists between operations
- **Reset behavior**: Reset returns counter to initial value, not necessarily 0
- **Step size flexibility**: Different increment and decrement amounts enable asymmetric counting

## Common Workflow

### 1. **Deploy Counter**
```json
{
    "restriction": "private",
    "initial_state": {
        "initial_amount": 0,
        "increase_amount": 1,
        "decrease_amount": 1
    }
}
```

### 2. **Increment Counter**
```json
{
    "increment": {}
}
```

### 3. **Check Current Value**
```json
{
    "get_current_amount": {}
}
```

### 4. **Modify Step Size**
```json
{
    "set_increase_amount": {
        "increase_amount": 5
    }
}
```

### 5. **Decrement Counter**
```json
{
    "decrement": {}
}
```

### 6. **Reset to Initial**
```json
{
    "reset": {}
}
```

### 7. **Change Access Mode**
```json
{
    "update_restriction": {
        "restriction": "public"
    }
}
```

### 8. **Query Configuration**
```json
{
    "get_increase_amount": {}
}
```

The Counter ADO provides essential counting infrastructure for the Andromeda ecosystem, enabling precise numerical tracking, sequential numbering, and state management with robust security and flexible configuration options.