# Andromeda Conditional Splitter ADO

## Introduction

The Andromeda Conditional Splitter ADO is an advanced fund distribution contract that automatically allocates funds to different recipients based on configurable threshold conditions. This ADO enables dynamic distribution logic where the allocation percentages change depending on the amount received, making it ideal for tiered commission structures, progressive fee systems, and conditional reward mechanisms with lock-time security features.

<b>Ado_type:</b> conditional-splitter

## Why Conditional Splitter ADO

The Conditional Splitter ADO serves as a sophisticated distribution mechanism for applications requiring:

- **Tiered Commission Systems**: Different commission rates based on sales volume or transaction amounts
- **Progressive Fee Structures**: Higher service fees for larger transactions or premium services
- **Volume-Based Rewards**: Different reward distributions based on staking amounts or contribution levels
- **Conditional Revenue Sharing**: Dynamic revenue splits based on performance milestones
- **Threshold-Based Allocations**: Automated fund allocation based on predefined amount ranges
- **Sales Incentive Programs**: Variable incentive distributions based on achievement tiers
- **Dynamic Treasury Management**: Conditional fund allocation for different operational needs
- **Performance-Based Distributions**: Reward structures that scale with performance metrics
- **Graduated Fee Systems**: Fee structures that change based on usage or volume tiers
- **Smart Contract Automation**: Eliminate manual intervention in complex distribution logic

The ADO provides flexible threshold-based distribution with lock-time security, automatic remainder handling, and comprehensive validation for reliable conditional fund management.

## Key Features

### **Threshold-Based Distribution**
- **Multiple thresholds**: Define unlimited conditional distribution tiers
- **Minimum amount triggers**: Each threshold activated by minimum amount requirements
- **Dynamic allocation**: Different recipient lists and percentages per threshold
- **Automatic selection**: Contract automatically selects appropriate threshold based on amount

### **Flexible Recipient Management**
- **Per-threshold recipients**: Each threshold can have unique recipient configurations
- **Percentage-based allocation**: Define exact percentage allocation for each recipient
- **Remainder handling**: Automatically return unallocated funds to sender
- **Recipient validation**: Comprehensive validation of recipient addresses and percentages

### **Security Features**
- **Lock-time protection**: Time-based locking to prevent unauthorized threshold updates
- **Owner restrictions**: Only contract owner can modify thresholds and lock settings
- **Validation enforcement**: Automatic validation of thresholds, percentages, and recipients
- **Duplicate prevention**: Prevent duplicate thresholds and recipients within same tier

### **Administrative Controls**
- **Threshold updates**: Modify distribution logic when contract is unlocked
- **Lock management**: Set lock periods between 1 day and 1 year
- **Configuration queries**: Query current threshold configuration and lock status
- **Emergency controls**: Owner can update configurations when security permits

## Threshold Logic

### **Threshold Selection Algorithm**
The contract automatically selects the appropriate threshold using the following logic:

1. **Sort thresholds**: Order all thresholds by minimum amount in descending order
2. **Find match**: Return first threshold where `amount >= threshold.min`
3. **Apply distribution**: Use that threshold's recipient configuration
4. **Handle remainder**: Return any undistributed funds to sender

### **Example Threshold Selection**
Given thresholds with minimums: `[1000, 500, 100]` and amount `750`:
- Check `750 >= 1000`? **No**
- Check `750 >= 500`? **Yes** → Use 500 threshold
- Apply 500 threshold's distribution rules

## InstantiateMsg

```rust
pub struct InstantiateMsg {
    pub thresholds: Vec<Threshold>,
    pub lock_time: Option<Expiry>,
}

pub struct Threshold {
    pub min: Uint128,
    pub address_percent: Vec<AddressPercent>,
}

pub struct AddressPercent {
    pub recipient: Recipient,
    pub percent: Decimal,
}
```

```json
{
    "thresholds": [
        {
            "min": "1000000",
            "address_percent": [
                {
                    "recipient": {
                        "address": "andr1recipient1...",
                        "msg": null
                    },
                    "percent": "0.3"
                },
                {
                    "recipient": {
                        "address": "andr1recipient2...",
                        "msg": null
                    },
                    "percent": "0.2"
                }
            ]
        },
        {
            "min": "100000",
            "address_percent": [
                {
                    "recipient": {
                        "address": "andr1recipient1...",
                        "msg": null
                    },
                    "percent": "0.2"
                },
                {
                    "recipient": {
                        "address": "andr1recipient3...",
                        "msg": null
                    },
                    "percent": "0.1"
                }
            ]
        }
    ],
    "lock_time": {
        "at_time": "1641081600000000000"
    }
}
```

**Parameters**:
- **thresholds**: Array of threshold configurations
  - **min**: Minimum amount to trigger this threshold
  - **address_percent**: Recipients and their allocation percentages
    - **recipient**: Recipient address and optional message
    - **percent**: Percentage allocation (decimal between 0 and 1)
- **lock_time**: Optional lock expiration time
  - Must be between 1 day and 1 year from instantiation
  - Prevents threshold updates until expiration

## ExecuteMsg

### Send
Distributes attached funds according to threshold configuration.

```rust
Send {}
```

```json
{
    "send": {}
}
```

**Usage**: Send native tokens as funds with this message. The contract will:
1. Determine appropriate threshold based on total amount
2. Distribute funds to threshold recipients according to percentages
3. Return any remainder to the message sender

**Validation**:
- At least one coin must be sent
- Maximum 5 different coin denominations allowed
- All coin amounts must be non-zero
- Total amount must meet at least one threshold minimum

### UpdateThresholds
Updates the threshold configuration (owner-only, when unlocked).

```rust
UpdateThresholds {
    thresholds: Vec<Threshold>,
}
```

```json
{
    "update_thresholds": {
        "thresholds": [
            {
                "min": "2000000",
                "address_percent": [
                    {
                        "recipient": {
                            "address": "andr1new_recipient...",
                            "msg": null
                        },
                        "percent": "0.4"
                    }
                ]
            },
            {
                "min": "500000",
                "address_percent": [
                    {
                        "recipient": {
                            "address": "andr1recipient1...",
                            "msg": null
                        },
                        "percent": "0.25"
                    },
                    {
                        "recipient": {
                            "address": "andr1recipient2...",
                            "msg": null
                        },
                        "percent": "0.15"
                    }
                ]
            }
        ]
    }
}
```

**Requirements**:
- Only contract owner can execute
- Contract must not be locked (lock_time must be expired)
- New thresholds must pass all validation rules

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
            "at_time": "1672617600000000000"
        }
    }
}
```

**Parameters**:
- **lock_time**: New lock expiration time
  - Must be between 1 day and 1 year from current time
  - Prevents threshold updates until this time

**Requirements**:
- Only contract owner can execute
- Contract must not be currently locked
- New lock time must be within valid range

## QueryMsg

### GetConditionalSplitterConfig
Returns the current threshold configuration and lock status.

```rust
pub enum QueryMsg {
    #[returns(GetConditionalSplitterConfigResponse)]
    GetConditionalSplitterConfig {},
}
```

```json
{
    "get_conditional_splitter_config": {}
}
```

**Response:**
```json
{
    "config": {
        "thresholds": [
            {
                "min": "1000000",
                "address_percent": [
                    {
                        "recipient": {
                            "address": "andr1recipient1...",
                            "msg": null
                        },
                        "percent": "0.3"
                    },
                    {
                        "recipient": {
                            "address": "andr1recipient2...",
                            "msg": null
                        },
                        "percent": "0.2"
                    }
                ]
            }
        ],
        "lock_time": "1641081600000"
    }
}
```

## Threshold Examples

### Sales Commission Structure
```json
{
    "thresholds": [
        {
            "min": "100000000000",
            "address_percent": [
                {
                    "recipient": {
                        "address": "andr1top_salesperson...",
                        "msg": null
                    },
                    "percent": "0.15"
                },
                {
                    "recipient": {
                        "address": "andr1company_treasury...",
                        "msg": null
                    },
                    "percent": "0.05"
                }
            ]
        },
        {
            "min": "10000000000",
            "address_percent": [
                {
                    "recipient": {
                        "address": "andr1salesperson...",
                        "msg": null
                    },
                    "percent": "0.1"
                },
                {
                    "recipient": {
                        "address": "andr1company_treasury...",
                        "msg": null
                    },
                    "percent": "0.03"
                }
            ]
        },
        {
            "min": "1000000000",
            "address_percent": [
                {
                    "recipient": {
                        "address": "andr1salesperson...",
                        "msg": null
                    },
                    "percent": "0.05"
                }
            ]
        }
    ]
}
```
**Logic**: 
- Sales > 100,000 tokens: 15% to top salesperson + 5% to treasury
- Sales > 10,000 tokens: 10% to salesperson + 3% to treasury  
- Sales > 1,000 tokens: 5% to salesperson

### Staking Reward Tiers
```json
{
    "thresholds": [
        {
            "min": "1000000000000",
            "address_percent": [
                {
                    "recipient": {
                        "address": "andr1whale_staker...",
                        "msg": null
                    },
                    "percent": "0.8"
                },
                {
                    "recipient": {
                        "address": "andr1bonus_pool...",
                        "msg": null
                    },
                    "percent": "0.1"
                }
            ]
        },
        {
            "min": "100000000000",
            "address_percent": [
                {
                    "recipient": {
                        "address": "andr1large_staker...",
                        "msg": null
                    },
                    "percent": "0.7"
                }
            ]
        },
        {
            "min": "10000000000",
            "address_percent": [
                {
                    "recipient": {
                        "address": "andr1regular_staker...",
                        "msg": null
                    },
                    "percent": "0.6"
                }
            ]
        }
    ]
}
```

### Service Fee Structure
```json
{
    "thresholds": [
        {
            "min": "50000000000",
            "address_percent": [
                {
                    "recipient": {
                        "address": "andr1premium_service...",
                        "msg": null
                    },
                    "percent": "0.02"
                },
                {
                    "recipient": {
                        "address": "andr1development_fund...",
                        "msg": null
                    },
                    "percent": "0.01"
                }
            ]
        },
        {
            "min": "5000000000",
            "address_percent": [
                {
                    "recipient": {
                        "address": "andr1standard_service...",
                        "msg": null
                    },
                    "percent": "0.015"
                }
            ]
        },
        {
            "min": "500000000",
            "address_percent": [
                {
                    "recipient": {
                        "address": "andr1basic_service...",
                        "msg": null
                    },
                    "percent": "0.01"
                }
            ]
        }
    ]
}
```

### Revenue Sharing Model
```json
{
    "thresholds": [
        {
            "min": "1000000000000",
            "address_percent": [
                {
                    "recipient": {
                        "address": "andr1partner_a...",
                        "msg": null
                    },
                    "percent": "0.4"
                },
                {
                    "recipient": {
                        "address": "andr1partner_b...",
                        "msg": null
                    },
                    "percent": "0.3"
                },
                {
                    "recipient": {
                        "address": "andr1operations...",
                        "msg": null
                    },
                    "percent": "0.2"
                }
            ]
        },
        {
            "min": "100000000000",
            "address_percent": [
                {
                    "recipient": {
                        "address": "andr1partner_a...",
                        "msg": null
                    },
                    "percent": "0.35"
                },
                {
                    "recipient": {
                        "address": "andr1partner_b...",
                        "msg": null
                    },
                    "percent": "0.25"
                },
                {
                    "recipient": {
                        "address": "andr1operations...",
                        "msg": null
                    },
                    "percent": "0.15"
                }
            ]
        }
    ]
}
```

## Usage Examples

### Basic Conditional Distribution
```bash
# Send 1,500 tokens to trigger the 1,000 threshold
andr tx wasm execute <conditional-splitter-addr> \
  '{"send":{}}' \
  --amount 1500000000utoken \
  --from sender
```

### Update Thresholds (Owner Only)
```json
{
    "update_thresholds": {
        "thresholds": [
            {
                "min": "5000000000",
                "address_percent": [
                    {
                        "recipient": {
                            "address": "andr1new_recipient...",
                            "msg": null
                        },
                        "percent": "0.25"
                    }
                ]
            }
        ]
    }
}
```

### Set Lock Time (Owner Only)
```json
{
    "update_lock": {
        "lock_time": {
            "at_time": "1672617600000000000"
        }
    }
}
```

### Query Configuration
```json
{
    "get_conditional_splitter_config": {}
}
```

## Integration Patterns

### With App Contract
The Conditional Splitter can be integrated into App contracts for automated distribution:

```json
{
    "components": [
        {
            "name": "sales_commission_splitter",
            "ado_type": "conditional-splitter",
            "component_type": {
                "new": {
                    "thresholds": [
                        {
                            "min": "100000000000",
                            "address_percent": [
                                {
                                    "recipient": {
                                        "address": "andr1top_sales...",
                                        "msg": null
                                    },
                                    "percent": "0.15"
                                }
                            ]
                        }
                    ],
                    "lock_time": {
                        "at_time": "1672617600000000000"
                    }
                }
            }
        }
    ]
}
```

### Sales Commission System
For e-commerce and marketplace applications:

1. **Deploy conditional splitter** with tiered commission structure
2. **Configure thresholds** based on sales volume levels
3. **Integrate with payment processing** to automatically distribute commissions
4. **Update thresholds** as business requirements evolve

### Staking Reward Distribution
For DeFi staking applications:

1. **Set up threshold tiers** based on staking amounts
2. **Configure reward percentages** for different staker categories
3. **Automate reward distribution** through staking contract integration
4. **Adjust tiers** based on protocol requirements

### Progressive Fee System
For service platforms with usage-based pricing:

1. **Define fee thresholds** based on usage levels
2. **Configure service providers** for different fee tiers
3. **Implement automatic fee collection** and distribution
4. **Monitor and adjust** fee structures as needed

## Security Features

### **Lock-Time Protection**
- **Minimum lock period**: 1 day minimum to prevent frequent changes
- **Maximum lock period**: 1 year maximum to ensure eventual access
- **Owner-only updates**: Only contract owner can modify when unlocked
- **Automatic enforcement**: Contract automatically checks lock status

### **Validation Safeguards**
- **Threshold validation**: Prevents empty or invalid threshold configurations
- **Percentage limits**: Ensures total percentages don't exceed 100%
- **Recipient verification**: Validates all recipient addresses
- **Duplicate prevention**: Prevents duplicate thresholds and recipients

### **Fund Security**
- **Remainder protection**: Automatically returns unallocated funds to sender
- **Amount validation**: Ensures non-zero amounts and validates fund limits
- **Multi-coin support**: Handles up to 5 different coin denominations safely
- **Error handling**: Comprehensive error handling for all edge cases

### **Access Controls**
- **Owner restrictions**: Administrative functions restricted to contract owner
- **Lock enforcement**: Threshold updates blocked during lock periods
- **State validation**: Maintains consistent internal state
- **Input sanitization**: Validates all user inputs thoroughly

## Validation Rules

### **Threshold Validation**
- Must have at least one threshold
- Each threshold must have at least one recipient
- Maximum 100 recipients per threshold
- No duplicate minimum values between thresholds
- All minimum values must be valid Uint128

### **Percentage Validation**
- Each threshold's total percentages cannot exceed 100%
- Individual percentages must be between 0 and 1
- All percentages must be valid Decimal values
- Precision maintained through decimal arithmetic

### **Recipient Validation**
- All recipient addresses must be valid
- No duplicate recipients within the same threshold
- Recipient addresses must be properly formatted
- Optional messages must be valid JSON if provided

## Important Notes

- **Lock time enforcement**: Threshold updates are only possible when contract is unlocked
- **Automatic threshold selection**: Contract automatically selects highest applicable threshold
- **Remainder handling**: Any unallocated funds are automatically returned to sender
- **Multi-coin support**: Supports up to 5 different coin denominations per transaction
- **Owner requirements**: Only contract owner can update thresholds and lock settings
- **Validation constraints**: All configurations must pass comprehensive validation
- **Time restrictions**: Lock times must be between 1 day and 1 year
- **Percentage limits**: Total allocation per threshold cannot exceed 100%

## Common Workflow

### 1. **Deploy with Initial Thresholds**
```json
{
    "thresholds": [
        {
            "min": "1000000",
            "address_percent": [
                {
                    "recipient": {
                        "address": "andr1recipient...",
                        "msg": null
                    },
                    "percent": "0.3"
                }
            ]
        }
    ],
    "lock_time": {
        "at_time": "1672617600000000000"
    }
}
```

### 2. **Send Funds for Distribution**
```json
{
    "send": {}
}
```
_Send native tokens as funds with the transaction._

### 3. **Update Configuration (When Unlocked)**
```json
{
    "update_thresholds": {
        "thresholds": [
            {
                "min": "2000000",
                "address_percent": [
                    {
                        "recipient": {
                            "address": "andr1new_recipient...",
                            "msg": null
                        },
                        "percent": "0.4"
                    }
                ]
            }
        ]
    }
}
```

### 4. **Query Current Configuration**
```json
{
    "get_conditional_splitter_config": {}
}
```

The Conditional Splitter ADO provides sophisticated automated distribution logic with threshold-based conditions, enabling complex business rules and incentive structures while maintaining security through lock-time protection and comprehensive validation.