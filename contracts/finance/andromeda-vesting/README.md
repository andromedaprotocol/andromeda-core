# Andromeda Vesting ADO

## Introduction

The Andromeda Vesting ADO is a sophisticated token distribution contract that enables gradual release of tokens over configurable time periods. This contract implements batch-based vesting with flexible lockup periods, release schedules, and claiming mechanisms. The vesting system supports multiple independent batches, allowing for complex token distribution strategies with cliff periods, linear vesting, and customizable release amounts for employee compensation, investor distributions, and project funding scenarios.

<b>Ado_type:</b> vesting

## Why Vesting ADO

The Vesting ADO serves as critical infrastructure for applications requiring:

- **Employee Compensation**: Gradual release of tokens for employee stock option plans and equity compensation
- **Investor Distributions**: Controlled token release for seed, private, and public sale participants
- **Team Allocations**: Vesting schedules for founders, advisors, and key contributors
- **Project Funding**: Milestone-based funding release for development teams and partners
- **Liquidity Management**: Controlled market entry to prevent sudden token dumps
- **Incentive Alignment**: Long-term alignment through gradual token release
- **Compliance Requirements**: Meeting regulatory requirements for token distribution
- **Treasury Management**: Strategic token release from protocol treasuries
- **Ecosystem Development**: Phased token allocation for ecosystem growth and development
- **Performance-Based Rewards**: Merit-based token distribution with vesting requirements

The ADO provides flexible batch management, precise time-based controls, and comprehensive claiming mechanisms for reliable token vesting operations.

## Key Features

### **Batch-Based Vesting**
- **Multiple batches**: Support for unlimited independent vesting batches
- **Flexible timing**: Each batch can have unique lockup and release schedules
- **Batch isolation**: Independent tracking and claiming for each batch
- **Batch management**: Create, track, and claim from individual or all batches
- **Scalable architecture**: Efficient handling of numerous concurrent vesting schedules

### **Flexible Release Mechanisms**
- **Lockup periods**: Optional cliff periods before vesting begins
- **Release intervals**: Configurable time periods between releases
- **Release amounts**: Support for fixed amounts or percentage-based releases
- **Continuous vesting**: Automated calculation of claimable amounts over time
- **Precision timing**: Millisecond-level precision for accurate vesting calculations

### **Advanced Claiming**
- **Individual claims**: Claim specific amounts from individual batches
- **Batch claims**: Claim all available tokens from all batches
- **Partial claims**: Claim specific number of release periods
- **Time-bound claims**: Claim up to specific timestamps for precise control
- **Bulk operations**: Efficient claiming across multiple batches

### **Security and Validation**
- **Recipient protection**: Only designated recipients can claim tokens
- **Amount validation**: Comprehensive validation of batch creation and claiming
- **Time enforcement**: Strict enforcement of lockup and release schedules
- **Balance protection**: Prevent claims exceeding available balances
- **State consistency**: Maintain accurate state across all operations

## Release Types

### **Fixed Amount Releases**
Release a fixed token amount at each interval:
```rust
WithdrawalType::Amount(Uint128::new(1000000))
```

### **Percentage-Based Releases**
Release a percentage of the original batch amount:
```rust
WithdrawalType::Percentage(Decimal::percent(5))
```

## InstantiateMsg

```rust
pub struct InstantiateMsg {
    pub recipient: Recipient,
    pub denom: String,
}
```

```json
{
    "recipient": {
        "address": "andr1employee_address...",
        "msg": null
    },
    "denom": "uandr"
}
```

**Parameters**:
- **recipient**: The address that will receive vested tokens
  - Can include optional message for contract recipients
  - Must be a valid Andromeda address
- **denom**: Native token denomination for all vesting batches
  - Must be a valid native denomination
  - All batches in the contract use the same denomination

**Validation**:
- Recipient address must be valid and accessible
- Denomination must be a valid native token denomination
- Contract validates denominations against the current chain

## ExecuteMsg

### CreateBatch
Creates a new vesting batch with specified parameters.

```rust
CreateBatch {
    lockup_duration: Option<Milliseconds>,
    release_duration: Milliseconds,
    release_amount: WithdrawalType,
}
```

```json
{
    "create_batch": {
        "lockup_duration": 2592000000,
        "release_duration": 604800000,
        "release_amount": {
            "percentage": "0.05"
        }
    }
}
```

**Parameters**:
- **lockup_duration**: Optional cliff period in milliseconds before vesting begins
  - `null` means vesting starts immediately
  - Prevents any claims until lockup expires
- **release_duration**: Time interval between releases in milliseconds
- **release_amount**: Amount released at each interval
  - **Amount**: Fixed token amount per release
  - **Percentage**: Percentage of original batch amount per release

**Usage**: Send tokens as funds with this message. These tokens become the vesting batch.

**Example with Fixed Amount:**
```json
{
    "create_batch": {
        "lockup_duration": null,
        "release_duration": 86400000,
        "release_amount": {
            "amount": "1000000000"
        }
    }
}
```

**Requirements**:
- Must send exactly one native token as funds
- Token denomination must match contract's configured denomination
- Release duration and amount must be non-zero
- Sender must have restricted permissions (typically contract owner)

### Claim
Claims available tokens from a specific vesting batch.

```rust
Claim {
    number_of_claims: Option<u64>,
    batch_id: u64,
}
```

```json
{
    "claim": {
        "number_of_claims": 3,
        "batch_id": 1
    }
}
```

**Parameters**:
- **number_of_claims**: Number of release periods to claim (optional)
  - If not specified, claims all available periods
  - Cannot exceed available claimable periods
- **batch_id**: ID of the batch to claim from

**Authorization**: Only the designated recipient can claim tokens
**Validation**: Claims cannot exceed lockup periods or available amounts

### ClaimAll
Claims available tokens from all vesting batches.

```rust
ClaimAll {
    up_to_time: Option<Milliseconds>,
    limit: Option<u32>,
}
```

```json
{
    "claim_all": {
        "up_to_time": 1672617600000,
        "limit": 50
    }
}
```

**Parameters**:
- **up_to_time**: Maximum timestamp to claim up to (optional)
  - Defaults to current time if not specified
  - Useful for claiming historical periods without latest releases
- **limit**: Maximum number of batches to process (optional)
  - Defaults to reasonable limit for gas efficiency
  - Enables pagination for large numbers of batches

**Authorization**: Only the designated recipient can claim tokens
**Efficiency**: Processes multiple batches in a single transaction

## QueryMsg

### Config
Returns the vesting contract configuration.

```rust
#[returns(Config)]
Config {}

pub struct Config {
    pub recipient: Recipient,
    pub denom: String,
}
```

```json
{
    "config": {}
}
```

**Response:**
```json
{
    "recipient": {
        "address": "andr1employee_address...",
        "msg": null
    },
    "denom": "uandr"
}
```

### Batch
Returns detailed information about a specific vesting batch.

```rust
#[returns(BatchResponse)]
Batch { id: u64 }

pub struct BatchResponse {
    pub id: u64,
    pub amount: Uint128,
    pub amount_claimed: Uint128,
    pub amount_available_to_claim: Uint128,
    pub number_of_available_claims: Uint128,
    pub lockup_end: Milliseconds,
    pub release_duration: Milliseconds,
    pub release_amount: WithdrawalType,
    pub last_claimed_release_time: Milliseconds,
}
```

```json
{
    "batch": {
        "id": 1
    }
}
```

**Response:**
```json
{
    "id": 1,
    "amount": "10000000000",
    "amount_claimed": "2000000000",
    "amount_available_to_claim": "500000000",
    "number_of_available_claims": "1",
    "lockup_end": "1640995200000",
    "release_duration": "604800000",
    "release_amount": {
        "percentage": "0.05"
    },
    "last_claimed_release_time": "1641600000000"
}
```

**Response Fields**:
- **id**: Unique batch identifier
- **amount**: Total tokens in the batch
- **amount_claimed**: Tokens already claimed
- **amount_available_to_claim**: Tokens available for immediate claiming
- **number_of_available_claims**: Number of release periods available to claim
- **lockup_end**: Timestamp when lockup period ends
- **release_duration**: Milliseconds between each release
- **release_amount**: Amount or percentage released per period
- **last_claimed_release_time**: Timestamp of last claim

### Batches
Returns information about all vesting batches with pagination.

```rust
#[returns(Vec<BatchResponse>)]
Batches {
    start_after: Option<u64>,
    limit: Option<u32>,
}
```

```json
{
    "batches": {
        "start_after": null,
        "limit": 10
    }
}
```

**Response:**
```json
[
    {
        "id": 1,
        "amount": "10000000000",
        "amount_claimed": "2000000000",
        "amount_available_to_claim": "500000000",
        "number_of_available_claims": "1",
        "lockup_end": "1640995200000",
        "release_duration": "604800000",
        "release_amount": {
            "percentage": "0.05"
        },
        "last_claimed_release_time": "1641600000000"
    },
    {
        "id": 2,
        "amount": "5000000000",
        "amount_claimed": "0",
        "amount_available_to_claim": "0",
        "number_of_available_claims": "0",
        "lockup_end": "1672531200000",
        "release_duration": "2629746000",
        "release_amount": {
            "amount": "500000000"
        },
        "last_claimed_release_time": "1672531200000"
    }
]
```

## Usage Examples

### Employee Stock Option Plan
```json
{
    "recipient": {
        "address": "andr1employee...",
        "msg": null
    },
    "denom": "uandr"
}
```

**Create 4-Year Vesting with 1-Year Cliff:**
```json
{
    "create_batch": {
        "lockup_duration": 31557600000,
        "release_duration": 2629746000,
        "release_amount": {
            "percentage": "0.0277777778"
        }
    }
}
```
_Send 1,000,000 tokens as funds. 1-year cliff, then ~2.78% monthly for 36 months._

### Investor Token Distribution
```json
{
    "create_batch": {
        "lockup_duration": 15778800000,
        "release_duration": 604800000,
        "release_amount": {
            "percentage": "0.02"
        }
    }
}
```
_6-month cliff, then 2% weekly releases for 50 weeks._

### Advisor Compensation
```json
{
    "create_batch": {
        "lockup_duration": null,
        "release_duration": 2629746000,
        "release_amount": {
            "amount": "10000000000"
        }
    }
}
```
_No cliff, 10,000 tokens released monthly._

### Performance Milestone Rewards
```json
{
    "create_batch": {
        "lockup_duration": 7776000000,
        "release_duration": 7776000000,
        "release_amount": {
            "percentage": "0.25"
        }
    }
}
```
_3-month cliff, then 25% quarterly releases._

## Claiming Examples

### Claim from Specific Batch
```json
{
    "claim": {
        "number_of_claims": 2,
        "batch_id": 1
    }
}
```

### Claim All Available
```json
{
    "claim_all": {
        "up_to_time": null,
        "limit": null
    }
}
```

### Historical Claim
```json
{
    "claim_all": {
        "up_to_time": 1672617600000,
        "limit": 100
    }
}
```

### Query Examples

### Check Batch Status
```json
{
    "batch": {
        "id": 1
    }
}
```

### List All Batches
```json
{
    "batches": {
        "start_after": null,
        "limit": 20
    }
}
```

### Check Configuration
```json
{
    "config": {}
}
```

## Integration Patterns

### With App Contract
Vesting can be integrated for employee and investor management:

```json
{
    "components": [
        {
            "name": "employee_vesting",
            "ado_type": "vesting",
            "component_type": {
                "new": {
                    "recipient": {
                        "address": "andr1employee...",
                        "msg": null
                    },
                    "denom": "uandr"
                }
            }
        },
        {
            "name": "investor_vesting",
            "ado_type": "vesting",
            "component_type": {
                "new": {
                    "recipient": {
                        "address": "andr1investor...",
                        "msg": null
                    },
                    "denom": "uandr"
                }
            }
        }
    ]
}
```

### Employee Management
For managing employee token compensation:

1. **Deploy vesting contracts** for each employee with appropriate recipients
2. **Create vesting batches** with standard company vesting schedules
3. **Handle cliff periods** for employment security and retention
4. **Enable self-service claiming** for employees to access vested tokens
5. **Track vesting progress** through batch queries

### Investor Relations
For managing investor token distributions:

1. **Set up investor-specific contracts** with appropriate vesting terms
2. **Create batches** matching investment agreement schedules
3. **Handle different investor tiers** with varying vesting parameters
4. **Automate distribution** through batch claiming mechanisms
5. **Maintain compliance** through transparent vesting records

### Protocol Governance
For managing protocol token distributions:

1. **Establish treasury vesting** for protocol development funding
2. **Create milestone-based releases** tied to development goals
3. **Enable community oversight** through transparent batch tracking
4. **Implement governance controls** over vesting schedule creation
5. **Support ecosystem growth** through strategic token release

## Advanced Features

### **Batch Management**
- **Independent tracking**: Each batch operates independently with unique parameters
- **Flexible scheduling**: Different lockup and release schedules per batch
- **Precise calculations**: Millisecond-level precision for accurate vesting
- **State persistence**: Robust state management across all batch operations

### **Time-Based Controls**
- **Lockup enforcement**: Strict enforcement of cliff periods before vesting
- **Release intervals**: Precise timing for periodic token releases
- **Historical claims**: Support for claiming past periods up to specific times
- **Future scheduling**: Pre-configured release schedules extending into the future

### **Claiming Mechanisms**
- **Granular control**: Claim specific numbers of release periods
- **Bulk operations**: Efficient claiming across multiple batches
- **Partial claims**: Claim portions of available amounts
- **Automated calculations**: Real-time calculation of claimable amounts

### **Validation and Security**
- **Balance protection**: Prevent claims exceeding available balances
- **Recipient verification**: Ensure only authorized recipients can claim
- **State consistency**: Maintain accurate state across all operations
- **Error handling**: Graceful handling of edge cases and errors

## Security Features

### **Access Control**
- **Recipient restrictions**: Only designated recipients can claim tokens
- **Batch creation permissions**: Controlled through standard Andromeda permissions
- **Ownership validation**: Verify ownership before allowing sensitive operations
- **Address validation**: Comprehensive validation of all addresses

### **Fund Protection**
- **Escrow security**: Tokens held securely in contract until claimed
- **Balance verification**: Continuous verification of contract balances
- **Atomic operations**: All operations are atomic to prevent partial failures
- **Overflow protection**: Safe math operations to prevent overflow attacks

### **Timing Security**
- **Lockup enforcement**: Immutable lockup periods prevent early claims
- **Release validation**: Accurate validation of release timing and amounts
- **Time manipulation protection**: Protection against timestamp manipulation
- **Precision timing**: Millisecond precision prevents timing exploitation

### **State Integrity**
- **Consistent state**: Maintain consistent state across all batch operations
- **Validation checks**: Comprehensive validation of all state transitions
- **Error recovery**: Graceful handling of failed operations
- **Data persistence**: Reliable persistence of all vesting data

## Important Notes

- **Single denomination**: All batches in a contract use the same token denomination
- **Immutable schedules**: Once created, batch parameters cannot be modified
- **Recipient restriction**: Only the designated recipient can claim from batches
- **Precise timing**: Vesting calculations use millisecond precision
- **Balance dependency**: Claims are limited by actual contract token balance
- **Batch independence**: Each batch operates independently with its own schedule
- **No early termination**: Batches cannot be terminated or canceled once created
- **Cumulative claiming**: Claims are cumulative and track total claimed amounts

## Common Workflow

### 1. **Deploy Vesting Contract**
```json
{
    "recipient": {
        "address": "andr1employee...",
        "msg": null
    },
    "denom": "uandr"
}
```

### 2. **Create Vesting Batch**
```json
{
    "create_batch": {
        "lockup_duration": 31557600000,
        "release_duration": 2629746000,
        "release_amount": {
            "percentage": "0.0833333333"
        }
    }
}
```
_Send tokens as funds with the transaction._

### 3. **Monitor Vesting Progress**
```json
{
    "batch": {
        "id": 1
    }
}
```

### 4. **Claim Vested Tokens**
```json
{
    "claim": {
        "number_of_claims": null,
        "batch_id": 1
    }
}
```

### 5. **Check All Batches**
```json
{
    "batches": {
        "start_after": null,
        "limit": 10
    }
}
```

### 6. **Claim All Available**
```json
{
    "claim_all": {
        "up_to_time": null,
        "limit": null
    }
}
```

The Vesting ADO provides comprehensive infrastructure for token distribution management, enabling secure, flexible, and transparent vesting mechanisms for all types of token allocation scenarios in the Andromeda ecosystem.