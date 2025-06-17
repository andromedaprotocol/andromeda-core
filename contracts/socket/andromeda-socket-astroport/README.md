# Andromeda Socket Astroport ADO

## Introduction

The Andromeda Socket Astroport ADO is a comprehensive DEX integration contract that provides seamless interaction with the Astroport decentralized exchange protocol. This socket enables automated token swapping, liquidity management, and pair operations through a unified interface, supporting both native and CW20 tokens with advanced features like multi-hop swaps, slippage protection, and automated liquidity provision for sophisticated DeFi applications.

<b>Ado_type:</b> socket-astroport

## Why Socket Astroport ADO

The Socket Astroport ADO serves as a critical DeFi infrastructure component for applications requiring:

- **Automated Token Swapping**: Execute token swaps with slippage protection and minimum receive guarantees
- **Liquidity Management**: Provide and withdraw liquidity from Astroport pools
- **DEX Integration**: Seamlessly integrate with Astroport's comprehensive DEX ecosystem
- **Multi-Hop Trading**: Execute complex trading routes across multiple token pairs
- **Arbitrage Operations**: Build automated arbitrage bots and trading strategies
- **Portfolio Management**: Rebalance portfolios through automated swapping
- **DeFi Automation**: Create complex DeFi workflows with automated execution
- **Liquidity Mining**: Participate in liquidity mining with automated provision
- **Cross-Asset Operations**: Convert between different asset types seamlessly
- **Trading Infrastructure**: Build advanced trading applications and interfaces

The ADO provides comprehensive Astroport integration with swap execution, liquidity operations, pair management, and simulation capabilities for sophisticated DeFi applications.

## Key Features

### **Token Swapping**
- **Native and CW20 support**: Swap between native tokens and CW20 tokens
- **Multi-hop operations**: Execute complex swap routes across multiple pairs
- **Slippage protection**: Configure maximum acceptable slippage tolerance
- **Minimum receive**: Set minimum output amounts for swap protection
- **Auto-forwarding**: Automatically forward swapped tokens to recipients

### **Liquidity Operations**
- **Liquidity provision**: Add liquidity to existing or new Astroport pairs
- **Liquidity withdrawal**: Remove liquidity and receive underlying assets
- **Auto-staking**: Automatically stake LP tokens in Astroport's Generator
- **Flexible receivers**: Specify different receivers for LP tokens
- **Slippage tolerance**: Configure acceptable slippage for liquidity operations

### **Pair Management**
- **Pair creation**: Create new trading pairs on Astroport
- **Combined operations**: Create pairs and provide liquidity in single transactions
- **Multiple pair types**: Support for XYK, Stable, and Custom pair types
- **Initialization parameters**: Configure custom parameters for specialized pools

### **Advanced Features**
- **Swap simulation**: Preview swap outcomes before execution
- **Router updates**: Update swap router addresses for protocol upgrades
- **Reply handling**: Comprehensive error handling and state management
- **Asset conversion**: Convert between different asset representation formats

## Pair Types

### **XYK Pairs**
Traditional constant product market maker pairs suitable for most token pairs:
```json
{
    "xyk": {}
}
```

### **Stable Pairs**
Optimized for assets with similar values (stablecoins, liquid staking derivatives):
```json
{
    "stable": {}
}
```

### **Custom Pairs**
Specialized pair types with custom logic:
```json
{
    "custom": "custom_pair_type_identifier"
}
```

## InstantiateMsg

```rust
pub struct InstantiateMsg {
    pub swap_router: Option<AndrAddr>,
    pub factory: Option<AndrAddr>,
}
```

```json
{
    "swap_router": "/lib/astroport/router",
    "factory": "/lib/astroport/factory"
}
```

**Parameters**:
- **swap_router**: Optional Astroport router contract address
  - Defaults to "/lib/astroport/router" if not specified
  - Used for executing swap operations
  - Can be updated by contract owner
- **factory**: Optional Astroport factory contract address
  - Defaults to "/lib/astroport/factory" if not specified
  - Used for creating new trading pairs

## ExecuteMsg

### SwapAndForward
Swaps native tokens and forwards the result to a recipient.

```rust
SwapAndForward {
    to_asset: Asset,
    recipient: Option<Recipient>,
    max_spread: Option<Decimal>,
    minimum_receive: Option<Uint128>,
    operations: Option<Vec<SwapOperation>>,
}
```

```json
{
    "swap_and_forward": {
        "to_asset": {
            "native_token": "uusd"
        },
        "recipient": {
            "address": "andr1recipient...",
            "msg": null
        },
        "max_spread": "0.05",
        "minimum_receive": "95000000",
        "operations": [
            {
                "offer_asset_info": {
                    "native_token": "uluna"
                },
                "ask_asset_info": {
                    "native_token": "uusd"
                }
            }
        ]
    }
}
```

**Parameters**:
- **to_asset**: Target asset to receive from swap
- **recipient**: Optional recipient address (defaults to sender)
- **max_spread**: Maximum acceptable slippage (as decimal)
- **minimum_receive**: Minimum amount to receive from swap
- **operations**: Optional custom swap route (auto-generated if omitted)

### Receive (CW20 Hook)
Handles CW20 token swaps through the receive hook mechanism.

```rust
pub enum Cw20HookMsg {
    SwapAndForward {
        to_asset: Asset,
        recipient: Option<Recipient>,
        max_spread: Option<Decimal>,
        minimum_receive: Option<Uint128>,
        operations: Option<Vec<SwapOperation>>,
    },
}
```

```json
{
    "send": {
        "contract": "andr1socket_astroport...",
        "amount": "100000000",
        "msg": "eyJzd2FwX2FuZF9mb3J3YXJkIjp7InRvX2Fzc2V0Ijp7Im5hdGl2ZV90b2tlbiI6InV1c2QifX19"
    }
}
```

### CreatePair
Creates a new trading pair on Astroport.

```rust
CreatePair {
    pair_type: PairType,
    asset_infos: Vec<AssetInfo>,
    init_params: Option<Binary>,
}
```

```json
{
    "create_pair": {
        "pair_type": {
            "xyk": {}
        },
        "asset_infos": [
            {
                "native_token": {
                    "denom": "uluna"
                }
            },
            {
                "token": {
                    "contract_addr": "andr1cw20_token..."
                }
            }
        ],
        "init_params": null
    }
}
```

**Parameters**:
- **pair_type**: Type of trading pair (XYK, Stable, or Custom)
- **asset_infos**: Information about the two assets in the pair
- **init_params**: Optional initialization parameters for custom pair types

### ProvideLiquidity
Adds liquidity to an existing trading pair.

```rust
ProvideLiquidity {
    assets: Vec<AssetEntry>,
    slippage_tolerance: Option<Decimal>,
    auto_stake: Option<bool>,
    receiver: Option<AndrAddr>,
    pair_address: AndrAddr,
}
```

```json
{
    "provide_liquidity": {
        "assets": [
            {
                "info": {
                    "native_token": {
                        "denom": "uluna"
                    }
                },
                "amount": "1000000"
            },
            {
                "info": {
                    "token": {
                        "contract_addr": "andr1cw20_token..."
                    }
                },
                "amount": "1000000"
            }
        ],
        "slippage_tolerance": "0.01",
        "auto_stake": true,
        "receiver": "andr1lp_receiver...",
        "pair_address": "andr1pair_contract..."
    }
}
```

**Parameters**:
- **assets**: Assets to provide as liquidity
- **slippage_tolerance**: Maximum acceptable slippage for the operation
- **auto_stake**: Whether to automatically stake LP tokens in Generator
- **receiver**: Optional address to receive LP tokens
- **pair_address**: Address of the target trading pair

### CreatePairAndProvideLiquidity
Creates a new pair and provides initial liquidity in a single transaction.

```rust
CreatePairAndProvideLiquidity {
    pair_type: PairType,
    asset_infos: Vec<AssetInfo>,
    init_params: Option<Binary>,
    assets: Vec<AssetEntry>,
    slippage_tolerance: Option<Decimal>,
    auto_stake: Option<bool>,
    receiver: Option<AndrAddr>,
}
```

```json
{
    "create_pair_and_provide_liquidity": {
        "pair_type": {
            "stable": {}
        },
        "asset_infos": [
            {
                "native_token": {
                    "denom": "uusd"
                }
            },
            {
                "native_token": {
                    "denom": "usdt"
                }
            }
        ],
        "init_params": null,
        "assets": [
            {
                "info": {
                    "native_token": {
                        "denom": "uusd"
                    }
                },
                "amount": "1000000000"
            },
            {
                "info": {
                    "native_token": {
                        "denom": "usdt"
                    }
                },
                "amount": "1000000000"
            }
        ],
        "slippage_tolerance": "0.01",
        "auto_stake": false,
        "receiver": null
    }
}
```

### WithdrawLiquidity
Withdraws liquidity from a trading pair.

```rust
WithdrawLiquidity {
    pair_address: AndrAddr,
}
```

```json
{
    "withdraw_liquidity": {
        "pair_address": "andr1pair_contract..."
    }
}
```

**Note**: LP tokens must be sent as funds with this message.

### UpdateSwapRouter
Updates the swap router address (owner-only operation).

```rust
UpdateSwapRouter {
    swap_router: AndrAddr,
}
```

```json
{
    "update_swap_router": {
        "swap_router": "andr1new_router..."
    }
}
```

## QueryMsg

### SimulateSwapOperation
Simulates a swap operation to preview expected output.

```rust
pub enum QueryMsg {
    #[returns(SimulateSwapOperationResponse)]
    SimulateSwapOperation {
        offer_amount: Uint128,
        operations: Vec<SwapOperation>,
    },
}
```

```json
{
    "simulate_swap_operation": {
        "offer_amount": "1000000",
        "operations": [
            {
                "offer_asset_info": {
                    "native_token": "uluna"
                },
                "ask_asset_info": {
                    "native_token": "uusd"
                }
            }
        ]
    }
}
```

**Response:**
```json
{
    "amount": "4750000"
}
```

## Usage Examples

### Basic Token Swap
```json
{
    "swap_and_forward": {
        "to_asset": {
            "cw20_token": "andr1usdc_token..."
        },
        "max_spread": "0.05",
        "minimum_receive": "95000000"
    }
}
```
_Swaps native tokens sent as funds to USDC with 5% max slippage._

### Multi-Hop Swap
```json
{
    "swap_and_forward": {
        "to_asset": {
            "native_token": "uatom"
        },
        "operations": [
            {
                "offer_asset_info": {
                    "native_token": "uluna"
                },
                "ask_asset_info": {
                    "native_token": "uusd"
                }
            },
            {
                "offer_asset_info": {
                    "native_token": "uusd"
                },
                "ask_asset_info": {
                    "native_token": "uatom"
                }
            }
        ]
    }
}
```
_Swaps LUNA → USD → ATOM in a single transaction._

### CW20 Token Swap
```json
{
    "send": {
        "contract": "andr1socket_astroport...",
        "amount": "1000000",
        "msg": "eyJzd2FwX2FuZF9mb3J3YXJkIjp7InRvX2Fzc2V0Ijp7Im5hdGl2ZV90b2tlbiI6InVsdW5hIn19fQ=="
    }
}
```
_Base64 encoded: {"swap_and_forward":{"to_asset":{"native_token":"uluna"}}}_

### Create Stablecoin Pool
```json
{
    "create_pair_and_provide_liquidity": {
        "pair_type": {
            "stable": {}
        },
        "asset_infos": [
            {
                "native_token": {
                    "denom": "uusd"
                }
            },
            {
                "token": {
                    "contract_addr": "andr1usdt_token..."
                }
            }
        ],
        "assets": [
            {
                "info": {
                    "native_token": {
                        "denom": "uusd"
                    }
                },
                "amount": "10000000000"
            },
            {
                "info": {
                    "token": {
                        "contract_addr": "andr1usdt_token..."
                    }
                },
                "amount": "10000000000"
            }
        ],
        "slippage_tolerance": "0.001",
        "auto_stake": true
    }
}
```

### Arbitrage Bot Setup
```json
{
    "swap_and_forward": {
        "to_asset": {
            "native_token": "uluna"
        },
        "recipient": {
            "address": "andr1arbitrage_bot...",
            "msg": "eyJjaGVja19wcm9maXQiOnt9fQ=="
        },
        "minimum_receive": "1050000000"
    }
}
```

## Integration Patterns

### With App Contract
The Socket Astroport can be integrated into App contracts for DeFi operations:

```json
{
    "components": [
        {
            "name": "astroport_dex",
            "ado_type": "socket-astroport",
            "component_type": {
                "new": {
                    "swap_router": "/lib/astroport/router",
                    "factory": "/lib/astroport/factory"
                }
            }
        }
    ]
}
```

### Automated Trading
For algorithmic trading strategies:

1. **Deploy socket contract** with Astroport integration
2. **Set up trading logic** in external contracts or bots
3. **Execute swaps** based on market conditions and strategies
4. **Monitor performance** through transaction events and queries

### Liquidity Management
For automated liquidity provision:

1. **Create trading pairs** for new tokens or strategies
2. **Provide initial liquidity** to bootstrap trading
3. **Monitor pool performance** and adjust positions
4. **Withdraw liquidity** when conditions change

### DeFi Yield Farming
For yield optimization strategies:

1. **Swap tokens** to optimal yield farming assets
2. **Provide liquidity** to high-yield pools
3. **Auto-stake LP tokens** in Generator for additional rewards
4. **Compound rewards** by swapping and re-investing

## Security Features

### **Slippage Protection**
- **Maximum spread limits**: Prevent excessive slippage during volatile conditions
- **Minimum receive guarantees**: Ensure minimum output amounts are met
- **Transaction reversal**: Automatic reversal if conditions not met
- **Market impact control**: Limit large trades that could affect market prices

### **Access Controls**
- **Owner restrictions**: Router updates restricted to contract owner
- **Recipient validation**: Validate all recipient addresses before operations
- **Permission checks**: Verify permissions for restricted operations
- **State management**: Secure state transitions during complex operations

### **Error Handling**
- **Reply mechanisms**: Comprehensive error handling for Astroport operations
- **State cleanup**: Automatic cleanup of temporary state on failures
- **Refund mechanisms**: Return assets to users on failed operations
- **Event logging**: Detailed logging for debugging and monitoring

### **Asset Security**
- **Balance tracking**: Track asset balances before and after operations
- **Transfer validation**: Validate all asset transfers and allowances
- **Atomic operations**: Ensure operations complete fully or revert entirely
- **Overflow protection**: Prevent arithmetic overflow in calculations

## Swap Operations

### **Single-Hop Swaps**
Direct swaps between two assets through a single pool:
```json
{
    "operations": [
        {
            "offer_asset_info": {
                "native_token": "uluna"
            },
            "ask_asset_info": {
                "native_token": "uusd"
            }
        }
    ]
}
```

### **Multi-Hop Swaps**
Complex routes through multiple pools for optimal pricing:
```json
{
    "operations": [
        {
            "offer_asset_info": {
                "native_token": "uluna"
            },
            "ask_asset_info": {
                "cw20_token": "andr1intermediate..."
            }
        },
        {
            "offer_asset_info": {
                "cw20_token": "andr1intermediate..."
            },
            "ask_asset_info": {
                "native_token": "uatom"
            }
        }
    ]
}
```

## Important Notes

- **Router configuration**: Swap router must be configured during instantiation or updated by owner
- **Asset allowances**: CW20 tokens require proper allowance setup for liquidity operations
- **Slippage settings**: Always configure appropriate slippage tolerance for market conditions
- **Multi-hop complexity**: Complex swap routes may have higher gas costs and slippage
- **LP token handling**: LP tokens are automatically managed during liquidity operations
- **Auto-staking**: LP tokens can be automatically staked in Astroport's Generator contract
- **State persistence**: Contract maintains state during multi-step operations
- **Error recovery**: Failed operations automatically clean up state and refund assets

## Common Workflows

### 1. **Execute Simple Swap**
```json
{
    "swap_and_forward": {
        "to_asset": {
            "native_token": "uusd"
        },
        "max_spread": "0.05"
    }
}
```
_Send native tokens as funds with the transaction._

### 2. **Create New Pool**
```json
{
    "create_pair": {
        "pair_type": {
            "xyk": {}
        },
        "asset_infos": [
            {
                "native_token": {
                    "denom": "uluna"
                }
            },
            {
                "token": {
                    "contract_addr": "andr1new_token..."
                }
            }
        ]
    }
}
```

### 3. **Add Liquidity**
```json
{
    "provide_liquidity": {
        "assets": [
            {
                "info": {
                    "native_token": {
                        "denom": "uluna"
                    }
                },
                "amount": "1000000"
            },
            {
                "info": {
                    "token": {
                        "contract_addr": "andr1token..."
                    }
                },
                "amount": "1000000"
            }
        ],
        "pair_address": "andr1pair..."
    }
}
```

### 4. **Simulate Before Trading**
```json
{
    "simulate_swap_operation": {
        "offer_amount": "1000000",
        "operations": [
            {
                "offer_asset_info": {
                    "native_token": "uluna"
                },
                "ask_asset_info": {
                    "native_token": "uusd"
                }
            }
        ]
    }
}
```

The Socket Astroport ADO provides comprehensive integration with the Astroport DEX ecosystem, enabling sophisticated DeFi applications with automated trading, liquidity management, and yield optimization capabilities through a secure and user-friendly interface.