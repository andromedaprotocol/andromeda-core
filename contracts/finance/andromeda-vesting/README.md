# Andromeda Vesting ADO

## Introduction

The Andromeda Vesting ADO is a sophisticated token vesting contract that manages time-based token releases to a designated recipient. It supports multiple vesting batches with customizable lockup periods, release schedules, and withdrawal amounts, making it ideal for employee compensation, investor token allocations, advisor rewards, and any scenario requiring controlled token distribution over time.

<b>Ado_type:</b> vesting

## Why Vesting ADO

The Vesting ADO serves as a critical financial tool for applications requiring:

- **Employee Compensation**: Implement token-based compensation with vesting schedules
- **Investor Token Allocations**: Distribute investor tokens over predetermined timeframes
- **Advisor Rewards**: Vest advisor tokens based on engagement duration
- **Founder Token Lockup**: Implement founder token vesting for project credibility
- **Team Incentivization**: Create performance-based token release schedules
- **Token Sale Distributions**: Vest tokens from private sales or ICO/IDO events
- **Partnership Agreements**: Implement milestone-based token releases
- **DAO Compensation**: Distribute DAO contributor rewards over time
- **Liquidity Incentives**: Vest tokens for liquidity providers and protocol participants
- **Long-term Alignment**: Ensure long-term commitment through extended vesting periods

The ADO supports multiple concurrent vesting batches, flexible release mechanisms (fixed amounts or percentages), and efficient claiming with batch processing capabilities.

## Key Features

### **Multi-Batch Vesting**
- **Multiple schedules**: Create numerous vesting batches with different parameters
- **Independent batches**: Each batch has its own lockup, release schedule, and amounts
- **Batch management**: Efficient storage and retrieval with pagination support
- **Concurrent vesting**: Multiple batches can vest simultaneously

### **Flexible Vesting Parameters**
- **Lockup periods**: Optional lockup before vesting begins
- **Release schedules**: Configurable release frequency (daily, weekly, monthly, etc.)
- **Release amounts**: Fixed token amounts or percentages per release
- **Single recipient**: All batches vest to the same designated recipient

### **Time-Based Releases**
- **Automatic calculation**: System calculates available claims based on elapsed time
- **Precise timing**: Millisecond precision for release calculations
- **Claim optimization**: Only release tokens when lockup periods expire
- **Overflow protection**: Safe arithmetic prevents calculation errors

### **Efficient Claiming**
- **Individual claims**: Claim specific amounts from individual batches
- **Batch claims**: Claim all available tokens across multiple batches
- **Pagination support**: Handle large numbers of batches efficiently
- **Gas optimization**: Minimize transaction costs for large claims

## Vesting Mechanics

### **Vesting Formula**
Available tokens are calculated based on:
1. **Lockup End Check**: Current time ≥ batch lockup end time
2. **Elapsed Time**: Time since last claim or lockup end
3. **Release Periods**: Number of complete release periods elapsed
4. **Release Amount**: Tokens per release period (fixed amount or percentage)

### **Example Calculation**
- **Batch Amount**: 1,000,000 tokens
- **Release Duration**: 2,592,000,000 ms (30 days)
- **Release Amount**: 10% of original amount
- **Lockup End**: January 1, 2024
- **Current Time**: March 1, 2024 (60 days after lockup)

**Calculation**:
- Elapsed periods: 60 days ÷ 30 days = 2 complete periods
- Tokens per release: 10% × 1,000,000 = 100,000 tokens
- Available to claim: 2 × 100,000 = 200,000 tokens

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
        "address": "andr1employee123...",
        "msg": null
    },
    "denom": "uandr"
}
```

- **recipient**: The address that will receive all vested tokens from all batches
- **denom**: The denomination of tokens that can be vested (all batches must use this denom)

_**Note:** The recipient and denom are fixed for the entire contract lifetime and cannot be changed._

## ExecuteMsg

### CreateBatch
Creates a new vesting batch with specified parameters.

_**Note:** Only contract owner can execute. Must send exactly one coin matching the configured denom._

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
        "lockup_duration": "2592000000",
        "release_duration": "2592000000",
        "release_amount": {
            "percentage": "0.1"
        }
    }
}
```

**Parameters**:
- **lockup_duration**: Optional lockup period in milliseconds before vesting starts
  - `null` means vesting starts immediately
  - Must be a positive duration if specified
- **release_duration**: Time between releases in milliseconds
  - Cannot be zero
  - Examples: 86400000 (1 day), 2592000000 (30 days), 31536000000 (1 year)
- **release_amount**: Amount released per period
  - **Fixed Amount**: `{"amount": "100000"}` (specific token amount)
  - **Percentage**: `{"percentage": "0.1"}` (10% of original batch amount)

### Claim
Claims available tokens from a specific vesting batch.

_**Note:** Only contract owner can execute._

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
- **number_of_claims**: Optional limit on number of release periods to claim
  - `null` claims all available periods
  - Cannot exceed available claims
- **batch_id**: ID of the specific batch to claim from

### ClaimAll
Claims available tokens from all claimable batches using pagination.

_**Note:** Only contract owner can execute._

```rust
ClaimAll {
    up_to_time: Option<Milliseconds>,
    limit: Option<u32>,
}
```

```json
{
    "claim_all": {
        "up_to_time": "1641081600000",
        "limit": 10
    }
}
```

**Parameters**:
- **up_to_time**: Optional time limit for claims in milliseconds
  - `null` claims up to current time
  - Useful for partial claims or historical claiming
- **limit**: Optional limit on number of batches to process (max 30)
  - `null` uses default limit of 10
  - Helps manage gas costs for large numbers of batches

## QueryMsg

### Config
Returns the vesting contract configuration.

```rust
pub enum QueryMsg {
    #[returns(Config)]
    Config {},
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
        "address": "andr1employee123...",
        "msg": null
    },
    "denom": "uandr"
}
```

### Batch
Returns detailed information about a specific vesting batch.

```rust
pub enum QueryMsg {
    #[returns(BatchResponse)]
    Batch { id: u64 },
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
    "amount": "1000000",
    "amount_claimed": "200000",
    "amount_available_to_claim": "100000",
    "number_of_available_claims": "1",
    "lockup_end": "1640995200000",
    "release_duration": "2592000000",
    "release_amount": {
        "percentage": "0.1"
    },
    "last_claimed_release_time": "1643587200000"
}
```

### Batches
Returns information about multiple batches with pagination.

```rust
pub enum QueryMsg {
    #[returns(Vec<BatchResponse>)]
    Batches {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
}
```

```json
{
    "batches": {
        "start_after": 5,
        "limit": 10
    }
}
```

**Response:**
```json
[
    {
        "id": 6,
        "amount": "500000",
        "amount_claimed": "100000",
        "amount_available_to_claim": "50000",
        "number_of_available_claims": "1",
        "lockup_end": "1640995200000",
        "release_duration": "2592000000",
        "release_amount": {
            "amount": "50000"
        },
        "last_claimed_release_time": "1643587200000"
    }
]
```

## Usage Examples

### Employee 4-Year Vesting with 1-Year Cliff
```json
{
    "create_batch": {
        "lockup_duration": "31536000000",
        "release_duration": "2592000000",
        "release_amount": {
            "percentage": "0.0208333"
        }
    }
}
```
**Setup**: 1-year lockup, then 2.08333% per month for 48 months (4 years total)

### Investor Quarterly Vesting
```json
{
    "create_batch": {
        "lockup_duration": null,
        "release_duration": "7776000000",
        "release_amount": {
            "percentage": "0.25"
        }
    }
}
```
**Setup**: No lockup, 25% every quarter (3 months) for 1 year

### Advisor Monthly Fixed Amount
```json
{
    "create_batch": {
        "lockup_duration": "7776000000",
        "release_duration": "2592000000",
        "release_amount": {
            "amount": "10000"
        }
    }
}
```
**Setup**: 3-month lockup, then 10,000 tokens per month

### Team Lead Progressive Vesting
```json
{
    "create_batch": {
        "lockup_duration": "15552000000",
        "release_duration": "2592000000",
        "release_amount": {
            "percentage": "0.05"
        }
    }
}
```
**Setup**: 6-month lockup, then 5% per month for 20 months

## Integration Patterns

### With App Contract
The Vesting ADO can be integrated into App contracts for employee management:

```json
{
    "components": [
        {
            "name": "employee_vesting",
            "ado_type": "vesting",
            "component_type": {
                "new": {
                    "recipient": {
                        "address": "andr1employee..."
                    },
                    "denom": "ucompany"
                }
            }
        }
    ]
}
```

### Multi-Employee Setup
For organizations with multiple employees:

1. **Deploy separate contracts** for each employee
2. **Configure different schedules** based on role and seniority
3. **Manage vesting collectively** through administrative tools
4. **Track progress** across all employee vesting schedules

### Investor Relations
For managing investor token distributions:

1. **Create investor-specific contracts** with appropriate schedules
2. **Implement milestone-based releases** using multiple batches
3. **Provide transparency** through public query endpoints
4. **Automate distributions** through scheduled claiming

### DAO Compensation
For decentralized organization compensation:

1. **Vest contributor rewards** based on participation
2. **Implement role-based schedules** for different contribution types
3. **Enable self-claiming** by contributors when vesting periods complete
4. **Maintain governance alignment** through extended vesting periods

## Vesting Schedule Examples

### Standard Startup Employee (4-year with 1-year cliff)
- **Total**: 48,000 tokens
- **Cliff**: 12 months (12,000 tokens)
- **Monthly**: 1,000 tokens for 36 months after cliff

### Investor Series A (2-year quarterly)
- **Total**: 1,000,000 tokens
- **Quarterly**: 125,000 tokens every 3 months
- **Duration**: 8 quarters (2 years)

### Advisor Compensation (2-year monthly)
- **Total**: 24,000 tokens
- **Monthly**: 1,000 tokens
- **Duration**: 24 months

### Founder Vesting (5-year with 2-year cliff)
- **Total**: 10,000,000 tokens
- **Cliff**: 24 months (4,000,000 tokens)
- **Monthly**: 166,667 tokens for 36 months after cliff

## Claiming Strategies

### Individual Batch Claims
```json
{
    "claim": {
        "number_of_claims": null,
        "batch_id": 1
    }
}
```
**Use case**: Claim all available tokens from a specific batch

### Partial Claims
```json
{
    "claim": {
        "number_of_claims": 2,
        "batch_id": 1
    }
}
```
**Use case**: Claim only 2 release periods, leaving rest for later

### Bulk Claims
```json
{
    "claim_all": {
        "up_to_time": null,
        "limit": 20
    }
}
```
**Use case**: Claim from up to 20 batches at current time

### Historical Claims
```json
{
    "claim_all": {
        "up_to_time": "1641081600000",
        "limit": 10
    }
}
```
**Use case**: Claim up to a specific historical point in time

## Security Features

### **Owner-Only Operations**
- **Batch creation**: Only owner can create new vesting batches
- **Token claiming**: Only owner can trigger token claims
- **Configuration immutable**: Recipient and denom cannot be changed

### **Validation Safeguards**
- **Denom validation**: All batches must use the configured denomination
- **Amount validation**: Prevents zero amounts and validates sufficient funds
- **Time validation**: Ensures lockup periods and release durations are valid
- **Overflow protection**: Safe arithmetic prevents calculation errors

### **Fund Security**
- **Contract balance checks**: Validates sufficient contract balance for claims
- **Atomic operations**: Claims either succeed completely or fail entirely
- **Precise calculations**: Prevents rounding errors in token distributions

## Important Notes

- **Single recipient**: All batches vest to the same recipient address
- **Single denomination**: All batches must use the same token type
- **Immutable config**: Recipient and denom cannot be changed after instantiation
- **Owner control**: Only contract owner can create batches and trigger claims
- **Automatic calculations**: System calculates available claims based on time elapsed
- **Efficient claiming**: ClaimAll processes multiple batches in single transaction
- **Batch independence**: Each batch vests independently with its own schedule
- **Precise timing**: Uses millisecond precision for accurate vesting calculations

## Gas Optimization

### **Batch Processing**
- **Pagination limits**: Maximum 30 batches per ClaimAll transaction
- **Efficient indexing**: Smart indexing for fast claimable batch lookup
- **Selective claiming**: Only process batches with available claims

### **Storage Efficiency**
- **Compact batch storage**: Efficient storage of batch parameters
- **Index optimization**: Fast lookup for claimable batches
- **Minimal state updates**: Only update necessary fields during claims

## Error Handling

### Common Errors
- **FundsAreLocked**: Attempting to claim before lockup expires
- **WithdrawalIsEmpty**: No tokens available to claim
- **InvalidFunds**: Incorrect denomination or zero amounts
- **InvalidZeroAmount**: Zero release amounts or durations
- **Overflow**: Mathematical overflow in calculations

The Vesting ADO provides a comprehensive, secure, and flexible solution for token vesting in blockchain applications, offering the precision and control needed for professional token distribution while maintaining gas efficiency and ease of use.