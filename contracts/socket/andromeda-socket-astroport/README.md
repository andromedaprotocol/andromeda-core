# Andromeda Socket Astroport ADO

## Introduction

The Andromeda Socket Astroport ADO is a specialized integration module that provides seamless access to Astroport DEX functionality within the Andromeda ecosystem. This contract enables automated token swapping, liquidity pool creation and management, and advanced trading operations while maintaining full integration with Andromeda's messaging protocol (AMP). The socket acts as a bridge between Andromeda applications and Astroport's sophisticated DeFi features, enabling complex trading strategies, liquidity provision, and automated market making.

<b>Ado_type:</b> socket-astroport

## Why Socket Astroport ADO

The Socket Astroport ADO serves as critical DeFi infrastructure for applications requiring:

- **Automated Token Swapping**: Execute token swaps with automatic recipient forwarding and routing
- **Liquidity Pool Management**: Create, provide liquidity to, and withdraw from Astroport pools
- **Advanced Trading Strategies**: Implement sophisticated trading algorithms through multi-hop swaps
- **Cross-Token Operations**: Seamless swapping between native tokens and CW20 tokens
- **Pool Creation and Management**: Create new trading pairs and manage liquidity positions
- **Slippage Protection**: Advanced slippage and minimum receive protection mechanisms
- **DeFi Integration**: Access Astroport's full feature set from Andromeda applications
- **Automated Market Making**: Implement AMM strategies through programmatic pool management
- **Portfolio Rebalancing**: Automate portfolio management through intelligent swapping
- **Yield Optimization**: Optimize returns through strategic liquidity provision

The ADO supports all Astroport pool types including XYK, Stable, and Custom pools with comprehensive liquidity management features.

## Key Features

### **Token Swapping**
- **Multi-asset support**: Swap between native tokens and CW20 tokens
- **Automatic routing**: Intelligent routing through optimal swap paths
- **Slippage protection**: Multiple slippage protection mechanisms including max spread and minimum receive
- **Multi-hop swaps**: Execute complex multi-hop swaps through multiple pools
- **CW20 integration**: Full support for CW20 token swapping through receive hooks

### **Liquidity Management**
- **Pool creation**: Create new Astroport trading pairs with configurable parameters
- **Liquidity provision**: Provide liquidity to existing or newly created pools
- **Combined operations**: Create pools and provide liquidity in single transactions
- **LP token management**: Automatic handling of LP token distribution and staking
- **Liquidity withdrawal**: Withdraw liquidity positions with automatic asset distribution

### **Advanced Pool Types**
- **XYK pools**: Standard constant product automated market maker pools
- **Stable pools**: Optimized pools for stable asset trading with minimal slippage
- **Custom pools**: Support for custom pool implementations with flexible parameters
- **Pool simulation**: Simulate swap operations to preview results before execution
- **Dynamic pricing**: Real-time pricing through Astroport's pricing algorithms

### **Integration Features**
- **AMP compatibility**: Full integration with Andromeda Messaging Protocol
- **Recipient forwarding**: Automatic forwarding of swapped tokens to specified recipients
- **Factory integration**: Direct integration with Astroport factory for pool creation
- **Router optimization**: Leverage Astroport router for optimal swap execution
- **Error handling**: Comprehensive error handling and transaction safety

## Pool Types

### **XYK Pools**
Constant product pools for general token swapping:
- **Balanced liquidity**: Equal value provision of two assets
- **Dynamic pricing**: Price determined by constant product formula (x * y = k)
- **General purpose**: Suitable for most token pairs
- **Flexible ratios**: Support for different token weightings

### **Stable Pools**
Optimized for stable asset trading:
- **Low slippage**: Minimal price impact for stable asset swaps
- **Correlated assets**: Designed for assets that maintain similar values
- **Capital efficiency**: High capital efficiency for stable swaps
- **Precision**: High precision arithmetic for stable asset calculations

### **Custom Pools**
Flexible pool implementations:
- **Custom logic**: Support for specialized pool algorithms
- **Configurable parameters**: Flexible parameter configuration through init_params
- **Extensibility**: Enable new pool types and trading mechanisms
- **Innovation**: Support for experimental and advanced pool designs

## InstantiateMsg

```rust
pub struct InstantiateMsg {
    pub swap_router: Option<AndrAddr>,
    pub factory: Option<AndrAddr>,
}
```

```json
{
    "swap_router": "andr1astroport_router_address...",
    "factory": "andr1astroport_factory_address..."
}
```

**Parameters**:
- **swap_router**: Optional Astroport router contract address (defaults to `/lib/astroport/router`)
- **factory**: Optional Astroport factory contract address (defaults to `/lib/astroport/factory`)

**Default Configuration**:
If no addresses are provided, the contract uses default Astroport addresses that automatically resolve to the appropriate contracts for the current network.

**Validation**:
- All addresses must be valid and resolvable
- Contracts must implement the required Astroport interfaces
- Address validation occurs during instantiation

## ExecuteMsg

### SwapAndForward
Executes a token swap and forwards the result to a specified recipient.

```rust
SwapAndForward {
    to_asset: Asset,
    recipient: Option<Recipient>,
    max_spread: Option<Decimal>,
    minimum_receive: Option<Uint128>,
    operations: Option<Vec<SwapOperation>>,
}

pub struct SwapOperation {
    pub offer_asset_info: Asset,
    pub ask_asset_info: Asset,
}
```

```json
{
    "swap_and_forward": {
        "to_asset": {
            "native_token": "uosmo"
        },
        "recipient": {
            "address": "andr1recipient...",
            "msg": null
        },
        "max_spread": "0.05",
        "minimum_receive": "950000000",
        "operations": [
            {
                "offer_asset_info": {
                    "native_token": "uatom"
                },
                "ask_asset_info": {
                    "native_token": "uosmo"
                }
            }
        ]
    }
}
```

**Usage**: Send native tokens as funds or use CW20 receive hook for token swapping.

**Parameters**:
- **to_asset**: Target asset to swap to
- **recipient**: Optional recipient address (defaults to sender)
- **max_spread**: Maximum allowed spread/slippage (as decimal)
- **minimum_receive**: Minimum amount of tokens to receive
- **operations**: Optional custom swap route (uses automatic routing if not specified)

**Requirements**:
- Must send exactly one coin as funds for native token swaps
- CW20 tokens use the Receive hook mechanism
- All addresses must be valid

### Receive (CW20 Hook)
Handles CW20 token swapping through the standard CW20 receive mechanism.

```rust
Receive(Cw20ReceiveMsg)

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

**Usage**: CW20 tokens automatically call this when sent to the contract with a hook message.

### CreatePair
Creates a new trading pair on Astroport.

```rust
CreatePair {
    pair_type: PairType,
    asset_infos: Vec<AssetInfo>,
    init_params: Option<Binary>,
}

pub enum PairType {
    Xyk {},
    Stable {},
    Custom(String),
}

pub enum AssetInfo {
    Token { contract_addr: Addr },
    NativeToken { denom: String },
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
                    "denom": "uatom"
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
- **pair_type**: Type of pool to create (XYK, Stable, or Custom)
- **asset_infos**: Information about the two assets in the pair
- **init_params**: Optional initialization parameters for custom pools

**Effect**: Creates a new trading pair on Astroport and returns the pair address

### ProvideLiquidity
Provides liquidity to an existing Astroport pool.

```rust
ProvideLiquidity {
    assets: Vec<AssetEntry>,
    slippage_tolerance: Option<Decimal>,
    auto_stake: Option<bool>,
    receiver: Option<AndrAddr>,
    pair_address: AndrAddr,
}

pub struct AssetEntry {
    pub info: AssetInfo,
    pub amount: Uint128,
}
```

```json
{
    "provide_liquidity": {
        "assets": [
            {
                "info": {
                    "native_token": {
                        "denom": "uatom"
                    }
                },
                "amount": "1000000000"
            },
            {
                "info": {
                    "token": {
                        "contract_addr": "andr1cw20_token..."
                    }
                },
                "amount": "1000000000"
            }
        ],
        "slippage_tolerance": "0.01",
        "auto_stake": true,
        "receiver": "andr1liquidity_receiver...",
        "pair_address": "andr1pair_address..."
    }
}
```

**Parameters**:
- **assets**: Assets to provide as liquidity
- **slippage_tolerance**: Maximum acceptable slippage for the transaction
- **auto_stake**: Whether to automatically stake LP tokens in the generator
- **receiver**: Optional receiver of LP tokens (defaults to sender)
- **pair_address**: Address of the target liquidity pool

**Requirements**:
- Must send native tokens as funds for native assets
- Must have CW20 token allowances for CW20 assets
- Assets must match the pool's asset types

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
                    "denom": "uusdc"
                }
            },
            {
                "native_token": {
                    "denom": "uusdt"
                }
            }
        ],
        "init_params": null,
        "assets": [
            {
                "info": {
                    "native_token": {
                        "denom": "uusdc"
                    }
                },
                "amount": "1000000000"
            },
            {
                "info": {
                    "native_token": {
                        "denom": "uusdt"
                    }
                },
                "amount": "1000000000"
            }
        ],
        "slippage_tolerance": "0.005",
        "auto_stake": false,
        "receiver": null
    }
}
```

**Efficiency**: Combines pair creation and liquidity provision in a single transaction
**Use Cases**: Initial pool creation with immediate liquidity provision

### WithdrawLiquidity
Withdraws liquidity from an Astroport pool.

```rust
WithdrawLiquidity {
    pair_address: AndrAddr,
}
```

```json
{
    "withdraw_liquidity": {
        "pair_address": "andr1pair_address..."
    }
}
```

**Usage**: Send LP tokens as funds with this message
**Effect**: Burns LP tokens and returns underlying assets to sender
**Requirements**: Must send LP tokens corresponding to the specified pair

### UpdateSwapRouter
Updates the Astroport router contract address (owner-only).

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

**Authorization**: Only contract owner can update the router
**Validation**: New router address must be valid and accessible

## QueryMsg

### SimulateSwapOperation
Simulates a swap operation to preview expected results.

```rust
#[returns(SimulateSwapOperationResponse)]
SimulateSwapOperation {
    offer_amount: Uint128,
    operations: Vec<SwapOperation>,
}

pub struct SimulateSwapOperationResponse {
    pub amount: Uint128,
}
```

```json
{
    "simulate_swap_operation": {
        "offer_amount": "1000000000",
        "operations": [
            {
                "offer_asset_info": {
                    "native_token": "uatom"
                },
                "ask_asset_info": {
                    "native_token": "uosmo"
                }
            }
        ]
    }
}
```

**Response:**
```json
{
    "amount": "1895000000"
}
```

**Usage**: Preview swap results before executing actual trades

## Usage Examples

### Simple Token Swap
```json
{
    "swap_and_forward": {
        "to_asset": {
            "native_token": "uosmo"
        },
        "recipient": null,
        "max_spread": "0.03",
        "minimum_receive": "950000000",
        "operations": null
    }
}
```
_Send ATOM as funds to swap for OSMO._

### CW20 Token Swap
```json
{
    "send": {
        "contract": "andr1socket_astroport...",
        "amount": "1000000000",
        "msg": "eyJzd2FwX2FuZF9mb3J3YXJkIjp7InRvX2Fzc2V0Ijp7Im5hdGl2ZV90b2tlbiI6InVvc21vIn0sInJlY2lwaWVudCI6bnVsbCwibWF4X3NwcmVhZCI6IjAuMDMiLCJtaW5pbXVtX3JlY2VpdmUiOiI5NTAwMDAwMDAiLCJvcGVyYXRpb25zIjpudWxsfX0="
    }
}
```
_CW20 send with SwapAndForward hook message._

### Create XYK Pool
```json
{
    "create_pair": {
        "pair_type": {
            "xyk": {}
        },
        "asset_infos": [
            {
                "native_token": {
                    "denom": "uatom"
                }
            },
            {
                "native_token": {
                    "denom": "ujuno"
                }
            }
        ],
        "init_params": null
    }
}
```

### Provide Liquidity
```json
{
    "provide_liquidity": {
        "assets": [
            {
                "info": {
                    "native_token": {
                        "denom": "uatom"
                    }
                },
                "amount": "1000000000"
            },
            {
                "info": {
                    "native_token": {
                        "denom": "ujuno"
                    }
                },
                "amount": "5000000000"
            }
        ],
        "slippage_tolerance": "0.01",
        "auto_stake": true,
        "receiver": null,
        "pair_address": "andr1pair_address..."
    }
}
```
_Send both tokens as funds._

### Create Stable Pool with Liquidity
```json
{
    "create_pair_and_provide_liquidity": {
        "pair_type": {
            "stable": {}
        },
        "asset_infos": [
            {
                "native_token": {
                    "denom": "uusdc"
                }
            },
            {
                "native_token": {
                    "denom": "uusdt"
                }
            }
        ],
        "init_params": null,
        "assets": [
            {
                "info": {
                    "native_token": {
                        "denom": "uusdc"
                    }
                },
                "amount": "1000000000"
            },
            {
                "info": {
                    "native_token": {
                        "denom": "uusdt"
                    }
                },
                "amount": "1000000000"
            }
        ],
        "slippage_tolerance": "0.002",
        "auto_stake": false,
        "receiver": null
    }
}
```
_Send both stablecoins as funds._

## Query Examples

### Simulate Swap
```json
{
    "simulate_swap_operation": {
        "offer_amount": "1000000000",
        "operations": [
            {
                "offer_asset_info": {
                    "native_token": "uatom"
                },
                "ask_asset_info": {
                    "native_token": "uosmo"
                }
            }
        ]
    }
}
```

### Multi-Hop Simulation
```json
{
    "simulate_swap_operation": {
        "offer_amount": "1000000000",
        "operations": [
            {
                "offer_asset_info": {
                    "native_token": "uatom"
                },
                "ask_asset_info": {
                    "native_token": "uosmo"
                }
            },
            {
                "offer_asset_info": {
                    "native_token": "uosmo"
                },
                "ask_asset_info": {
                    "native_token": "ujuno"
                }
            }
        ]
    }
}
```

## Integration Patterns

### With App Contract
Socket Astroport can be integrated for DeFi functionality:

```json
{
    "components": [
        {
            "name": "astroport_gateway",
            "ado_type": "socket-astroport",
            "component_type": {
                "new": {
                    "swap_router": null,
                    "factory": null
                }
            }
        },
        {
            "name": "trading_bot",
            "ado_type": "app-contract",
            "component_type": {
                "new": {
                    "components": ["./astroport_gateway"]
                }
            }
        }
    ]
}
```

### Automated Trading Strategies
For implementing algorithmic trading:

1. **Deploy socket contract** with appropriate router and factory configuration
2. **Implement trading logic** in calling contracts or off-chain systems
3. **Execute trades through socket** with slippage protection
4. **Monitor market conditions** and adjust strategies accordingly

### Liquidity Management
For liquidity provision and yield farming:

1. **Create liquidity pools** for new token pairs
2. **Provide initial liquidity** with balanced asset ratios
3. **Monitor pool performance** through external analytics
4. **Rebalance positions** based on market conditions
5. **Withdraw liquidity** when strategies require adjustment

### Portfolio Rebalancing
For automated portfolio management:

1. **Assess current allocations** through external portfolio tracking
2. **Calculate required trades** to achieve target allocations
3. **Execute rebalancing swaps** through socket contract
4. **Apply slippage protection** to minimize trading costs
5. **Track performance** and adjust strategies

## Advanced Features

### **Multi-Hop Routing**
- **Intelligent routing**: Automatic route discovery for optimal swap execution
- **Custom routes**: Specify exact swap paths for advanced trading strategies
- **Route optimization**: Leverage Astroport's routing algorithms for best prices
- **Gas optimization**: Efficient routing to minimize transaction costs

### **Slippage Protection**
- **Max spread control**: Limit maximum acceptable price spread
- **Minimum receive**: Guarantee minimum output amounts
- **Dynamic protection**: Adapt slippage protection to market conditions
- **MEV resistance**: Built-in protection against front-running and sandwich attacks

### **Liquidity Operations**
- **Atomic operations**: Create pools and provide liquidity in single transactions
- **LP token management**: Automatic handling of LP token distribution
- **Auto-staking**: Optional automatic staking in Astroport generator
- **Flexible receivers**: Send LP tokens to specified recipients

### **Asset Management**
- **Multi-asset support**: Handle both native tokens and CW20 tokens seamlessly
- **Allowance management**: Automatic CW20 allowance handling for liquidity provision
- **Asset validation**: Comprehensive validation of all asset operations
- **Balance tracking**: Accurate tracking of asset balances throughout operations

## Security Features

### **Access Control**
- **Owner restrictions**: Only contract owner can update critical parameters
- **Address validation**: Comprehensive validation of all addresses
- **Permission checking**: Validate permissions before executing operations
- **Configuration protection**: Secure configuration management

### **Transaction Safety**
- **Atomic operations**: All operations are atomic to prevent partial failures
- **Slippage protection**: Multiple layers of slippage protection
- **Balance verification**: Verify balances before and after operations
- **Error handling**: Comprehensive error handling and recovery

### **Asset Protection**
- **Secure transfers**: Safe handling of both native and CW20 token transfers
- **Allowance management**: Proper management of CW20 token allowances
- **Refund mechanisms**: Automatic refunds for failed operations
- **State consistency**: Maintain consistent state across all operations

### **Integration Security**
- **Router validation**: Validate router and factory contract interfaces
- **Reply handling**: Secure handling of transaction replies and responses
- **State management**: Robust state management for multi-step operations
- **Recovery mechanisms**: Proper recovery from failed sub-operations

## Important Notes

- **Asset requirements**: SwapAndForward requires exactly one coin as funds for native tokens
- **CW20 integration**: CW20 tokens use the Receive hook mechanism for swapping
- **Liquidity provision**: Must provide both assets for liquidity provision operations
- **LP token handling**: LP tokens are automatically distributed to appropriate recipients
- **Slippage protection**: Always use appropriate slippage protection for trades
- **Gas considerations**: Complex operations and multi-hop swaps consume significant gas
- **Router dependency**: Contract depends on external Astroport router for swap execution
- **Factory integration**: Pool creation requires access to Astroport factory contract

## Common Workflow

### 1. **Deploy Socket Contract**
```json
{
    "swap_router": null,
    "factory": null
}
```
_Use default Astroport contracts._

### 2. **Execute Simple Swap**
```json
{
    "swap_and_forward": {
        "to_asset": {
            "native_token": "uosmo"
        },
        "recipient": null,
        "max_spread": "0.03",
        "minimum_receive": "950000000",
        "operations": null
    }
}
```
_Send ATOM as funds._

### 3. **Simulate Complex Swap**
```json
{
    "simulate_swap_operation": {
        "offer_amount": "1000000000",
        "operations": [
            {
                "offer_asset_info": {
                    "native_token": "uatom"
                },
                "ask_asset_info": {
                    "native_token": "ujuno"
                }
            }
        ]
    }
}
```

### 4. **Create New Pool**
```json
{
    "create_pair": {
        "pair_type": {
            "xyk": {}
        },
        "asset_infos": [
            {
                "native_token": {
                    "denom": "uatom"
                }
            },
            {
                "native_token": {
                    "denom": "ujuno"
                }
            }
        ],
        "init_params": null
    }
}
```

### 5. **Provide Liquidity**
```json
{
    "provide_liquidity": {
        "assets": [
            {
                "info": {
                    "native_token": {
                        "denom": "uatom"
                    }
                },
                "amount": "1000000000"
            },
            {
                "info": {
                    "native_token": {
                        "denom": "ujuno"
                    }
                },
                "amount": "5000000000"
            }
        ],
        "slippage_tolerance": "0.01",
        "auto_stake": true,
        "receiver": null,
        "pair_address": "andr1pair..."
    }
}
```
_Send both tokens as funds._

### 6. **Withdraw Liquidity**
```json
{
    "withdraw_liquidity": {
        "pair_address": "andr1pair..."
    }
}
```
_Send LP tokens as funds._

The Socket Astroport ADO provides comprehensive DeFi infrastructure for the Andromeda ecosystem, enabling sophisticated trading strategies, liquidity management, and automated market making with seamless integration to Astroport's advanced DEX features.