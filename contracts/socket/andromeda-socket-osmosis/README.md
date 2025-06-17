# Andromeda Socket Osmosis ADO

## Introduction

The Andromeda Socket Osmosis ADO is a powerful integration contract that provides seamless access to Osmosis DEX functionality, including token swapping, liquidity pool operations, and advanced DeFi features. This contract serves as a bridge between the Andromeda ecosystem and the Osmosis protocol, enabling automated market making, token swaps, pool creation, and liquidity management. The socket supports multiple pool types, sophisticated routing, slippage protection, and comprehensive pool operations, making it essential for DeFi applications, trading systems, and liquidity management strategies.

<b>Ado_type:</b> socket-osmosis

## Why Socket Osmosis ADO

The Socket Osmosis ADO serves as essential DeFi infrastructure for applications requiring:

- **Token Trading**: Execute token swaps with optimal routing and slippage protection
- **Liquidity Provision**: Create and manage liquidity pools across different pool types
- **DeFi Integration**: Integrate with Osmosis protocol for advanced DeFi operations
- **Automated Market Making**: Implement automated market making strategies
- **Pool Management**: Create, manage, and withdraw from various types of liquidity pools
- **Cross-Asset Trading**: Enable trading between different tokens and assets
- **Yield Farming**: Participate in liquidity mining and yield farming opportunities
- **Portfolio Management**: Manage token portfolios with automated rebalancing
- **Arbitrage Operations**: Execute arbitrage strategies across different pools
- **Financial Services**: Provide comprehensive DeFi services to users

The ADO provides comprehensive Osmosis integration with sophisticated trading and liquidity management capabilities.

## Key Features

### **Advanced Token Swapping**
- **Optimal routing**: Automatic route optimization for best swap prices
- **Multi-hop swaps**: Support for complex multi-hop swap operations
- **Slippage protection**: Configurable slippage protection mechanisms
- **TWAP integration**: Time-weighted average price calculations for better execution
- **Custom routing**: Override automatic routing with custom swap routes

### **Multiple Pool Types**
- **Balancer pools**: Create and manage weighted balancer pools
- **Stable swap pools**: Optimized pools for stable assets with minimal slippage
- **Concentrated liquidity**: High-efficiency concentrated liquidity pools
- **CosmWasm pools**: Custom pool implementations using CosmWasm contracts
- **Pool governance**: Comprehensive pool governance and management features

### **Liquidity Management**
- **Pool creation**: Create various types of liquidity pools
- **Liquidity provision**: Add liquidity to existing pools
- **Pool withdrawal**: Withdraw liquidity and claim rewards
- **LP token management**: Automatic LP token handling and distribution
- **Yield optimization**: Optimize yields through strategic liquidity placement

### **Router Integration**
- **Swap router**: Integration with Osmosis swap router for optimal execution
- **Route queries**: Query optimal routes for any token pair
- **Dynamic routing**: Automatic route updates based on market conditions
- **Router management**: Administrative control over router configuration
- **Performance optimization**: Optimized routing for minimal fees and slippage

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
- **swap_router**: Optional address of the Osmosis swap router (defaults to standard router)

**Configuration**:
- If no router specified, uses default Osmosis router address
- Router address must be valid and accessible
- Router configuration affects all swap operations

## ExecuteMsg

### SwapAndForward
Executes a token swap and forwards the result to a recipient.

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
            "address": "andr1recipient...",
            "msg": null
        },
        "slippage": {
            "twap": {
                "window_seconds": 300,
                "slippage_percentage": "0.05"
            }
        },
        "route": [
            {
                "pool_id": "1",
                "token_out_denom": "uosmo"
            }
        ]
    }
}
```

**Parameters**:
- **to_denom**: Target denomination to swap to
- **recipient**: Optional recipient (defaults to sender)
- **slippage**: Slippage protection configuration
- **route**: Optional custom swap route

**Usage**: Send tokens as funds to swap them to the target denomination

### CreatePool
Creates a new liquidity pool.

```rust
CreatePool {
    pool_type: Pool,
}

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
                            "amount": "1000000"
                        },
                        "weight": "50"
                    },
                    {
                        "token": {
                            "denom": "uosmo",
                            "amount": "1000000"
                        },
                        "weight": "50"
                    }
                ]
            }
        }
    }
}
```

**Requirements**: Must send exactly 2 tokens as funds for pool creation

### WithdrawPool
Withdraws liquidity from a pool.

```rust
WithdrawPool {
    withdraw_msg: MsgExitPool,
}
```

**Authorization**: Only the original pool creator can withdraw
**Effect**: Exits the pool and returns underlying assets

### UpdateSwapRouter
Updates the swap router address (owner-only).

```rust
UpdateSwapRouter {
    swap_router: AndrAddr,
}
```

```json
{
    "update_swap_router": {
        "swap_router": "andr1new_router_address..."
    }
}
```

**Authorization**: Only contract owner can update router
**Effect**: Changes router for all future swap operations

## QueryMsg

### GetRoute
Returns the optimal swap route between two denominations.

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
            "token_out_denom": "uosmo"
        }
    ]
}
```

## Usage Examples

### Simple Token Swap
```json
{
    "swap_and_forward": {
        "to_denom": "uosmo",
        "recipient": null,
        "slippage": {
            "min_output_amount": "950000"
        },
        "route": null
    }
}
```
_Send 1000000 uatom as funds to swap to OSMO_

### Multi-Hop Swap with Custom Route
```json
{
    "swap_and_forward": {
        "to_denom": "ujuno",
        "recipient": {
            "address": "andr1recipient...",
            "msg": null
        },
        "slippage": {
            "twap": {
                "window_seconds": 600,
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

### Create Balancer Pool
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
                            "amount": "10000000"
                        },
                        "weight": "60"
                    },
                    {
                        "token": {
                            "denom": "uosmo",
                            "amount": "5000000"
                        },
                        "weight": "40"
                    }
                ]
            }
        }
    }
}
```

### Create Concentrated Liquidity Pool
```json
{
    "create_pool": {
        "pool_type": {
            "concentrated": {
                "tick_spacing": 100,
                "spread_factor": "0.001"
            }
        }
    }
}
```

## Integration Patterns

### With App Contract
Osmosis socket can be integrated for DeFi functionality:

```json
{
    "components": [
        {
            "name": "osmosis_dex",
            "ado_type": "socket-osmosis",
            "component_type": {
                "new": {
                    "swap_router": "andr1osmosis_router..."
                }
            }
        }
    ]
}
```

### DeFi Trading Platform
For building trading applications:

1. **Deploy Osmosis socket** with appropriate router configuration
2. **Implement swap functionality** with slippage protection
3. **Query optimal routes** for price discovery
4. **Execute trades** with automatic forwarding
5. **Manage liquidity** through pool operations

### Automated Market Making
For AMM strategies:

1. **Create liquidity pools** with strategic asset ratios
2. **Monitor pool performance** and adjust as needed
3. **Withdraw liquidity** when market conditions change
4. **Rebalance portfolios** through swap operations
5. **Optimize yields** through strategic pool selection

## Important Notes

- **Osmosis integration**: Direct integration with Osmosis protocol
- **Pool creation**: Requires exactly 2 tokens for pool creation
- **LP token management**: Automatic LP token distribution to pool creators
- **Slippage protection**: Multiple slippage protection mechanisms available
- **Router dependency**: All swaps depend on configured router
- **Pool governance**: Pool creators become governors of their pools
- **Withdrawal permissions**: Only pool creators can withdraw from their pools
- **Custom routing**: Supports both automatic and custom routing

## Common Workflow

### 1. **Deploy Osmosis Socket**
```json
{
    "swap_router": null
}
```

### 2. **Execute Token Swap**
```json
{
    "swap_and_forward": {
        "to_denom": "uosmo",
        "recipient": null,
        "slippage": {
            "twap": {
                "window_seconds": 300,
                "slippage_percentage": "0.05"
            }
        },
        "route": null
    }
}
```

### 3. **Query Swap Route**
```json
{
    "get_route": {
        "from_denom": "uatom",
        "to_denom": "uosmo"
    }
}
```

### 4. **Create Liquidity Pool**
```json
{
    "create_pool": {
        "pool_type": {
            "balancer": {
                "pool_params": null,
                "pool_assets": [
                    {
                        "token": {
                            "denom": "uatom",
                            "amount": "1000000"
                        },
                        "weight": "50"
                    },
                    {
                        "token": {
                            "denom": "uosmo",
                            "amount": "1000000"
                        },
                        "weight": "50"
                    }
                ]
            }
        }
    }
}
```

The Socket Osmosis ADO provides comprehensive DeFi infrastructure for the Andromeda ecosystem, enabling sophisticated trading, liquidity management, and automated market making with seamless Osmosis protocol integration.