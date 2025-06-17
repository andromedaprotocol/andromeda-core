# Andromeda Fixed Amount Splitter ADO

## Introduction

The Andromeda Fixed Amount Splitter ADO is a financial distribution contract that automatically splits funds to specified recipients based on predetermined fixed amounts rather than percentages. This contract enables precise fund allocation where each recipient receives an exact amount of tokens, with any remainder being handled according to configurable rules. The splitter supports multiple token types, time-based locking, and dynamic configuration updates, making it ideal for precise payment distributions, salary payments, and fixed allocation scenarios.

<b>Ado_type:</b> fixed-amount-splitter

## Why Fixed Amount Splitter ADO

The Fixed Amount Splitter ADO serves as precise financial infrastructure for applications requiring:

- **Fixed Payment Systems**: Distribute exact amounts to employees, contractors, or partners
- **Salary and Payroll**: Automated payroll systems with fixed amounts per recipient
- **Commission Structures**: Pay fixed commissions regardless of total revenue
- **Multi-Recipient Payments**: Split payments to multiple parties with exact amounts
- **Allowance Distribution**: Distribute fixed allowances to multiple accounts
- **Budget Allocation**: Allocate exact budget amounts to different departments or projects
- **Revenue Sharing**: Fixed revenue sharing with predetermined amounts
- **Vendor Payments**: Automated payments to vendors with fixed amounts
- **Subscription Services**: Distribute subscription revenues with fixed allocations
- **Investment Distributions**: Fixed return distributions to investors

The ADO provides precise amount control, remainder handling, and lock-time security for reliable fixed distribution operations.

## Key Features

### **Fixed Amount Distribution**
- **Exact amounts**: Each recipient receives a predetermined fixed amount
- **Multi-token support**: Distribute multiple token types in a single operation
- **Remainder handling**: Configurable handling of surplus funds after fixed distributions
- **Atomic operations**: All distributions occur atomically or fail completely
- **Precise calculations**: No rounding errors or percentage-based approximations

### **Flexible Recipient Management**
- **Up to 100 recipients**: Support for large distribution lists
- **Multi-coin recipients**: Each recipient can receive up to 2 different token types
- **Dynamic updates**: Update recipient lists when contract is unlocked
- **Address validation**: Comprehensive validation of all recipient addresses
- **Duplicate prevention**: Prevent duplicate recipients and coin denominations

### **Security and Lock Management**
- **Time-based locking**: Lock contract configuration for security periods
- **Lock duration limits**: Lock periods between 1 day and 1 year
- **Configuration protection**: Prevent unauthorized configuration changes during lock
- **Owner controls**: Only contract owner can modify configuration
- **Emergency unlocking**: Automatic unlocking after lock period expires

### **Advanced Fund Handling**
- **Native token support**: Full support for native blockchain tokens
- **CW20 token support**: Handle CW20 tokens through receive hooks
- **Default recipient**: Configure default recipient for remainder funds
- **AMP integration**: Full integration with Andromeda Messaging Protocol
- **Batch operations**: Efficient handling of multiple distributions

## Distribution Logic

### **Fixed Amount Allocation**
The contract distributes funds based on fixed amounts:
1. **Check recipient allocations** for each sent token denomination
2. **Subtract fixed amounts** from total for each matching recipient
3. **Verify sufficient funds** to cover all fixed allocations
4. **Send fixed amounts** to respective recipients
5. **Handle remainder** according to default recipient configuration

### **Remainder Handling**
Any funds remaining after fixed distributions are handled as follows:
- **Default recipient**: Send to configured default recipient
- **Fallback to sender**: If no default recipient, return to original sender
- **Multi-token support**: Handle remainders for each token type separately

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
                    "amount": "5000000000"
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
                    "amount": "3000000000"
                },
                {
                    "denom": "uusdc",
                    "amount": "1000000000"
                }
            ]
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
- **recipients**: List of recipients with their fixed amounts
  - **recipient**: Recipient address and optional message
  - **coins**: Fixed amounts for up to 2 different token types per recipient
- **lock_time**: Optional lock expiration time (between 1 day and 1 year)
- **default_recipient**: Optional default recipient for remainder funds

**Validation**:
- Must have at least 1 recipient, maximum 100 recipients
- Each recipient can receive 1-2 different token types
- No duplicate recipients or coin denominations per recipient
- All amounts must be non-zero
- All addresses must be valid

## ExecuteMsg

### Send
Distributes attached funds according to the fixed amount configuration.

```rust
Send {
    config: Option<Vec<AddressAmount>>,
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
1. Check that sent funds cover all fixed allocations
2. Distribute fixed amounts to each recipient
3. Send any remainder to the default recipient or sender

**Parameters**:
- **config**: Optional override configuration (only works when contract is unlocked)

**Requirements**:
- Must send 1-2 native token denominations as funds
- Sent amounts must be sufficient to cover all fixed allocations
- Cannot send zero amounts or duplicate denominations

### Receive (CW20 Hook)
Handles CW20 token distribution through the standard CW20 receive mechanism.

```rust
Receive(Cw20ReceiveMsg)

pub enum Cw20HookMsg {
    Send { config: Option<Vec<AddressAmount>> },
    AmpReceive(AMPPkt),
}
```

**Usage**: CW20 tokens automatically call this when sent to the contract with a hook message.

### UpdateRecipients
Updates the recipient list (owner-only, when unlocked).

```rust
UpdateRecipients {
    recipients: Vec<AddressAmount>,
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
                        "amount": "4000000000"
                    }
                ]
            }
        ]
    }
}
```

**Requirements**:
- Only contract owner can execute
- Contract must not be locked
- New recipient list must pass validation

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
- **lock_time**: New lock expiration time (between 1 day and 1 year from current time)

**Requirements**:
- Only contract owner can execute
- Contract must not be currently locked
- New lock time must be within valid range

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
    pub recipients: Vec<AddressAmount>,
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
                    "address": "andr1employee1...",
                    "msg": null
                },
                "coins": [
                    {
                        "denom": "uandr",
                        "amount": "5000000000"
                    }
                ]
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

## Usage Examples

### Employee Payroll System
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
                    "amount": "50000000000"
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
                    "amount": "40000000000"
                }
            ]
        },
        {
            "recipient": {
                "address": "andr1employee3...",
                "msg": null
            },
            "coins": [
                {
                    "denom": "uandr",
                    "amount": "35000000000"
                }
            ]
        }
    ],
    "lock_time": {
        "time": 2629746
    },
    "default_recipient": {
        "address": "andr1company_treasury...",
        "msg": null
    }
}
```

### Multi-Token Commission System
```json
{
    "recipients": [
        {
            "recipient": {
                "address": "andr1sales_manager...",
                "msg": null
            },
            "coins": [
                {
                    "denom": "uandr",
                    "amount": "10000000000"
                },
                {
                    "denom": "uusdc",
                    "amount": "5000000000"
                }
            ]
        },
        {
            "recipient": {
                "address": "andr1support_team...",
                "msg": null
            },
            "coins": [
                {
                    "denom": "uandr",
                    "amount": "5000000000"
                }
            ]
        }
    ],
    "lock_time": null,
    "default_recipient": null
}
```

### Fixed Allowance Distribution
```json
{
    "recipients": [
        {
            "recipient": {
                "address": "andr1development_team...",
                "msg": null
            },
            "coins": [
                {
                    "denom": "uandr",
                    "amount": "20000000000"
                }
            ]
        },
        {
            "recipient": {
                "address": "andr1marketing_team...",
                "msg": null
            },
            "coins": [
                {
                    "denom": "uandr",
                    "amount": "15000000000"
                }
            ]
        },
        {
            "recipient": {
                "address": "andr1operations_team...",
                "msg": null
            },
            "coins": [
                {
                    "denom": "uandr",
                    "amount": "10000000000"
                }
            ]
        }
    ],
    "lock_time": {
        "at_time": "1704067200000000000"
    },
    "default_recipient": {
        "address": "andr1reserve_fund...",
        "msg": null
    }
}
```

## Operational Examples

### Execute Payment Distribution
```json
{
    "send": {
        "config": null
    }
}
```
_Send 150,000,000 uandr as funds to distribute 125,000,000 to recipients and 25,000,000 remainder._

### Override Configuration (When Unlocked)
```json
{
    "send": {
        "config": [
            {
                "recipient": {
                    "address": "andr1temporary_recipient...",
                    "msg": null
                },
                "coins": [
                    {
                        "denom": "uandr",
                        "amount": "30000000000"
                    }
                ]
            }
        ]
    }
}
```

### Update Recipients
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
                        "amount": "45000000000"
                    }
                ]
            }
        ]
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

### Query Configuration
```json
{
    "get_splitter_config": {}
}
```

## Integration Patterns

### With App Contract
Fixed amount splitter can be integrated for payment systems:

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
                            "recipient": {
                                "address": "andr1employee1...",
                                "msg": null
                            },
                            "coins": [
                                {
                                    "denom": "uandr",
                                    "amount": "50000000000"
                                }
                            ]
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

### Payroll Management
For automated payroll systems:

1. **Deploy splitter** with employee addresses and fixed salaries
2. **Lock configuration** for security during pay periods
3. **Execute payments** by sending total payroll amount
4. **Update recipients** when hiring/firing employees (when unlocked)
5. **Handle surplus** through default recipient configuration

### Commission Distribution
For fixed commission payments:

1. **Set fixed commission amounts** for each recipient role
2. **Configure multi-token support** for different payment types
3. **Execute distributions** based on sales or performance triggers
4. **Update amounts** based on role changes or promotions
5. **Track remainder funds** for company treasury

### Budget Allocation
For departmental budget distribution:

1. **Define department allocations** with fixed amounts
2. **Set quarterly lock periods** for budget stability
3. **Distribute budget tranches** at regular intervals
4. **Redirect surplus** to reserve funds or special projects
5. **Adjust allocations** between budget periods

### Vendor Payment Systems
For automated vendor payments:

1. **Configure vendor payments** with fixed service fees
2. **Set up payment schedules** through external triggering systems
3. **Handle multi-currency payments** through different token types
4. **Manage payment updates** when contracts change
5. **Track payment completion** through transaction logs

## Advanced Features

### **Multi-Token Distribution**
- **Dual-token recipients**: Each recipient can receive up to 2 different tokens
- **Cross-token operations**: Handle multiple token types in single transaction
- **Token-specific remainders**: Handle surplus funds for each token type separately
- **Native and CW20 support**: Seamless handling of both native and CW20 tokens

### **Configuration Security**
- **Time-based locking**: Prevent configuration changes during lock periods
- **Graduated lock times**: Support for various lock durations based on use cases
- **Emergency unlocking**: Automatic unlocking after expiration
- **Owner-only controls**: Restrict configuration changes to contract owner

### **Remainder Management**
- **Configurable handling**: Set default recipient for surplus funds
- **Fallback to sender**: Return surplus to original sender if no default set
- **Multi-token remainders**: Handle surplus for each token type independently
- **Precision handling**: Ensure exact distributions without rounding errors

### **Validation and Safety**
- **Comprehensive validation**: Validate all recipients, amounts, and addresses
- **Duplicate prevention**: Prevent duplicate recipients and coin denominations
- **Amount verification**: Ensure sufficient funds before distribution
- **Atomic operations**: All-or-nothing distribution execution

## Security Features

### **Access Control**
- **Owner restrictions**: Only contract owner can modify configuration
- **Lock enforcement**: Prevent unauthorized changes during lock periods
- **Address validation**: Comprehensive validation of all addresses
- **Permission verification**: Verify permissions before configuration changes

### **Fund Protection**
- **Atomic distributions**: All distributions occur atomically or fail completely
- **Balance verification**: Verify sufficient funds before distribution
- **Overflow protection**: Prevent arithmetic overflow in calculations
- **Remainder safety**: Safe handling of surplus funds

### **Configuration Protection**
- **Lock validation**: Ensure lock times are within valid ranges
- **Recipient limits**: Limit number of recipients to prevent gas issues
- **Amount validation**: Ensure all amounts are valid and non-zero
- **State consistency**: Maintain consistent state across all operations

### **Transaction Safety**
- **Gas optimization**: Efficient distribution algorithms
- **Error handling**: Comprehensive error handling and recovery
- **State preservation**: Maintain contract state integrity
- **Failed transaction recovery**: Proper handling of failed distributions

## Important Notes

- **Fixed amounts**: Recipients receive exact amounts, not percentages
- **Remainder handling**: Surplus funds go to default recipient or sender
- **Lock restrictions**: Cannot modify configuration while contract is locked
- **Multi-token support**: Support for native tokens and CW20 tokens
- **Recipient limits**: Maximum 100 recipients per configuration
- **Coin limits**: Each recipient can receive 1-2 different token types
- **Owner privileges**: Only contract owner can modify configuration
- **Atomic operations**: All distributions succeed or fail together

## Common Workflow

### 1. **Deploy Splitter**
```json
{
    "recipients": [
        {
            "recipient": {
                "address": "andr1employee...",
                "msg": null
            },
            "coins": [
                {
                    "denom": "uandr",
                    "amount": "50000000000"
                }
            ]
        }
    ],
    "lock_time": {
        "time": 2629746
    },
    "default_recipient": null
}
```

### 2. **Execute Payment**
```json
{
    "send": {
        "config": null
    }
}
```
_Send funds as transaction fees._

### 3. **Query Configuration**
```json
{
    "get_splitter_config": {}
}
```

### 4. **Update Recipients (When Unlocked)**
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
                        "amount": "45000000000"
                    }
                ]
            }
        ]
    }
}
```

### 5. **Update Lock Period**
```json
{
    "update_lock": {
        "lock_time": {
            "time": 5259492
        }
    }
}
```

### 6. **Set Default Recipient**
```json
{
    "update_default_recipient": {
        "recipient": {
            "address": "andr1treasury...",
            "msg": null
        }
    }
}
```

The Fixed Amount Splitter ADO provides precise fund distribution infrastructure for the Andromeda ecosystem, enabling accurate, secure, and flexible payment systems with fixed allocation amounts and comprehensive security controls.