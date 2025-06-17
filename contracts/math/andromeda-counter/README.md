# Andromeda Counter ADO

## Introduction

The Andromeda Counter ADO is a simple but powerful mathematical utility contract that provides configurable counter functionality. It can be used for tracking sequential values, implementing counters with custom increment/decrement amounts, and managing state in applications that require numerical progression tracking.

<b>Ado_type:</b> counter

## Why Counter ADO

The Counter ADO serves as a fundamental building block for applications requiring:

- **Sequential ID Generation**: Generate unique sequential identifiers for NFTs, orders, or other entities
- **Voting Systems**: Track vote counts with configurable increment amounts
- **Gaming Applications**: Manage scores, levels, or progression counters
- **Access Control**: Implement usage counters for rate limiting or subscription tracking
- **Statistical Tracking**: Count events, transactions, or user interactions
- **Inventory Management**: Track quantities with custom increment/decrement logic

The ADO supports both public and private access modes, making it suitable for both open community counters and restricted internal counting systems.

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
    "restriction": "Public",
    "initial_state": {
        "initial_amount": 0,
        "increase_amount": 1,
        "decrease_amount": 1
    }
}
```

- **restriction**: Controls who can modify the counter
  - `"Private"`: Only contract owner/operators can increment/decrement
  - `"Public"`: Anyone can increment/decrement the counter
- **initial_state**: Configuration for counter behavior
  - **initial_amount**: Starting value (defaults to 0 if not provided)
  - **increase_amount**: Amount to add on increment (defaults to 1)
  - **decrease_amount**: Amount to subtract on decrement (defaults to 1)

## ExecuteMsg

### Increment
Increases the current counter value by the configured increase amount.

_**Note:** Requires permission based on restriction setting. Private counters restrict access to owner/operators._

```rust
Increment {}
```

```json
{
    "increment": {}
}
```

### Decrement
Decreases the current counter value by the configured decrease amount. Uses saturating subtraction to prevent underflow.

_**Note:** Requires permission based on restriction setting. Uses saturating subtraction, so counter will not go below 0._

```rust
Decrement {}
```

```json
{
    "decrement": {}
}
```

### Reset
Resets the counter to its initial configured amount.

_**Note:** Requires permission based on restriction setting._

```rust
Reset {}
```

```json
{
    "reset": {}
}
```

### UpdateRestriction
Updates the access restriction for the counter operations.

_**Note:** Only contract owner can execute this operation._

```rust
UpdateRestriction { 
    restriction: CounterRestriction 
}
```

```json
{
    "update_restriction": {
        "restriction": "Private"
    }
}
```

### SetIncreaseAmount
Updates the amount by which the counter increments.

_**Note:** Only contract owner can execute this operation._

```rust
SetIncreaseAmount { 
    increase_amount: u64 
}
```

```json
{
    "set_increase_amount": {
        "increase_amount": 5
    }
}
```

### SetDecreaseAmount
Updates the amount by which the counter decrements.

_**Note:** Only contract owner can execute this operation._

```rust
SetDecreaseAmount { 
    decrease_amount: u64 
}
```

```json
{
    "set_decrease_amount": {
        "decrease_amount": 2
    }
}
```

## QueryMsg

### GetInitialAmount
Returns the initial amount that was set during instantiation.

```rust
pub enum QueryMsg {
    #[returns(GetInitialAmountResponse)]
    GetInitialAmount {},
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
    "initial_amount": 0
}
```

### GetCurrentAmount
Returns the current counter value.

```rust
pub enum QueryMsg {
    #[returns(GetCurrentAmountResponse)]
    GetCurrentAmount {},
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
    "current_amount": 42
}
```

### GetIncreaseAmount
Returns the configured increment amount.

```rust
pub enum QueryMsg {
    #[returns(GetIncreaseAmountResponse)]
    GetIncreaseAmount {},
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
    "increase_amount": 1
}
```

### GetDecreaseAmount
Returns the configured decrement amount.

```rust
pub enum QueryMsg {
    #[returns(GetDecreaseAmountResponse)]
    GetDecreaseAmount {},
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
    "decrease_amount": 1
}
```

### GetRestriction
Returns the current access restriction setting.

```rust
pub enum QueryMsg {
    #[returns(GetRestrictionResponse)]
    GetRestriction {},
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
    "restriction": "Public"
}
```

## Usage Examples

### Basic Public Counter
```json
{
    "restriction": "Public",
    "initial_state": {
        "initial_amount": 0,
        "increase_amount": 1,
        "decrease_amount": 1
    }
}
```

### Custom Vote Counter
```json
{
    "restriction": "Private",
    "initial_state": {
        "initial_amount": 0,
        "increase_amount": 1,
        "decrease_amount": 1
    }
}
```

### Score Tracker with Custom Increments
```json
{
    "restriction": "Public",
    "initial_state": {
        "initial_amount": 100,
        "increase_amount": 10,
        "decrease_amount": 5
    }
}
```

## Integration Patterns

### With App Contract
The Counter ADO can be included as a component in App contracts to provide counting functionality to complex applications:

```json
{
    "components": [
        {
            "name": "user_counter",
            "ado_type": "counter",
            "component_type": {
                "new": {
                    "restriction": "Public",
                    "initial_state": {
                        "initial_amount": 0,
                        "increase_amount": 1,
                        "decrease_amount": 1
                    }
                }
            }
        }
    ]
}
```

### Cross-Chain Usage
Counter ADOs can be deployed across multiple chains and synchronized through AMP messaging, enabling distributed counting systems.