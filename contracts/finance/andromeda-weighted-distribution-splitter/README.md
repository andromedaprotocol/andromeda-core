# Andromeda Weighted Distribution Splitter ADO

## Introduction

The Andromeda Weighted Distribution Splitter ADO is a sophisticated fund distribution contract that distributes tokens based on weighted proportions rather than fixed amounts. Each recipient is assigned a weight, and they receive a percentage of the total funds proportional to their weight relative to the total weight of all recipients, making it ideal for revenue sharing, profit distribution, stakeholder payments, and any scenario requiring proportional fund allocation.

<b>Ado_type:</b> weighted-distribution-splitter

## Why Weighted Distribution Splitter ADO

The Weighted Distribution Splitter ADO serves as a powerful financial tool for applications requiring:

- **Revenue Sharing**: Distribute revenue proportionally among stakeholders based on their stake
- **Profit Distribution**: Share profits according to ownership percentages or contribution weights
- **Dividend Payments**: Distribute dividends based on shareholding weights
- **Pool Rewards**: Distribute rewards proportionally to pool participants
- **Commission Distribution**: Share commissions among team members based on performance weights
- **Royalty Payments**: Distribute royalties to multiple creators based on contribution weights
- **Partnership Distributions**: Allocate funds among partners according to partnership percentages
- **Token Distribution**: Distribute tokens proportionally in airdrops or allocations
- **Validator Rewards**: Distribute staking rewards based on delegation weights
- **DAO Distributions**: Allocate DAO funds based on governance token weights

The ADO automatically calculates proportional distributions, handles remainder funds, supports multiple token types, and provides flexible weight management with security controls.

## Key Features

### **Weight-Based Distribution**
- **Proportional allocation**: Each recipient receives `(recipient_weight / total_weight) * total_funds`
- **Dynamic percentages**: Percentages automatically adjust when weights change
- **Flexible weights**: Weights can be any positive integer value
- **Real-time calculation**: Distributions calculated automatically at execution time

### **Flexible Weight Management**
- **Individual updates**: Update specific recipient weights without affecting others
- **Add/remove recipients**: Dynamically manage the recipient list
- **Bulk updates**: Replace entire recipient list when needed
- **Weight validation**: Ensures all weights are positive, non-zero values

### **Multi-Token Support**
- **Multiple coin types**: Support up to 5 different token types per transaction
- **Proportional distribution**: Each token type distributed proportionally
- **Native and CW20**: Compatible with Cosmos native tokens and CW20 standards
- **Simultaneous processing**: All token types processed in single transaction

### **Advanced Security**
- **Lock mechanism**: Time-based configuration locking for security
- **Owner controls**: Only owner can modify configuration when unlocked
- **Duplicate prevention**: Prevents duplicate recipients
- **Weight validation**: Enforces positive weight requirements

## Mathematical Formula

The distribution calculation follows this formula:

**Recipient Amount = (Recipient Weight / Total Weight) × Total Sent Amount**

### Example Calculation
- **Total sent**: 1,000,000 tokens
- **Recipient A weight**: 30
- **Recipient B weight**: 50  
- **Recipient C weight**: 20
- **Total weight**: 100

**Distribution**:
- Recipient A: (30/100) × 1,000,000 = 300,000 tokens
- Recipient B: (50/100) × 1,000,000 = 500,000 tokens
- Recipient C: (20/100) × 1,000,000 = 200,000 tokens

## InstantiateMsg

```rust
pub struct InstantiateMsg {
    pub recipients: Vec<AddressWeight>,
    pub lock_time: Option<Expiry>,
    pub default_recipient: Option<Recipient>,
}

pub struct AddressWeight {
    pub recipient: Recipient,
    pub weight: Uint128,
}
```

```json
{
    "recipients": [
        {
            "recipient": {
                "address": "andr1partner1...",
                "msg": null
            },
            "weight": "50"
        },
        {
            "recipient": {
                "address": "andr1partner2...",
                "msg": null
            },
            "weight": "30"
        },
        {
            "recipient": {
                "address": "andr1partner3...",
                "msg": null
            },
            "weight": "20"
        }
    ],
    "lock_time": {
        "at_time": "1640995200000000000"
    },
    "default_recipient": {
        "address": "andr1treasury...",
        "msg": null
    }
}
```

- **recipients**: List of recipients with their weights
  - **recipient**: Address and optional message for the recipient
  - **weight**: Positive integer representing the recipient's allocation weight
- **lock_time**: Optional lock expiration time for configuration security
- **default_recipient**: Optional recipient for remainder funds (defaults to sender)

## ExecuteMsg

### Send
Distributes the attached funds proportionally based on recipient weights.

```rust
Send { 
    config: Option<Vec<AddressWeight>> 
}
```

```json
{
    "send": {
        "config": null
    }
}
```

**With Custom Configuration (when unlocked):**
```json
{
    "send": {
        "config": [
            {
                "recipient": {
                    "address": "andr1temp_recipient...",
                    "msg": null
                },
                "weight": "75"
            },
            {
                "recipient": {
                    "address": "andr1another_recipient...",
                    "msg": null
                },
                "weight": "25"
            }
        ]
    }
}
```

_**Note:** Can handle up to 5 different coin types per transaction. If config is provided, contract must be unlocked._

### UpdateRecipients
Replaces the entire recipient list with new recipients and weights.

_**Note:** Only contract owner can execute when unlocked._

```rust
UpdateRecipients { 
    recipients: Vec<AddressWeight> 
}
```

```json
{
    "update_recipients": {
        "recipients": [
            {
                "recipient": {
                    "address": "andr1new_partner1...",
                    "msg": null
                },
                "weight": "40"
            },
            {
                "recipient": {
                    "address": "andr1new_partner2...",
                    "msg": null
                },
                "weight": "60"
            }
        ]
    }
}
```

### UpdateRecipientWeight
Updates the weight of a specific recipient without affecting others.

_**Note:** Only contract owner can execute when unlocked._

```rust
UpdateRecipientWeight { 
    recipient: AddressWeight 
}
```

```json
{
    "update_recipient_weight": {
        "recipient": {
            "recipient": {
                "address": "andr1partner1...",
                "msg": null
            },
            "weight": "75"
        }
    }
}
```

### AddRecipient
Adds a new recipient to the distribution list.

_**Note:** Only contract owner can execute when unlocked. Maximum 100 recipients._

```rust
AddRecipient { 
    recipient: AddressWeight 
}
```

```json
{
    "add_recipient": {
        "recipient": {
            "recipient": {
                "address": "andr1new_partner...",
                "msg": null
            },
            "weight": "25"
        }
    }
}
```

### RemoveRecipient
Removes a recipient from the distribution list.

_**Note:** Only contract owner can execute when unlocked._

```rust
RemoveRecipient { 
    recipient: AndrAddr 
}
```

```json
{
    "remove_recipient": {
        "recipient": "andr1partner_to_remove..."
    }
}
```

### UpdateLock
Updates the lock time for configuration security.

_**Note:** Only contract owner can execute when unlocked._

```rust
UpdateLock { 
    lock_time: Expiry 
}
```

```json
{
    "update_lock": {
        "lock_time": {
            "at_time": "1641081600000000000"
        }
    }
}
```

### UpdateDefaultRecipient
Updates the default recipient for remainder funds.

_**Note:** Only contract owner can execute when unlocked._

```rust
UpdateDefaultRecipient { 
    recipient: Option<Recipient> 
}
```

```json
{
    "update_default_recipient": {
        "recipient": {
            "address": "andr1new_treasury...",
            "msg": null
        }
    }
}
```

## QueryMsg

### GetSplitterConfig
Returns the current splitter configuration including all recipients, weights, lock status, and default recipient.

```rust
pub enum QueryMsg {
    #[returns(GetSplitterConfigResponse)]
    GetSplitterConfig {},
}
```

```json
{
    "get_splitter_config": {}
}
```

**Response:**
```json
{
    "config": {
        "recipients": [
            {
                "recipient": {
                    "address": "andr1partner1...",
                    "msg": null
                },
                "weight": "50"
            },
            {
                "recipient": {
                    "address": "andr1partner2...",
                    "msg": null
                },
                "weight": "30"
            }
        ],
        "lock": {
            "expiration": {
                "at_time": "1640995200000000000"
            }
        },
        "default_recipient": {
            "address": "andr1treasury...",
            "msg": null
        }
    }
}
```

### GetUserWeight
Returns a specific recipient's weight along with the total weight for perspective.

```rust
pub enum QueryMsg {
    #[returns(GetUserWeightResponse)]
    GetUserWeight { user: AndrAddr },
}
```

```json
{
    "get_user_weight": {
        "user": "andr1partner1..."
    }
}
```

**Response:**
```json
{
    "weight": "50",
    "total_weight": "100"
}
```

This response shows the user has a weight of 50 out of a total weight of 100, meaning they receive 50% of distributed funds.

## Usage Examples

### Revenue Sharing Partnership
```json
{
    "recipients": [
        {
            "recipient": {"address": "andr1founder..."},
            "weight": "40"
        },
        {
            "recipient": {"address": "andr1cto..."},
            "weight": "30"
        },
        {
            "recipient": {"address": "andr1marketing..."},
            "weight": "20"
        },
        {
            "recipient": {"address": "andr1operations..."},
            "weight": "10"
        }
    ]
}
```
**Result**: Founder gets 40%, CTO gets 30%, Marketing gets 20%, Operations gets 10%

### Stake-Based Rewards
```json
{
    "recipients": [
        {
            "recipient": {"address": "andr1validator1..."},
            "weight": "1000000"
        },
        {
            "recipient": {"address": "andr1validator2..."},
            "weight": "750000"
        },
        {
            "recipient": {"address": "andr1validator3..."},
            "weight": "500000"
        }
    ]
}
```
**Result**: Proportional distribution based on stake amounts (actual token amounts as weights)

### Royalty Distribution
```json
{
    "recipients": [
        {
            "recipient": {"address": "andr1artist..."},
            "weight": "70"
        },
        {
            "recipient": {"address": "andr1producer..."},
            "weight": "20"
        },
        {
            "recipient": {"address": "andr1platform..."},
            "weight": "10"
        }
    ]
}
```
**Result**: Artist gets 70%, Producer gets 20%, Platform gets 10% of royalties

### DAO Treasury Distribution
```json
{
    "recipients": [
        {
            "recipient": {"address": "andr1development_fund..."},
            "weight": "50"
        },
        {
            "recipient": {"address": "andr1marketing_fund..."},
            "weight": "25"
        },
        {
            "recipient": {"address": "andr1reserve_fund..."},
            "weight": "25"
        }
    ]
}
```

## Integration Patterns

### With App Contract
The Weighted Distribution Splitter can be integrated into App contracts for automated proportional distributions:

```json
{
    "components": [
        {
            "name": "revenue_splitter",
            "ado_type": "weighted-distribution-splitter",
            "component_type": {
                "new": {
                    "recipients": [
                        {
                            "recipient": {"address": "andr1partner1..."},
                            "weight": "60"
                        },
                        {
                            "recipient": {"address": "andr1partner2..."},
                            "weight": "40"
                        }
                    ]
                }
            }
        }
    ]
}
```

### DeFi Protocol Revenue
For protocol fee distribution:

1. **Configure stakeholders** with appropriate weights
2. **Collect protocol fees** in treasury contract
3. **Periodically distribute** accumulated fees
4. **Adjust weights** based on governance decisions

### Multi-Token Distributions
For complex token allocations:

1. **Send multiple token types** in single transaction
2. **Each token distributed** proportionally based on weights
3. **Remainder handling** for each token type separately
4. **Gas optimization** through batch processing

### Dynamic Weight Management
For adaptive distributions:

1. **Start with initial weights** based on contributions
2. **Add new recipients** as team/partnership grows
3. **Update weights** based on performance or new agreements
4. **Remove recipients** who leave the partnership

## Weight Management Strategies

### Equal Distribution
```json
[
    {"recipient": {"address": "andr1user1..."}, "weight": "100"},
    {"recipient": {"address": "andr1user2..."}, "weight": "100"},
    {"recipient": {"address": "andr1user3..."}, "weight": "100"}
]
```
**Result**: Each recipient gets 33.33% (100/300)

### Proportional Shares
```json
[
    {"recipient": {"address": "andr1major..."}, "weight": "500"},
    {"recipient": {"address": "andr1minor1..."}, "weight": "300"},
    {"recipient": {"address": "andr1minor2..."}, "weight": "200"}
]
```
**Result**: Major gets 50% (500/1000), Minor1 gets 30%, Minor2 gets 20%

### Performance-Based
```json
[
    {"recipient": {"address": "andr1top_performer..."}, "weight": "150"},
    {"recipient": {"address": "andr1good_performer..."}, "weight": "100"},
    {"recipient": {"address": "andr1standard_performer..."}, "weight": "75"}
]
```
**Result**: Performance-based distribution with 46.15%, 30.77%, 23.08% respectively

## Remainder Handling

Due to integer arithmetic and rounding, small remainder amounts may exist after distribution. These are handled as follows:

1. **Calculate exact amounts** using integer division
2. **Track remainder** from rounding down
3. **Send remainder** to default recipient or original sender
4. **Ensure no funds lost** - all sent funds are distributed

### Example with Remainders
- **Sent**: 1000 tokens
- **Weights**: A=33, B=33, C=34 (Total=100)
- **Distribution**: A=330, B=330, C=340
- **Remainder**: 0 tokens (perfect division)

If weights were A=33, B=33, C=33 (Total=99):
- **Distribution**: A=333, B=333, C=333
- **Remainder**: 1 token → goes to default recipient

## Validation Rules

### Recipient Validation
- **Minimum recipients**: At least 1 recipient required
- **Maximum recipients**: Maximum 100 recipients allowed
- **Unique addresses**: No duplicate recipient addresses
- **Valid addresses**: All recipient addresses must be valid

### Weight Validation
- **Positive weights**: All weights must be greater than zero
- **No zero weights**: Zero weights are not allowed
- **Integer weights**: Weights must be valid Uint128 values
- **Weight limits**: Practical limits based on total weight calculations

### Transaction Validation
- **Minimum funds**: At least 1 coin must be sent
- **Maximum coin types**: Maximum 5 different coin types per transaction
- **Non-zero amounts**: All coin amounts must be positive

## Security Features

### Lock Mechanism
- **Configuration protection**: Prevents unauthorized changes when locked
- **Time-based security**: Lock automatically expires after specified time
- **Owner control**: Only owner can update lock settings and configuration
- **Flexible locking**: Lock duration can be updated when unlocked

### Access Controls
- **Owner restrictions**: Only owner can modify recipients and weights
- **Lock enforcement**: Configuration changes blocked when locked
- **Address validation**: All recipient addresses validated before storage
- **Duplicate prevention**: Prevents adding duplicate recipients

## Important Notes

- **Proportional distribution**: Recipients receive percentages, not fixed amounts
- **Dynamic calculations**: Percentages change when weights are modified
- **Multi-token support**: Up to 5 different token types per transaction
- **Remainder handling**: Small remainders from rounding go to default recipient
- **Weight flexibility**: Weights can be any positive integer value
- **Gas optimization**: Efficient batch processing for multiple recipients
- **Lock security**: Configuration can be locked for specified time periods

## Performance Considerations

### Optimization Strategies
- **Reasonable recipient count**: Keep recipient list manageable for gas efficiency
- **Weight sizing**: Use reasonable weight values to prevent calculation issues
- **Batch distributions**: Send multiple token types together for efficiency
- **Lock periods**: Use appropriate lock periods for security vs. flexibility

### Scalability
- **100 recipient limit**: Hard limit to prevent gas issues
- **Multi-token processing**: Handle up to 5 token types efficiently
- **Weight calculations**: Optimized for large weight values
- **Query efficiency**: Fast lookups for individual recipient weights

The Weighted Distribution Splitter ADO provides a powerful, flexible solution for proportional fund distribution in blockchain applications, offering the mathematical precision and security controls needed for complex financial arrangements while maintaining gas efficiency and ease of use.