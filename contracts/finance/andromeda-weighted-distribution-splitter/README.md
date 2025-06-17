# Andromeda Weighted Distribution Splitter ADO

## Introduction

The Andromeda Weighted Distribution Splitter ADO is an advanced financial distribution contract that automatically splits funds to specified recipients based on weighted proportions rather than fixed amounts. This contract enables sophisticated fund allocation where each recipient receives a percentage of the total funds based on their assigned weight relative to the total weight of all recipients. The splitter supports dynamic weight management, time-based locking, and flexible recipient administration, making it ideal for profit sharing, dividend distribution, weighted payment systems, and proportional allocation scenarios.

<b>Ado_type:</b> weighted-distribution-splitter

## Why Weighted Distribution Splitter ADO

The Weighted Distribution Splitter ADO serves as advanced financial infrastructure for applications requiring:

- **Profit Sharing**: Distribute profits proportionally based on ownership stakes or contribution weights
- **Dividend Distribution**: Automated dividend payments to shareholders based on their holdings
- **Revenue Sharing**: Split revenues among partners, employees, or stakeholders with weighted allocations
- **Investment Returns**: Distribute investment returns proportionally to investor contributions
- **Commission Structures**: Weighted commission distribution based on sales performance or seniority
- **Staking Rewards**: Distribute staking rewards proportionally to stake amounts
- **Royalty Distribution**: Split royalties among creators, publishers, and distributors
- **Pool Distributions**: Distribute funds from liquidity pools or reward pools based on participation
- **Performance Bonuses**: Allocate bonuses based on weighted performance metrics
- **Governance Rewards**: Distribute governance rewards based on voting power or participation

The ADO provides precise percentage-based control, dynamic weight management, and lock-time security for reliable proportional distribution operations.

## Key Features

### **Weighted Distribution Logic**
- **Proportional allocation**: Each recipient receives funds proportional to their weight
- **Dynamic weighting**: Weights can be updated, added, or removed when contract is unlocked
- **Precise calculations**: Mathematical precision in percentage-based distributions
- **Remainder handling**: Configurable handling of remainder funds after weighted distributions
- **Multi-token support**: Distribute multiple token types in a single operation

### **Flexible Weight Management**
- **Individual weight updates**: Update specific recipient weights without affecting others
- **Add/remove recipients**: Dynamic recipient list management with weight validation
- **Weight validation**: Ensure all weights are positive and meaningful
- **Total weight queries**: Query individual and total weights for transparency
- **Bulk recipient updates**: Replace entire recipient list when needed

### **Security and Lock Management**
- **Time-based locking**: Lock contract configuration for security periods
- **Configuration protection**: Prevent unauthorized configuration changes during lock
- **Owner controls**: Only contract owner can modify configuration and weights
- **Lock duration validation**: Validate lock periods for security compliance
- **Emergency unlocking**: Automatic unlocking after lock period expires

### **Advanced Fund Handling**
- **Multi-coin support**: Handle up to 5 different token types per distribution
- **Native token support**: Full support for native blockchain tokens
- **Default recipient**: Configure default recipient for remainder funds
- **AMP integration**: Full integration with Andromeda Messaging Protocol
- **Atomic operations**: All distributions occur atomically or fail completely

## Weight Calculation Logic

### **Proportional Distribution**
The contract distributes funds based on weighted proportions:
1. **Calculate total weight** of all recipients
2. **Determine proportions** for each recipient (recipient_weight / total_weight)
3. **Calculate distributions** for each token type and recipient
4. **Send weighted amounts** to respective recipients
5. **Handle remainders** according to default recipient configuration

### **Mathematical Formula**
For each recipient and token:
```
recipient_amount = total_amount × (recipient_weight / total_weight)
```

### **Remainder Handling**
Due to rounding in percentage calculations:
- **Remainder funds**: Any remaining tokens after weighted distribution
- **Default recipient**: Send remainders to configured default recipient
- **Fallback to sender**: If no default recipient, return to original sender
- **Multi-token support**: Handle remainders for each token type separately

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
            "weight": "500"
        },
        {
            "recipient": {
                "address": "andr1partner2...",
                "msg": null
            },
            "weight": "300"
        },
        {
            "recipient": {
                "address": "andr1partner3...",
                "msg": null
            },
            "weight": "200"
        }
    ],
    "lock_time": {
        "at_time": "1672617600000000000"
    },
    "default_recipient": {
        "address": "andr1treasury...",
        "msg": null
    }
}
```

**Parameters**:
- **recipients**: List of recipients with their weights
  - **recipient**: Recipient address and optional message
  - **weight**: Positive integer representing the recipient's weight
- **lock_time**: Optional lock expiration time for configuration protection
- **default_recipient**: Optional default recipient for remainder funds

**Validation**:
- Must have at least 1 recipient, maximum 100 recipients
- All weights must be positive (greater than zero)
- No duplicate recipients allowed
- All addresses must be valid
- Total recipients cannot exceed 100

## ExecuteMsg

### Send
Distributes attached funds according to the weighted configuration.

```rust
Send {
    config: Option<Vec<AddressWeight>>,
}
```

```json
{
    "send": {
        "config": null
    }
}
```

**Usage**: Send native tokens as funds with this message. The contract will:
1. Calculate total weight of all recipients
2. Distribute funds proportionally based on weights
3. Send any remainder to the default recipient or sender

**Parameters**:
- **config**: Optional override configuration (only works when contract is unlocked)

**Requirements**:
- Must send 1-5 native token denominations as funds
- Cannot send zero amounts
- Config override only available when contract is unlocked

### UpdateRecipients
Updates the entire recipient list with new weights (owner-only, when unlocked).

```rust
UpdateRecipients {
    recipients: Vec<AddressWeight>,
}
```

```json
{
    "update_recipients": {
        "recipients": [
            {
                "recipient": {
                    "address": "andr1new_partner...",
                    "msg": null
                },
                "weight": "400"
            },
            {
                "recipient": {
                    "address": "andr1existing_partner...",
                    "msg": null
                },
                "weight": "600"
            }
        ]
    }
}
```

**Requirements**:
- Only contract owner can execute
- Contract must not be locked
- New recipient list must pass validation
- All weights must be positive

### UpdateRecipientWeight
Updates a specific recipient's weight (owner-only, when unlocked).

```rust
UpdateRecipientWeight {
    recipient: AddressWeight,
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
            "weight": "750"
        }
    }
}
```

**Requirements**:
- Only contract owner can execute
- Contract must not be locked
- Recipient must exist in current list
- Weight must be positive

### AddRecipient
Adds a new recipient to the list (owner-only, when unlocked).

```rust
AddRecipient {
    recipient: AddressWeight,
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
            "weight": "250"
        }
    }
}
```

**Requirements**:
- Only contract owner can execute
- Contract must not be locked
- Recipient must not already exist
- Weight must be positive
- Total recipients must not exceed 100

### RemoveRecipient
Removes a recipient from the list (owner-only, when unlocked).

```rust
RemoveRecipient {
    recipient: AndrAddr,
}
```

```json
{
    "remove_recipient": {
        "recipient": "andr1partner_to_remove..."
    }
}
```

**Requirements**:
- Only contract owner can execute
- Contract must not be locked
- Recipient must exist in current list

### UpdateLock
Updates the lock expiration time (owner-only, when unlocked).

```rust
UpdateLock {
    lock_time: Expiry,
}
```

```json
{
    "update_lock": {
        "lock_time": {
            "at_time": "1704067200000000000"
        }
    }
}
```

**Parameters**:
- **lock_time**: New lock expiration time

**Requirements**:
- Only contract owner can execute
- Contract must not be currently locked
- New lock time must be valid

### UpdateDefaultRecipient
Updates the default recipient for remainder funds (owner-only, when unlocked).

```rust
UpdateDefaultRecipient {
    recipient: Option<Recipient>,
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

**Parameters**:
- **recipient**: New default recipient (or null to remove)

**Requirements**:
- Only contract owner can execute
- Contract must not be locked
- Recipient address must be valid if provided

## QueryMsg

### GetSplitterConfig
Returns the current splitter configuration.

```rust
#[returns(GetSplitterConfigResponse)]
GetSplitterConfig {}

pub struct GetSplitterConfigResponse {
    pub config: Splitter,
}

pub struct Splitter {
    pub recipients: Vec<AddressWeight>,
    pub lock: MillisecondsExpiration,
    pub default_recipient: Option<Recipient>,
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
                "weight": "500"
            },
            {
                "recipient": {
                    "address": "andr1partner2...",
                    "msg": null
                },
                "weight": "300"
            }
        ],
        "lock": "1672617600000",
        "default_recipient": {
            "address": "andr1treasury...",
            "msg": null
        }
    }
}
```

### GetUserWeight
Returns a specific user's weight and the total weight of all recipients.

```rust
#[returns(GetUserWeightResponse)]
GetUserWeight {
    user: AndrAddr,
}

pub struct GetUserWeightResponse {
    pub weight: Uint128,
    pub total_weight: Uint128,
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
    "weight": "500",
    "total_weight": "1000"
}
```

## Usage Examples

### Profit Sharing Partnership
```json
{
    "recipients": [
        {
            "recipient": {
                "address": "andr1founder...",
                "msg": null
            },
            "weight": "500"
        },
        {
            "recipient": {
                "address": "andr1investor1...",
                "msg": null
            },
            "weight": "300"
        },
        {
            "recipient": {
                "address": "andr1investor2...",
                "msg": null
            },
            "weight": "200"
        }
    ],
    "lock_time": {
        "time": 2629746
    },
    "default_recipient": {
        "address": "andr1company_reserve...",
        "msg": null
    }
}
```
_Founder gets 50%, Investor1 gets 30%, Investor2 gets 20%_

### Employee Bonus Distribution
```json
{
    "recipients": [
        {
            "recipient": {
                "address": "andr1senior_dev...",
                "msg": null
            },
            "weight": "400"
        },
        {
            "recipient": {
                "address": "andr1junior_dev...",
                "msg": null
            },
            "weight": "200"
        },
        {
            "recipient": {
                "address": "andr1designer...",
                "msg": null
            },
            "weight": "200"
        },
        {
            "recipient": {
                "address": "andr1marketing...",
                "msg": null
            },
            "weight": "200"
        }
    ],
    "lock_time": null,
    "default_recipient": null
}
```
_Senior gets 40%, others get 20% each_

### Staking Rewards Distribution
```json
{
    "recipients": [
        {
            "recipient": {
                "address": "andr1validator1...",
                "msg": null
            },
            "weight": "1000"
        },
        {
            "recipient": {
                "address": "andr1validator2...",
                "msg": null
            },
            "weight": "750"
        },
        {
            "recipient": {
                "address": "andr1validator3...",
                "msg": null
            },
            "weight": "500"
        }
    ],
    "lock_time": {
        "at_time": "1704067200000000000"
    },
    "default_recipient": {
        "address": "andr1staking_pool...",
        "msg": null
    }
}
```

## Operational Examples

### Execute Weighted Distribution
```json
{
    "send": {
        "config": null
    }
}
```
_Send 1,000,000 uandr as funds to distribute based on weights_

### Update Individual Weight
```json
{
    "update_recipient_weight": {
        "recipient": {
            "recipient": {
                "address": "andr1partner1...",
                "msg": null
            },
            "weight": "600"
        }
    }
}
```

### Add New Recipient
```json
{
    "add_recipient": {
        "recipient": {
            "recipient": {
                "address": "andr1new_partner...",
                "msg": null
            },
            "weight": "150"
        }
    }
}
```

### Remove Recipient
```json
{
    "remove_recipient": {
        "recipient": "andr1former_partner..."
    }
}
```

### Query User's Share
```json
{
    "get_user_weight": {
        "user": "andr1partner1..."
    }
}
```

### Lock Configuration
```json
{
    "update_lock": {
        "lock_time": {
            "time": 5259492
        }
    }
}
```

## Integration Patterns

### With App Contract
Weighted splitter can be integrated for revenue sharing:

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
                            "recipient": {
                                "address": "andr1operations...",
                                "msg": null
                            },
                            "weight": "400"
                        },
                        {
                            "recipient": {
                                "address": "andr1development...",
                                "msg": null
                            },
                            "weight": "300"
                        },
                        {
                            "recipient": {
                                "address": "andr1marketing...",
                                "msg": null
                            },
                            "weight": "300"
                        }
                    ],
                    "lock_time": {
                        "time": 2629746
                    },
                    "default_recipient": null
                }
            }
        }
    ]
}
```

### Revenue Sharing System
For automated revenue distribution:

1. **Deploy splitter** with stakeholder addresses and weights
2. **Lock configuration** for quarterly periods
3. **Execute distributions** when revenue is collected
4. **Update weights** based on changing ownership or contributions
5. **Handle remainder funds** through company treasury

### Investment Fund Distribution
For proportional investment returns:

1. **Set investor weights** based on investment amounts
2. **Configure quarterly lock periods** for stability
3. **Distribute returns** proportionally to investments
4. **Update investor lists** when new investors join
5. **Track individual allocations** through weight queries

### Team Incentive Programs
For performance-based team rewards:

1. **Define team member weights** based on roles and performance
2. **Set monthly distribution cycles** through lock management
3. **Distribute incentives** based on team performance metrics
4. **Adjust weights** based on performance reviews
5. **Add/remove team members** as team composition changes

### DAO Treasury Management
For decentralized fund allocation:

1. **Configure workstream allocations** with appropriate weights
2. **Set governance-approved lock periods** for fund security
3. **Execute treasury distributions** based on DAO decisions
4. **Update allocations** through governance proposals
5. **Manage fund remainders** through DAO treasury

## Advanced Features

### **Dynamic Weight Management**
- **Individual updates**: Update specific recipient weights without affecting others
- **Bulk operations**: Replace entire recipient lists efficiently
- **Add/remove functionality**: Dynamic list management with validation
- **Weight validation**: Ensure all weights are meaningful and positive

### **Proportional Calculations**
- **Mathematical precision**: Accurate percentage-based distributions
- **Multi-token support**: Handle multiple token types in single distribution
- **Remainder handling**: Proper handling of rounding remainders
- **Total weight tracking**: Transparent weight calculations and queries

### **Configuration Security**
- **Time-based locking**: Prevent configuration changes during lock periods
- **Owner-only controls**: Restrict configuration changes to contract owner
- **Validation checks**: Comprehensive validation of all configuration changes
- **Emergency management**: Automatic unlocking after expiration

### **Advanced Querying**
- **Individual weight queries**: Query specific recipient weights and percentages
- **Total weight calculation**: Understand overall weight distribution
- **Configuration inspection**: Complete view of splitter setup
- **Weight percentage calculation**: Calculate actual percentage allocations

## Security Features

### **Access Control**
- **Owner restrictions**: Only contract owner can modify configuration
- **Lock enforcement**: Prevent unauthorized changes during lock periods
- **Address validation**: Comprehensive validation of all addresses
- **Permission verification**: Verify permissions before configuration changes

### **Weight Validation**
- **Positive weights**: Ensure all weights are greater than zero
- **Duplicate prevention**: Prevent duplicate recipients in lists
- **Limit enforcement**: Enforce maximum recipient limits
- **Mathematical safety**: Prevent overflow in weight calculations

### **Fund Protection**
- **Atomic distributions**: All distributions occur atomically or fail completely
- **Balance verification**: Verify sufficient funds before distribution
- **Multi-token safety**: Safe handling of multiple token types
- **Remainder protection**: Secure handling of remainder funds

### **Configuration Protection**
- **Lock validation**: Ensure lock times are within valid ranges
- **Recipient limits**: Limit number of recipients to prevent gas issues
- **State consistency**: Maintain consistent state across all operations
- **Error handling**: Comprehensive error handling and recovery

## Important Notes

- **Weighted distribution**: Recipients receive proportional amounts based on weights
- **Remainder handling**: Surplus funds go to default recipient or sender
- **Lock restrictions**: Cannot modify configuration while contract is locked
- **Multi-token support**: Support for up to 5 different token types per distribution
- **Recipient limits**: Maximum 100 recipients per configuration
- **Weight requirements**: All weights must be positive integers
- **Owner privileges**: Only contract owner can modify configuration
- **Proportional calculation**: Exact formula: amount × (weight / total_weight)

## Common Workflow

### 1. **Deploy Weighted Splitter**
```json
{
    "recipients": [
        {
            "recipient": {
                "address": "andr1partner1...",
                "msg": null
            },
            "weight": "500"
        },
        {
            "recipient": {
                "address": "andr1partner2...",
                "msg": null
            },
            "weight": "300"
        }
    ],
    "lock_time": {
        "time": 2629746
    },
    "default_recipient": null
}
```

### 2. **Execute Distribution**
```json
{
    "send": {
        "config": null
    }
}
```
_Send funds as transaction fees._

### 3. **Query Weight Information**
```json
{
    "get_user_weight": {
        "user": "andr1partner1..."
    }
}
```

### 4. **Update Recipient Weight**
```json
{
    "update_recipient_weight": {
        "recipient": {
            "recipient": {
                "address": "andr1partner1...",
                "msg": null
            },
            "weight": "600"
        }
    }
}
```

### 5. **Add New Recipient**
```json
{
    "add_recipient": {
        "recipient": {
            "recipient": {
                "address": "andr1new_partner...",
                "msg": null
            },
            "weight": "200"
        }
    }
}
```

### 6. **Lock Configuration**
```json
{
    "update_lock": {
        "lock_time": {
            "time": 5259492
        }
    }
}
```

### 7. **Query Full Configuration**
```json
{
    "get_splitter_config": {}
}
```

The Weighted Distribution Splitter ADO provides sophisticated proportional fund distribution infrastructure for the Andromeda ecosystem, enabling flexible, secure, and mathematically precise weighted allocation systems for complex financial applications.