# Andromeda Fixed Amount Splitter ADO

## Introduction

The Andromeda Fixed Amount Splitter ADO is a precise fund distribution contract that distributes specific fixed amounts of tokens to predefined recipients. Unlike percentage-based splitters, this contract sends exact token amounts to each recipient, making it ideal for salary payments, fixed distributions, subscription services, and any scenario requiring precise token allocations with predictable amounts.

<b>Ado_type:</b> fixed-amount-splitter

## Why Fixed Amount Splitter ADO

The Fixed Amount Splitter ADO serves as a powerful financial tool for applications requiring:

- **Payroll Systems**: Distribute fixed salaries or payments to employees
- **Subscription Services**: Send fixed amounts to service providers or partners
- **Fixed Dividends**: Distribute specific dividend amounts to token holders
- **Supplier Payments**: Pay fixed amounts to multiple suppliers automatically
- **Multi-Party Agreements**: Execute contracts with predetermined payment amounts
- **Batch Payments**: Send different fixed amounts to multiple recipients efficiently
- **Allowance Distribution**: Distribute specific allowances or stipends
- **Fixed Rewards**: Distribute predetermined reward amounts to participants
- **Invoice Processing**: Automate payment of fixed invoice amounts
- **Expense Reimbursement**: Distribute specific expense amounts to multiple parties

The ADO supports both native and CW20 tokens, handles multiple coin types per recipient, includes a locking mechanism for security, and provides surplus handling through default recipients.

## Key Features

### **Fixed Amount Distribution**
- Each recipient receives a **specific fixed amount** (not percentages)
- Supports **1-2 different coin types** per recipient
- **Precise control** over distribution amounts
- **No rounding errors** from percentage calculations

### **Multi-Token Support**
- **Native tokens**: Cosmos native tokens (e.g., ATOM, OSMO)
- **CW20 tokens**: All standard CW20 tokens
- **Multi-coin recipients**: Each recipient can receive up to 2 different token types
- **Batch processing**: Handle multiple token types in single transaction

### **Lock Mechanism**
- **Time-based locking**: Lock configuration for security
- **Minimum lock time**: 1 day (86,400 seconds)
- **Maximum lock time**: 1 year (31,536,000 seconds)
- **Unlock flexibility**: Update lock duration when unlocked

### **Surplus Handling**
- **Default recipient**: Configurable recipient for surplus funds
- **Automatic refund**: Surplus goes to sender if no default recipient
- **No loss of funds**: All sent funds are distributed or returned

## InstantiateMsg

```rust
pub struct InstantiateMsg {
    pub recipients: Vec<AddressAmount>,
    pub lock_time: Option<Expiry>,
    pub default_recipient: Option<Recipient>,
}

pub struct AddressAmount {
    pub recipient: Recipient,
    pub coins: Vec<Coin>,
}
```

```json
{
    "recipients": [
        {
            "recipient": {
                "address": "andr1employee1...",
                "msg": null
            },
            "coins": [
                {
                    "denom": "uandr",
                    "amount": "1000000"
                }
            ]
        },
        {
            "recipient": {
                "address": "andr1employee2...",
                "msg": null
            },
            "coins": [
                {
                    "denom": "uandr",
                    "amount": "1500000"
                },
                {
                    "denom": "usdc",
                    "amount": "500000"
                }
            ]
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

- **recipients**: List of recipients with their fixed amounts
  - **recipient**: Address and optional message for the recipient
  - **coins**: 1-2 specific coin amounts for this recipient
- **lock_time**: Optional lock expiration time for configuration security
- **default_recipient**: Optional recipient for surplus funds (defaults to sender)

## ExecuteMsg

### Send
Distributes the attached funds according to the recipient configuration.

```rust
Send { 
    config: Option<Vec<AddressAmount>> 
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
                "coins": [
                    {
                        "denom": "uandr",
                        "amount": "2000000"
                    }
                ]
            }
        ]
    }
}
```

_**Note:** If config is provided, the contract must be unlocked and the custom configuration will be validated._

### Receive (CW20 Support)
Handles CW20 token transfers through the standard CW20 receive mechanism.

```rust
Receive(Cw20ReceiveMsg)
```

**CW20 Send Message:**
```json
{
    "send": {
        "contract": "andr1splitter_contract...",
        "amount": "5000000",
        "msg": "eyJzZW5kIjp7ImNvbmZpZyI6bnVsbH19"
    }
}
```

### UpdateRecipients
Updates the recipient list with new fixed amounts.

_**Note:** Only contract owner can execute when unlocked._

```rust
UpdateRecipients { 
    recipients: Vec<AddressAmount> 
}
```

```json
{
    "update_recipients": {
        "recipients": [
            {
                "recipient": {
                    "address": "andr1new_employee...",
                    "msg": null
                },
                "coins": [
                    {
                        "denom": "uandr",
                        "amount": "1200000"
                    }
                ]
            }
        ]
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
Updates the default recipient for surplus funds.

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
Returns the current splitter configuration including recipients, lock status, and default recipient.

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
                    "address": "andr1employee1...",
                    "msg": null
                },
                "coins": [
                    {
                        "denom": "uandr",
                        "amount": "1000000"
                    }
                ]
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

## Validation Rules

### Recipient Validation
- **Minimum recipients**: At least 1 recipient required
- **Maximum recipients**: Maximum 100 recipients allowed
- **Unique addresses**: No duplicate recipient addresses
- **Valid addresses**: All recipient addresses must be valid
- **Non-zero amounts**: All coin amounts must be greater than zero

### Coin Validation
- **Coin count**: Each recipient can have 1-2 different coin types
- **No duplicates**: No duplicate denominations per recipient
- **Valid denominations**: All coin denominations must be valid
- **Positive amounts**: All amounts must be positive

### Lock Validation
- **Minimum duration**: Lock time must be at least 1 day (86,400 seconds)
- **Maximum duration**: Lock time cannot exceed 1 year (31,536,000 seconds)
- **Valid expiry**: Expiry time must be in the future

## Usage Examples

### Employee Payroll
```json
{
    "recipients": [
        {
            "recipient": {"address": "andr1developer1..."},
            "coins": [{"denom": "uandr", "amount": "5000000000"}]
        },
        {
            "recipient": {"address": "andr1designer1..."},
            "coins": [{"denom": "uandr", "amount": "4000000000"}]
        },
        {
            "recipient": {"address": "andr1manager1..."},
            "coins": [{"denom": "uandr", "amount": "6000000000"}]
        }
    ],
    "lock_time": {
        "at_time": "1672531200000000000"
    }
}
```

### Multi-Token Distribution
```json
{
    "recipients": [
        {
            "recipient": {"address": "andr1service_provider..."},
            "coins": [
                {"denom": "uandr", "amount": "1000000"},
                {"denom": "usdc", "amount": "500000"}
            ]
        },
        {
            "recipient": {"address": "andr1partner..."},
            "coins": [{"denom": "uatom", "amount": "2000000"}]
        }
    ]
}
```

### Subscription Payments
```json
{
    "recipients": [
        {
            "recipient": {"address": "andr1hosting_provider..."},
            "coins": [{"denom": "usdc", "amount": "10000000"}]
        },
        {
            "recipient": {"address": "andr1security_service..."},
            "coins": [{"denom": "usdc", "amount": "5000000"}]
        },
        {
            "recipient": {"address": "andr1cdn_provider..."},
            "coins": [{"denom": "usdc", "amount": "3000000"}]
        }
    ],
    "default_recipient": {
        "address": "andr1company_treasury..."
    }
}
```

### Fixed Dividend Distribution
```json
{
    "recipients": [
        {
            "recipient": {"address": "andr1shareholder1..."},
            "coins": [{"denom": "uandr", "amount": "100000000"}]
        },
        {
            "recipient": {"address": "andr1shareholder2..."},
            "coins": [{"denom": "uandr", "amount": "150000000"}]
        },
        {
            "recipient": {"address": "andr1shareholder3..."},
            "coins": [{"denom": "uandr", "amount": "75000000"}]
        }
    ]
}
```

## Integration Patterns

### With App Contract
The Fixed Amount Splitter can be integrated into App contracts for automated payments:

```json
{
    "components": [
        {
            "name": "payroll_splitter",
            "ado_type": "fixed-amount-splitter",
            "component_type": {
                "new": {
                    "recipients": [
                        {
                            "recipient": {"address": "andr1employee1..."},
                            "coins": [{"denom": "uandr", "amount": "5000000000"}]
                        }
                    ],
                    "lock_time": {
                        "at_time": "1672531200000000000"
                    }
                }
            }
        }
    ]
}
```

### Automated Payroll
For recurring salary payments:

1. **Configure recipients** with fixed salary amounts
2. **Set lock time** to prevent unauthorized changes
3. **Send funds** monthly/weekly for automatic distribution
4. **Update recipients** when employees join/leave (when unlocked)

### Subscription Management
For service payment automation:

1. **Set up service providers** with fixed monthly fees
2. **Configure default recipient** for unused funds
3. **Automate payments** through scheduled transactions
4. **Adjust amounts** as service costs change

### Multi-Party Settlements
For business agreement execution:

1. **Define fixed amounts** per agreement terms
2. **Lock configuration** during agreement period
3. **Execute settlements** by sending total amount
4. **Handle surplus** through default recipient

## Distribution Logic

### Fixed Amount Calculation
1. **Match denominations**: Find recipients for each sent coin type
2. **Deduct fixed amounts**: Subtract each recipient's fixed amount
3. **Validate sufficiency**: Ensure sufficient funds for all recipients
4. **Calculate surplus**: Remaining funds after all distributions
5. **Handle remainder**: Send surplus to default recipient or sender

### Example Distribution
**Sent**: 10,000,000 uandr
**Recipients**:
- Recipient A: 3,000,000 uandr
- Recipient B: 2,500,000 uandr  
- Recipient C: 1,500,000 uandr

**Result**:
- Recipient A receives: 3,000,000 uandr
- Recipient B receives: 2,500,000 uandr
- Recipient C receives: 1,500,000 uandr
- Default recipient receives: 3,000,000 uandr (surplus)

## Security Features

### Lock Mechanism
- **Configuration protection**: Prevents unauthorized changes when locked
- **Time-based security**: Lock automatically expires after specified time
- **Owner control**: Only owner can update lock settings
- **Flexible duration**: Configurable lock periods from 1 day to 1 year

### Validation Safeguards
- **Amount validation**: Prevents zero or negative amounts
- **Duplicate prevention**: No duplicate recipients or coin types
- **Overflow protection**: Checked arithmetic prevents overflow errors
- **Address validation**: Ensures all recipient addresses are valid

## Important Notes

- **Fixed amounts**: Recipients receive exact amounts, not percentages
- **Multi-token support**: Each recipient can receive up to 2 different tokens
- **Surplus handling**: Unused funds automatically go to default recipient or sender
- **Lock security**: Configuration can be locked to prevent unauthorized changes
- **Recipient limits**: Maximum 100 recipients to prevent gas issues
- **CW20 compatible**: Supports both native and CW20 token distributions
- **AMP integration**: Uses Andromeda Messaging Protocol for cross-contract communication

## Error Handling

### Common Errors
- **InsufficientFunds**: Sent amount less than total required for recipients
- **ContractLocked**: Attempting to modify configuration while locked
- **EmptyRecipientsList**: No recipients configured
- **ReachedRecipientLimit**: More than 100 recipients
- **DuplicateRecipient**: Duplicate recipient addresses
- **InvalidZeroAmount**: Zero amount in recipient configuration
- **LockTimeTooShort/Long**: Lock time outside valid range (1 day - 1 year)

The Fixed Amount Splitter ADO provides precise, secure, and flexible fund distribution capabilities, making it ideal for any application requiring exact token allocations with predictable amounts and robust security features.