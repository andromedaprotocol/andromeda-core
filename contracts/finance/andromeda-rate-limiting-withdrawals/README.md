# Andromeda Rate Limiting Withdrawals ADO

## Introduction

The Andromeda Rate Limiting Withdrawals ADO is a secure treasury and fund management contract that enforces withdrawal rate limits and time-based restrictions on fund access. This ADO provides essential security controls for high-value treasuries, organizational funds, and time-sensitive financial operations by implementing configurable withdrawal limits, minimum withdrawal intervals, and comprehensive account tracking for responsible fund management.

<b>Ado_type:</b> rate-limiting-withdrawals

## Why Rate Limiting Withdrawals ADO

The Rate Limiting Withdrawals ADO serves as a critical security layer for applications requiring:

- **Treasury Security**: Protect organizational treasuries from excessive or rapid fund drainage
- **Emergency Fund Management**: Implement controlled access to emergency reserves and backup funds
- **Investment Protection**: Secure investment funds with time-based withdrawal restrictions
- **Organizational Risk Management**: Prevent insider threats and unauthorized bulk withdrawals
- **Compliance Requirements**: Meet regulatory requirements for fund access controls
- **Automated Security Controls**: Implement security without manual oversight or intervention
- **Multi-User Fund Management**: Track individual account balances within shared fund pools
- **Time-Based Access Control**: Enforce cooling-off periods between withdrawal requests
- **Withdrawal Limit Enforcement**: Prevent single large withdrawals that could destabilize operations
- **Audit Trail Management**: Maintain detailed records of all deposits and withdrawals

The ADO provides comprehensive rate limiting with single-coin support, time-based restrictions, and automatic enforcement for secure and controlled fund management.

## Key Features

### **Rate Limiting Controls**
- **Withdrawal limits**: Configurable maximum withdrawal amounts per transaction
- **Time restrictions**: Minimum time intervals required between withdrawals
- **Single coin support**: Focus on one specific token denomination for security
- **Automatic enforcement**: Contract automatically validates and enforces all limits

### **Account Management**
- **Individual accounts**: Track separate balances for each user
- **Deposit tracking**: Monitor all deposits with automatic balance updates
- **Withdrawal history**: Track timestamp of latest withdrawal for each account
- **Balance verification**: Ensure sufficient funds before allowing withdrawals

### **Security Features**
- **Time-based locks**: Prevent rapid successive withdrawals
- **Amount validation**: Enforce maximum withdrawal limits
- **Balance protection**: Prevent overdrafts and insufficient fund withdrawals
- **Access control**: Only account holders can withdraw their own funds

### **Flexible Recipients**
- **Self-withdrawal**: Default withdrawal to sender's address
- **Third-party recipients**: Optionally specify different withdrawal recipients
- **AMP integration**: Support for Andromeda Messaging Protocol recipients
- **Automatic routing**: Seamless fund transfer to specified recipients

## Time-Based Security

### **Withdrawal Frequency Control**
The contract enforces minimum time intervals between withdrawals to prevent:
- **Rapid fund drainage**: Protect against quick successive withdrawals
- **Emergency exploitation**: Prevent panic-driven bulk fund removal
- **Automated attacks**: Mitigate programmatic fund extraction attempts
- **Operational instability**: Maintain steady fund availability for operations

### **Cooling-Off Periods**
Configurable minimum frequencies allow for various security models:
- **Daily limits**: 24-hour cooling periods for regular operations
- **Weekly restrictions**: 7-day intervals for high-security applications
- **Custom intervals**: Flexible timing based on specific security requirements
- **Emergency overrides**: Optional administrative controls for critical situations

## InstantiateMsg

```rust
pub struct InstantiateMsg {
    pub allowed_coin: CoinAndLimit,
    pub minimal_withdrawal_frequency: MinimumFrequency,
}

pub struct CoinAndLimit {
    pub coin: String,
    pub limit: Uint128,
}

pub enum MinimumFrequency {
    Time { time: MillisecondsDuration },
}
```

```json
{
    "allowed_coin": {
        "coin": "uandr",
        "limit": "1000000000"
    },
    "minimal_withdrawal_frequency": {
        "time": {
            "milliseconds": 86400000
        }
    }
}
```

**Parameters**:
- **allowed_coin**: Configuration for the single supported token
  - **coin**: Token denomination (e.g., "uandr", "uusd")
  - **limit**: Maximum withdrawal amount per transaction
- **minimal_withdrawal_frequency**: Time restrictions between withdrawals
  - **time**: Minimum interval in milliseconds between withdrawals

**Validation**:
- Coin denomination cannot be empty
- Withdrawal limit must be greater than zero
- Time frequency must be greater than zero
- Only supports one token denomination per contract

## ExecuteMsg

### Deposit
Deposits funds into the contract for the specified recipient.

```rust
Deposit {
    recipient: Option<String>,
}
```

```json
{
    "deposit": {
        "recipient": "andr1account_holder..."
    }
}
```

**Usage**: Send tokens as funds with this message. The contract will:
1. Validate the token denomination matches the allowed coin
2. Create or update the recipient's account balance
3. Track the deposit for future withdrawal eligibility

**Parameters**:
- **recipient**: Optional address to credit the deposit (defaults to message sender)

**Requirements**:
- Must send exactly one coin denomination with the message
- Coin denomination must match the contract's allowed coin
- Amount must be greater than zero

### Withdraw
Withdraws funds from the sender's account with rate limiting enforcement.

```rust
Withdraw {
    amount: Uint128,
    recipient: Option<Recipient>,
}
```

```json
{
    "withdraw": {
        "amount": "500000000",
        "recipient": {
            "address": "andr1withdrawal_recipient...",
            "msg": null
        }
    }
}
```

**Parameters**:
- **amount**: Amount to withdraw from account
- **recipient**: Optional recipient for the withdrawn funds (defaults to sender)

**Validation**:
- Sender must have an existing account with sufficient balance
- Amount cannot exceed the account balance
- Amount cannot exceed the configured withdrawal limit
- Must respect minimum time interval since last withdrawal
- Previous withdrawal timestamp must be outside the cooling-off period

**Requirements**:
- Account must exist and have sufficient balance
- Withdrawal amount must not exceed per-transaction limit
- Must wait minimum frequency time since last withdrawal
- No payable funds should be sent with this message

## QueryMsg

### CoinAllowanceDetails
Returns the allowed coin configuration and withdrawal limits.

```rust
pub enum QueryMsg {
    #[returns(CoinAllowance)]
    CoinAllowanceDetails {},
}
```

```json
{
    "coin_allowance_details": {}
}
```

**Response:**
```json
{
    "coin": "uandr",
    "limit": "1000000000",
    "minimal_withdrawal_frequency": {
        "milliseconds": 86400000
    }
}
```

### AccountDetails
Returns account information for a specific address.

```rust
pub enum QueryMsg {
    #[returns(AccountDetails)]
    AccountDetails { account: String },
}
```

```json
{
    "account_details": {
        "account": "andr1account_holder..."
    }
}
```

**Response:**
```json
{
    "balance": "5000000000",
    "latest_withdrawal": "1672617600"
}
```

**Response Fields**:
- **balance**: Current account balance in the allowed coin
- **latest_withdrawal**: Timestamp of the last withdrawal (null if never withdrawn)

## Usage Examples

### Treasury Security Setup
```json
{
    "allowed_coin": {
        "coin": "uandr",
        "limit": "10000000000"
    },
    "minimal_withdrawal_frequency": {
        "time": {
            "milliseconds": 604800000
        }
    }
}
```
_Weekly withdrawal limit of 10,000 ANDR tokens maximum per transaction._

### Emergency Fund Configuration
```json
{
    "allowed_coin": {
        "coin": "uusd",
        "limit": "1000000000"
    },
    "minimal_withdrawal_frequency": {
        "time": {
            "milliseconds": 259200000
        }
    }
}
```
_3-day cooling period with 1,000 USD maximum per withdrawal._

### Investment Fund Protection
```json
{
    "allowed_coin": {
        "coin": "ustake",
        "limit": "500000000"
    },
    "minimal_withdrawal_frequency": {
        "time": {
            "milliseconds": 2592000000
        }
    }
}
```
_Monthly withdrawal restrictions with 500 token limit._

### Organizational Payroll Fund
```json
{
    "allowed_coin": {
        "coin": "uandr",
        "limit": "50000000000"
    },
    "minimal_withdrawal_frequency": {
        "time": {
            "milliseconds": 86400000
        }
    }
}
```
_Daily withdrawal capability for payroll operations._

## Operational Examples

### Deposit Funds
```json
{
    "deposit": {
        "recipient": "andr1employee..."
    }
}
```
_Send tokens as funds to credit the recipient's account._

### Withdraw to Self
```json
{
    "withdraw": {
        "amount": "1000000000",
        "recipient": null
    }
}
```
_Withdraw 1,000 tokens to sender's address._

### Withdraw to Third Party
```json
{
    "withdraw": {
        "amount": "500000000",
        "recipient": {
            "address": "andr1beneficiary...",
            "msg": null
        }
    }
}
```
_Withdraw 500 tokens to a different recipient._

### Check Account Status
```json
{
    "account_details": {
        "account": "andr1user..."
    }
}
```

### Check Contract Configuration
```json
{
    "coin_allowance_details": {}
}
```

## Integration Patterns

### With App Contract
The Rate Limiting Withdrawals can be integrated into App contracts for treasury management:

```json
{
    "components": [
        {
            "name": "secure_treasury",
            "ado_type": "rate-limiting-withdrawals",
            "component_type": {
                "new": {
                    "allowed_coin": {
                        "coin": "uandr",
                        "limit": "10000000000"
                    },
                    "minimal_withdrawal_frequency": {
                        "time": {
                            "milliseconds": 604800000
                        }
                    }
                }
            }
        }
    ]
}
```

### Treasury Management
For organizational fund security:

1. **Deploy with security parameters** based on organization risk tolerance
2. **Deposit operational funds** into individual accounts
3. **Monitor withdrawal patterns** and account activity
4. **Adjust limits** as organizational needs evolve

### Emergency Fund System
For crisis management and emergency reserves:

1. **Configure restrictive withdrawal limits** to preserve emergency funds
2. **Set up authorized personnel accounts** with appropriate access levels
3. **Implement approval workflows** for emergency fund access
4. **Maintain audit trails** for compliance and reporting

### Investment Protection
For protecting long-term investment funds:

1. **Set monthly or quarterly withdrawal periods** to prevent impulsive decisions
2. **Configure appropriate withdrawal limits** based on investment strategy
3. **Monitor fund performance** and withdrawal patterns
4. **Adjust restrictions** based on market conditions and strategy changes

## Security Features

### **Time-Based Protection**
- **Cooling-off periods**: Enforced minimum intervals between withdrawals
- **Withdrawal frequency limits**: Prevent rapid successive fund extraction
- **Timestamp tracking**: Accurate withdrawal time monitoring
- **Automatic enforcement**: Contract-level enforcement without manual intervention

### **Amount Restrictions**
- **Per-transaction limits**: Maximum amount per individual withdrawal
- **Balance verification**: Prevent overdrafts and insufficient fund attempts
- **Limit validation**: Automatic validation against configured limits
- **Zero-amount prevention**: Reject zero or negative withdrawal attempts

### **Account Security**
- **Individual isolation**: Separate account balances prevent cross-contamination
- **Ownership verification**: Only account holders can withdraw their funds
- **Balance tracking**: Accurate real-time balance maintenance
- **Access control**: Comprehensive access validation for all operations

### **Fund Protection**
- **Single coin focus**: Simplified security model with single token support
- **Denomination validation**: Strict validation of token types
- **Amount validation**: Comprehensive amount and balance checking
- **Error handling**: Graceful handling of invalid requests and edge cases

## Rate Limiting Models

### **Conservative Model**
- **Weekly withdrawals**: 7-day minimum intervals
- **Low limits**: 1-5% of total fund per transaction
- **Use case**: Long-term investment funds, pension funds

### **Moderate Model**
- **Daily withdrawals**: 24-hour minimum intervals
- **Medium limits**: 5-15% of fund per transaction
- **Use case**: Operational treasuries, business expenses

### **Flexible Model**
- **Hourly withdrawals**: 1-6 hour minimum intervals
- **Higher limits**: 15-25% of fund per transaction
- **Use case**: Active trading funds, liquidity management

### **Emergency Model**
- **Restrictive access**: Multi-day cooling periods
- **Minimal limits**: <1% of fund per transaction
- **Use case**: Emergency reserves, crisis funds

## Important Notes

- **Single coin support**: Contract only supports one token denomination
- **Time enforcement**: Withdrawal frequency is strictly enforced by contract
- **Account creation**: Accounts are automatically created on first deposit
- **Balance tracking**: Real-time balance updates with every transaction
- **Withdrawal history**: Latest withdrawal timestamp tracked per account
- **No modifications**: Withdrawal limits and frequency cannot be changed after deployment
- **Zero validation**: All amounts and time intervals must be greater than zero
- **Error handling**: Comprehensive validation prevents invalid operations

## Common Workflow

### 1. **Deploy with Security Parameters**
```json
{
    "allowed_coin": {
        "coin": "uandr",
        "limit": "5000000000"
    },
    "minimal_withdrawal_frequency": {
        "time": {
            "milliseconds": 86400000
        }
    }
}
```

### 2. **Deposit Funds**
```json
{
    "deposit": {
        "recipient": "andr1user..."
    }
}
```
_Send tokens as funds with the transaction._

### 3. **Check Account Status**
```json
{
    "account_details": {
        "account": "andr1user..."
    }
}
```

### 4. **Withdraw Funds (After Cooling Period)**
```json
{
    "withdraw": {
        "amount": "1000000000",
        "recipient": null
    }
}
```

### 5. **Monitor Contract Limits**
```json
{
    "coin_allowance_details": {}
}
```

The Rate Limiting Withdrawals ADO provides essential security controls for fund management, enabling organizations to protect their treasuries while maintaining necessary operational flexibility through configurable withdrawal limits and time-based restrictions.