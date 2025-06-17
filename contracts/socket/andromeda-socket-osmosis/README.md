# Andromeda Socket Osmosis ADO

## Introduction

The Andromeda Socket Osmosis ADO is a specialized integration module that provides seamless access to Osmosis DEX functionality within the Andromeda ecosystem. This contract enables automated token swapping, liquidity pool creation, and pool management operations while maintaining full integration with Andromeda's messaging protocol (AMP). The socket acts as a bridge between Andromeda applications and Osmosis's advanced DeFi features, enabling sophisticated trading strategies and liquidity management.

<b>Ado_type:</b> socket-osmosis

## Why Socket Osmosis ADO

The Socket Osmosis ADO serves as a critical DeFi infrastructure component for applications requiring:

- **Automated Token Swapping**: Execute token swaps with automatic recipient forwarding
- **Advanced DeFi Integration**: Access Osmosis's sophisticated DEX features from Andromeda apps
- **Liquidity Pool Management**: Create and manage various types of liquidity pools
- **Slippage Protection**: Implement TWAP-based and fixed slippage protection mechanisms
- **Route Optimization**: Leverage Osmosis's optimal routing for multi-hop swaps
- **Cross-Protocol Composability**: Seamlessly integrate DEX operations into complex workflows
- **Automated Strategy Execution**: Enable algorithmic trading and liquidity strategies
- **Portfolio Rebalancing**: Automate portfolio management through programmatic swaps
- **Yield Farming**: Access liquidity mining and yield farming opportunities
- **Risk Management**: Implement automated risk management through swap mechanisms

The ADO supports all Osmosis pool types including Balancer, Stable, Concentrated Liquidity, and CosmWasm pools.

## Key Features

### **Token Swapping**
- **Automated swaps**: Execute token swaps with automatic forwarding to recipients
- **Slippage protection**: TWAP-based and minimum output amount slippage controls
- **Custom routing**: Specify custom swap routes or use automatic optimal routing
- **AMP integration**: Full integration with Andromeda Messaging Protocol for complex workflows
- **Multi-hop support**: Execute complex multi-hop swaps through multiple pools

### **Pool Creation**
- **Multiple pool types**: Support for Balancer, Stable, Concentrated, and CosmWasm pools
- **Flexible parameters**: Customizable pool parameters for different trading strategies
- **Liquidity provision**: Automatic handling of initial liquidity provision
- **Pool governance**: Set up pool governance and control parameters
- **LP token management**: Automatic LP token distribution and tracking

### **Pool Management**
- **Liquidity withdrawal**: Withdraw liquidity from existing positions
- **Pool ID tracking**: Automatic tracking of created pools for management
- **Position monitoring**: Track and manage liquidity positions
- **Reward claiming**: Integration with Osmosis reward mechanisms
- **Pool updates**: Modify pool parameters where applicable

### **Advanced Trading**
- **Route discovery**: Query optimal routes for token pairs
- **Dynamic routing**: Support for both manual and automatic route selection
- **Slippage models**: Multiple slippage protection mechanisms
- **MEV protection**: Built-in protection against maximum extractable value attacks
- **Batch operations**: Execute multiple operations in single transactions

## Pool Types

### **Balancer Pools**
Weighted pools with customizable asset weights:
- **Custom weights**: Set different weights for pool assets
- **Flexible ratios**: Support for non-50/50 pools
- **Multiple assets**: Support for multi-asset pools
- **Dynamic weights**: Configurable weight adjustment mechanisms

### **Stable Pools**
Optimized for stable asset trading:
- **Low slippage**: Minimal slippage for stable asset swaps
- **Scaling factors**: Customizable scaling for different asset precisions
- **Stable asset focus**: Optimized for correlated asset trading
- **Capital efficiency**: High capital efficiency for stable swaps

### **Concentrated Liquidity Pools**
Advanced liquidity management with position control:
- **Range orders**: Provide liquidity in specific price ranges
- **Capital efficiency**: Maximum capital efficiency through concentrated positions
- **Tick spacing**: Customizable tick spacing for different strategies
- **Active management**: Enable active liquidity management strategies

### **CosmWasm Pools**
Custom pool implementations through CosmWasm:
- **Custom logic**: Implement custom pool logic through smart contracts
- **Advanced strategies**: Enable sophisticated trading strategies
- **Extensibility**: Full extensibility through contract upgrades
- **Innovation space**: Enable new pool types and mechanisms

## InstantiateMsg

```rust
pub struct InstantiateMsg {
    pub swap_router: Option<AndrAddr>,
}
```

```json
{
    "swap_router": "andr1osmosis_router_address..."
}
```

**Parameters**:
- **swap_router**: Optional Osmosis router contract address (defaults to `/lib/osmosis/router`)

**Default Configuration**:
If no swap router is provided, the contract uses the default Osmosis router at `/lib/osmosis/router`, which automatically resolves to the appropriate Osmosis router contract for the current network.

**Validation**:
- Swap router address must be valid and resolvable
- Router contract must implement the required Osmosis interface
- Address validation occurs during instantiation

## ExecuteMsg

### SwapAndForward
Executes a token swap and forwards the result to a specified recipient.

```rust
SwapAndForward {
    to_denom: String,
    recipient: Option<Recipient>,
    slippage: Slippage,
    route: Option<Vec<SwapAmountInRoute>>,
}

pub enum Slippage {
    Twap {
        window_seconds: Option<u64>,
        slippage_percentage: Decimal,
    },
    MinOutputAmount(Uint128),
}
```

```json
{
    "swap_and_forward": {
        "to_denom": "uosmo",
        "recipient": {
            "address": "andr1recipient_address...",
            "msg": null
        },
        "slippage": {
            "twap": {
                "window_seconds": 600,
                "slippage_percentage": "0.05"
            }
        },
        "route": [
            {
                "pool_id": "1",
                "token_out_denom": "uion"
            },
            {
                "pool_id": "2",
                "token_out_denom": "uosmo"
            }
        ]
    }
}
```

**Usage**: Send the token to swap as funds with this message. The contract will:
1. Execute the swap using the specified parameters
2. Forward the swapped tokens to the recipient
3. Handle any refunds if the swap is unsuccessful

**Parameters**:
- **to_denom**: Target token denomination to swap to
- **recipient**: Optional recipient for swapped tokens (defaults to sender)
- **slippage**: Slippage protection mechanism
- **route**: Optional custom swap route (uses automatic routing if not specified)

**Slippage Options**:
- **TWAP**: Use time-weighted average price with percentage slippage
- **MinOutputAmount**: Specify minimum output amount directly

**Requirements**:
- Must send exactly one coin denomination as funds
- Recipient address must be valid
- Route (if specified) must be valid for the token pair

### CreatePool
Creates a new liquidity pool on Osmosis.

```rust
CreatePool { pool_type: Pool }

pub enum Pool {
    Balancer {
        pool_params: Option<PoolParams>,
        pool_assets: Vec<PoolAsset>,
    },
    Stable {
        pool_params: Option<StablePoolParams>,
        scaling_factors: Vec<u64>,
    },
    Concentrated {
        tick_spacing: u64,
        spread_factor: String,
    },
    CosmWasm {
        code_id: u64,
        instantiate_msg: Vec<u8>,
    },
}
```

**Balancer Pool Example:**
```json
{
    "create_pool": {
        "pool_type": {
            "balancer": {
                "pool_params": {
                    "swap_fee": "0.003",
                    "exit_fee": "0.001"
                },
                "pool_assets": [
                    {
                        "token": {
                            "denom": "uatom",
                            "amount": "1000000000"
                        },
                        "weight": "50"
                    },
                    {
                        "token": {
                            "denom": "uosmo",
                            "amount": "1000000000"
                        },
                        "weight": "50"
                    }
                ]
            }
        }
    }
}
```

**Stable Pool Example:**
```json
{
    "create_pool": {
        "pool_type": {
            "stable": {
                "pool_params": {
                    "swap_fee": "0.001",
                    "exit_fee": "0.0"
                },
                "scaling_factors": [1, 1]
            }
        }
    }
}
```

**Concentrated Pool Example:**
```json
{
    "create_pool": {
        "pool_type": {
            "concentrated": {
                "tick_spacing": 100,
                "spread_factor": "0.002"
            }
        }
    }
}
```

**Requirements**:
- Must send exactly 2 tokens as funds for pool creation
- Pool parameters must be valid for the specified pool type
- Scaling factors must match the number of tokens for stable pools
- Tick spacing must be valid for concentrated pools

### WithdrawPool
Withdraws liquidity from a previously created pool.

```rust
WithdrawPool { withdraw_msg: MsgExitPool }
```

```json
{
    "withdraw_pool": {
        "withdraw_msg": {
            "sender": "andr1contract_address...",
            "pool_id": "123",
            "pool_shares_out": "1000000000",
            "token_out_mins": [
                {
                    "denom": "uatom",
                    "amount": "450000000"
                },
                {
                    "denom": "uosmo", 
                    "amount": "450000000"
                }
            ]
        }
    }
}
```

**Parameters**:
- **withdraw_msg**: Standard Osmosis pool exit message
- Pool ID must match a pool created by the sender
- Minimum output amounts provide slippage protection

**Authorization**: Only addresses that created pools can withdraw from them
**Validation**: Pool ID is validated against internal tracking

### UpdateSwapRouter
Updates the Osmosis router contract address.

```rust
UpdateSwapRouter { swap_router: AndrAddr }
```

```json
{
    "update_swap_router": {
        "swap_router": "andr1new_router_address..."
    }
}
```

**Authorization**: Only contract owner can update the swap router
**Validation**: New router address must be valid and accessible

## QueryMsg

### GetRoute
Queries the optimal route for swapping between two tokens.

```rust
#[returns(GetRouteResponse)]
GetRoute {
    from_denom: String,
    to_denom: String,
}

pub struct GetRouteResponse {
    pub pool_route: Vec<SwapAmountInRoute>,
}
```

```json
{
    "get_route": {
        "from_denom": "uatom",
        "to_denom": "uosmo"
    }
}
```

**Response:**
```json
{
    "pool_route": [
        {
            "pool_id": "1",
            "token_out_denom": "uion"
        },
        {
            "pool_id": "2", 
            "token_out_denom": "uosmo"
        }
    ]
}
```

**Usage**: This query returns the optimal routing path for a token swap, which can be used in the `SwapAndForward` message for custom routing control.

## Usage Examples

### Simple Token Swap
```json
{
    "swap_and_forward": {
        "to_denom": "uosmo",
        "recipient": null,
        "slippage": {
            "min_output_amount": "950000000"
        },
        "route": null
    }
}
```
_Send ATOM as funds to swap for OSMO with minimum output protection._

### Multi-Hop Swap with Custom Route
```json
{
    "swap_and_forward": {
        "to_denom": "ujuno",
        "recipient": {
            "address": "andr1destination_address...",
            "msg": null
        },
        "slippage": {
            "twap": {
                "window_seconds": 300,
                "slippage_percentage": "0.03"
            }
        },
        "route": [
            {
                "pool_id": "1",
                "token_out_denom": "uosmo"
            },
            {
                "pool_id": "497",
                "token_out_denom": "ujuno"
            }
        ]
    }
}
```
_Execute ATOM → OSMO → JUNO swap with TWAP slippage protection._

### Balancer Pool Creation
```json
{
    "create_pool": {
        "pool_type": {
            "balancer": {
                "pool_params": {
                    "swap_fee": "0.005",
                    "exit_fee": "0.0"
                },
                "pool_assets": [
                    {
                        "token": {
                            "denom": "uatom",
                            "amount": "5000000000"
                        },
                        "weight": "70"
                    },
                    {
                        "token": {
                            "denom": "uosmo",
                            "amount": "3000000000"
                        },
                        "weight": "30"
                    }
                ]
            }
        }
    }
}
```
_Create 70/30 ATOM/OSMO weighted pool with 0.5% swap fee._

### Stable Pool for Stablecoins
```json
{
    "create_pool": {
        "pool_type": {
            "stable": {
                "pool_params": {
                    "swap_fee": "0.0005",
                    "exit_fee": "0.0"
                },
                "scaling_factors": [1000000, 1000000]
            }
        }
    }
}
```
_Create stable pool for USDC/USDT with low 0.05% swap fee._

### Concentrated Liquidity Pool
```json
{
    "create_pool": {
        "pool_type": {
            "concentrated": {
                "tick_spacing": 100,
                "spread_factor": "0.002"
            }
        }
    }
}
```
_Create concentrated liquidity pool with 0.2% spread factor._

## Operational Examples

### Query Optimal Route
```json
{
    "get_route": {
        "from_denom": "uatom",
        "to_denom": "ujuno"
    }
}
```

### Withdraw from Pool
```json
{
    "withdraw_pool": {
        "withdraw_msg": {
            "sender": "andr1contract_address...",
            "pool_id": "567",
            "pool_shares_out": "500000000",
            "token_out_mins": [
                {
                    "denom": "uatom",
                    "amount": "2250000000"
                },
                {
                    "denom": "uosmo",
                    "amount": "1350000000"
                }
            ]
        }
    }
}
```

### Update Router
```json
{
    "update_swap_router": {
        "swap_router": "andr1upgraded_router..."
    }
}
```

## Integration Patterns

### With App Contract
The Socket Osmosis ADO can be integrated into App contracts for DeFi functionality:

```json
{
    "components": [
        {
            "name": "osmosis_gateway",
            "ado_type": "socket-osmosis",
            "component_type": {
                "new": {
                    "swap_router": null
                }
            }
        },
        {
            "name": "trading_strategy",
            "ado_type": "app-contract",
            "component_type": {
                "new": {
                    "components": ["./osmosis_gateway"]
                }
            }
        }
    ]
}
```

### Automated Trading Strategies
For algorithmic trading and portfolio management:

1. **Deploy socket contract** with appropriate router configuration
2. **Implement strategy logic** in calling contracts or off-chain systems
3. **Execute trades through socket** with slippage protection
4. **Monitor and rebalance** portfolios through automated swaps

### Liquidity Management
For liquidity provision and farming:

1. **Create pools** with desired parameters and initial liquidity
2. **Track LP tokens** through contract state management
3. **Monitor pool performance** through external tooling
4. **Withdraw liquidity** when strategies require rebalancing

### DeFi Composability
For complex DeFi workflows:

1. **Chain operations** through AMP messaging for complex strategies
2. **Integrate with other ADOs** for complete DeFi solutions
3. **Implement cross-protocol** strategies spanning multiple DEXs
4. **Automate yield farming** through programmatic pool management

## Advanced Features

### **TWAP Slippage Protection**
- **Time-weighted pricing**: Use historical price data for slippage calculation
- **Configurable windows**: Set custom time windows for TWAP calculation
- **Percentage-based protection**: Specify maximum acceptable slippage percentage
- **MEV resistance**: Built-in protection against sandwich attacks

### **Custom Routing**
- **Manual route specification**: Define exact swap paths for optimal execution
- **Multi-hop support**: Execute complex swaps through multiple pools
- **Route optimization**: Leverage Osmosis's routing algorithms when no custom route specified
- **Gas optimization**: Efficient routing to minimize transaction costs

### **Pool Management**
- **LP token tracking**: Automatic tracking of created pools and LP tokens
- **Position management**: Track liquidity positions for withdrawal management
- **Pool ID mapping**: Maintain mappings between users and their created pools
- **Automated distribution**: Automatic LP token distribution to pool creators

### **AMP Integration**
- **Message forwarding**: Seamless integration with Andromeda Messaging Protocol
- **Context preservation**: Maintain message context through swap operations
- **Cross-contract workflows**: Enable complex multi-step operations
- **Error handling**: Robust error handling and recovery mechanisms

## Security Features

### **Access Control**
- **Owner restrictions**: Only contract owner can update critical parameters
- **Pool creator validation**: Only pool creators can withdraw from their pools
- **Address validation**: Comprehensive validation of all addresses
- **Permission checking**: Validate permissions before executing operations

### **Fund Protection**
- **Atomic operations**: All operations are atomic to prevent partial failures
- **Slippage protection**: Multiple slippage protection mechanisms
- **Balance verification**: Verify balances before and after operations
- **Refund mechanisms**: Automatic refunds for failed operations

### **Input Validation**
- **Parameter validation**: Comprehensive validation of all input parameters
- **Route validation**: Validate swap routes before execution
- **Amount checking**: Verify token amounts and prevent zero-amount operations
- **Denomination verification**: Ensure token denominations are valid

### **State Management**
- **Consistent state**: Maintain consistent contract state across operations
- **Error recovery**: Graceful handling of failed operations
- **Replay protection**: Prevent duplicate operations and state corruption
- **Cleanup mechanisms**: Proper cleanup of temporary state

## Important Notes

- **Single coin requirement**: SwapAndForward requires exactly one coin as funds
- **Two coins for pools**: Pool creation requires exactly two coins as initial liquidity
- **Router dependency**: Contract depends on external Osmosis router for swap execution
- **Pool creator tracking**: Only addresses that create pools can withdraw from them
- **Slippage protection**: Always use appropriate slippage protection for swaps
- **Gas considerations**: Complex swaps and pool operations consume significant gas
- **Network-specific**: Router addresses may vary between different Osmosis deployments
- **LP token management**: LP tokens are automatically transferred to pool creators

## Common Workflow

### 1. **Deploy Socket Contract**
```json
{
    "swap_router": null
}
```
_Use default Osmosis router._

### 2. **Execute Simple Swap**
```json
{
    "swap_and_forward": {
        "to_denom": "uosmo",
        "recipient": null,
        "slippage": {
            "min_output_amount": "950000000"
        },
        "route": null
    }
}
```
_Send ATOM as funds with the transaction._

### 3. **Query Route for Complex Swap**
```json
{
    "get_route": {
        "from_denom": "uatom",
        "to_denom": "ujuno"
    }
}
```

### 4. **Execute Multi-Hop Swap**
```json
{
    "swap_and_forward": {
        "to_denom": "ujuno",
        "recipient": {
            "address": "andr1destination...",
            "msg": null
        },
        "slippage": {
            "twap": {
                "window_seconds": 600,
                "slippage_percentage": "0.05"
            }
        },
        "route": [
            {
                "pool_id": "1",
                "token_out_denom": "uosmo"
            },
            {
                "pool_id": "497",
                "token_out_denom": "ujuno"
            }
        ]
    }
}
```

### 5. **Create Liquidity Pool**
```json
{
    "create_pool": {
        "pool_type": {
            "balancer": {
                "pool_params": {
                    "swap_fee": "0.003",
                    "exit_fee": "0.0"
                },
                "pool_assets": [
                    {
                        "token": {
                            "denom": "uatom",
                            "amount": "1000000000"
                        },
                        "weight": "50"
                    },
                    {
                        "token": {
                            "denom": "uosmo",
                            "amount": "1000000000"
                        },
                        "weight": "50"
                    }
                ]
            }
        }
    }
}
```
_Send both tokens as funds with the transaction._

### 6. **Withdraw from Pool**
```json
{
    "withdraw_pool": {
        "withdraw_msg": {
            "sender": "andr1contract_address...",
            "pool_id": "123",
            "pool_shares_out": "500000000",
            "token_out_mins": [
                {
                    "denom": "uatom",
                    "amount": "450000000"
                },
                {
                    "denom": "uosmo",
                    "amount": "450000000"
                }
            ]
        }
    }
}
```

The Socket Osmosis ADO provides essential DeFi infrastructure for the Andromeda ecosystem, enabling sophisticated trading strategies, liquidity management, and automated portfolio operations with seamless integration to Osmosis's advanced DEX features.