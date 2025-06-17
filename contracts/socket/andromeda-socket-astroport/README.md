# Andromeda Astroport Socket Contract

The Andromeda Astroport Socket Contract is a comprehensive interface that allows seamless integration with the Astroport DEX protocol. This contract enables token swapping, liquidity provision, pair creation, and liquidity withdrawal operations through a unified interface.

## Overview

This contract serves as a bridge between applications and Astroport's decentralized exchange functionality, providing:

- **Token Swapping**: Swap between native tokens and CW20 tokens using Astroport's swap infrastructure
- **Liquidity Management**: Provide and withdraw liquidity from Astroport pools
- **Pair Creation**: Create new trading pairs on Astroport
- **Forward Operations**: Execute swaps and automatically forward results to specified recipients

## Contract Information

- **Contract Name**: `crates.io:andromeda-socket-astroport`
- **Type**: Andromeda ADO (Andromeda Decentralized Object)

## Installation & Setup

### Instantiation

```rust
use andromeda_socket::astroport::InstantiateMsg;
use andromeda_std::amp::AndrAddr;

let instantiate_msg = InstantiateMsg {
    // Optional: Custom swap router (defaults to "/lib/astroport/router")
    swap_router: Some(AndrAddr::from_string("neutron1...")),
    // Optional: Custom factory address (defaults to "/lib/astroport/factory")
    factory: Some(AndrAddr::from_string("neutron1...")),
    // Standard ADO fields
    kernel_address: "neutron1...".to_string(),
    owner: Some("neutron1...".to_string()),
};
```

## Message Types

### Execute Messages

#### 1. SwapAndForward (Native Tokens)

Swap native tokens to another asset and forward to a recipient.

```rust
use andromeda_socket::astroport::ExecuteMsg;
use andromeda_std::{amp::Recipient, common::denom::Asset};
use cosmwasm_std::{Uint128, Decimal};

let msg = ExecuteMsg::SwapAndForward {
    to_asset: Asset::NativeToken("uatom".to_string()),
    recipient: Some(Recipient::new("neutron1...", None)),
    max_spread: Some(Decimal::percent(1)), // 1% slippage tolerance
    minimum_receive: Some(Uint128::new(1000000)),
    operations: None, // Auto-determined by Astroport
};

// Send with native coins attached
let coins = vec![cosmwasm_std::Coin {
    denom: "untrn".to_string(),
    amount: Uint128::new(1000000),
}];
```

#### 2. CW20 Token Swaps

For CW20 tokens, use the `Receive` hook pattern:

```rust
use cw20::Cw20ExecuteMsg;
use andromeda_socket::astroport::Cw20HookMsg;

// First, send CW20 tokens to the contract with hook message
let cw20_msg = Cw20ExecuteMsg::Send {
    contract: "socket_contract_address".to_string(),
    amount: Uint128::new(1000000),
    msg: to_json_binary(&Cw20HookMsg::SwapAndForward {
        to_asset: Asset::NativeToken("untrn".to_string()),
        recipient: Some(Recipient::new("neutron1...", None)),
        max_spread: Some(Decimal::percent(1)),
        minimum_receive: Some(Uint128::new(900000)),
        operations: None,
    })?,
};
```

#### 3. Create Trading Pair

```rust
use andromeda_socket::astroport::{ExecuteMsg, PairType, AssetInfo};

let msg = ExecuteMsg::CreatePair {
    pair_type: PairType::Xyk {}, // or PairType::Stable {} for stable pairs
    asset_infos: vec![
        AssetInfo::NativeToken { 
            denom: "untrn".to_string() 
        },
        AssetInfo::Token { 
            contract_addr: Addr::unchecked("neutron1...") // CW20 token
        },
    ],
    init_params: None, // Optional for custom pair types
};
```

### 2. Provide Liquidity

The `ProvideLiquidity` operation adds tokens to an existing trading pair to earn trading fees and LP tokens.

#### Parameters:
- **`assets`**: Array of exactly 2 `AssetEntry` objects representing the tokens to deposit
- **`slippage_tolerance`**: Maximum acceptable price movement during transaction (e.g., 0.01 = 1%)
- **`auto_stake`**: Whether to automatically stake LP tokens in Astroport's Generator for additional rewards
- **`receiver`**: Optional address to receive LP tokens (defaults to sender)
- **`pair_address`**: The address of the existing trading pair contract

#### Process:
1. **Native tokens**: Socket receives coins attached to the transaction
2. **CW20 tokens**: Socket transfers tokens from sender and sets allowances for the pair contract
3. Socket calls the pair contract's `ProvideLiquidity` function
4. Pair contract mints LP tokens proportional to the liquidity added
5. LP tokens are sent to the receiver address

#### Token Handling:
- **Native tokens**: Must be sent as transaction funds (`coins`)
- **CW20 tokens**: 
  - Socket executes `TransferFrom` to move tokens from sender
  - Socket sets `IncreaseAllowance` for the pair contract
  - Pair contract uses allowance to deposit tokens

#### Important Considerations:
- **Asset ratio**: Must match current pool ratio or provide excess that gets refunded
- **Minimum liquidity**: First liquidity provider must deposit minimum amounts
- **Price impact**: Large deposits may affect token prices
- **Auto-staking**: Enables earning additional ASTRO rewards if available

```rust
use andromeda_socket::astroport::{ExecuteMsg, AssetEntry, AssetInfo};

let msg = ExecuteMsg::ProvideLiquidity {
    assets: vec![
        AssetEntry {
            info: AssetInfo::NativeToken { 
                denom: "untrn".to_string() 
            },
            amount: Uint128::new(1000000),
        },
        AssetEntry {
            info: AssetInfo::Token { 
                contract_addr: Addr::unchecked("neutron1...")
            },
            amount: Uint128::new(1000000),
        },
    ],
    slippage_tolerance: Some(Decimal::percent(1)),
    auto_stake: Some(false),
    receiver: None, // Defaults to sender
    pair_address: AndrAddr::from_string("neutron1..."),
};
```

### 3. Create Pair and Provide Liquidity (Atomic Operation)

This operation combines pair creation and initial liquidity provision in a single transaction, ensuring atomicity.

#### Parameters:
Combines parameters from both `CreatePair` and `ProvideLiquidity`:
- **Pair creation**: `pair_type`, `asset_infos`, `init_params`
- **Liquidity provision**: `assets`, `slippage_tolerance`, `auto_stake`, `receiver`

#### Process:
1. **Phase 1 - Create Pair**:
   - Socket calls factory to create new pair
   - Factory instantiates pair contract
   - Socket extracts pair address from events
   
2. **Phase 2 - Provide Liquidity**:
   - Socket handles token transfers (native and CW20)
   - Socket calls new pair's `ProvideLiquidity` function
   - Initial LP tokens are minted to receiver

#### Advantages:
- **Atomicity**: Either both operations succeed or both fail
- **Gas efficiency**: Single transaction instead of two separate calls
- **Immediate liquidity**: Pair is created with initial trading liquidity
- **First LP advantage**: Becomes the first liquidity provider with initial price setting

#### State Management:
The contract uses temporary state storage (`LIQUIDITY_PROVISION_STATE`) to pass liquidity parameters between the pair creation reply and execution phases.

```rust
let msg = ExecuteMsg::CreatePairAndProvideLiquidity {
    pair_type: PairType::Xyk {},
    asset_infos: vec![
        AssetInfo::NativeToken { denom: "untrn".to_string() },
        AssetInfo::Token { contract_addr: Addr::unchecked("neutron1...") },
    ],
    init_params: None,
    assets: vec![
        AssetEntry {
            info: AssetInfo::NativeToken { denom: "untrn".to_string() },
            amount: Uint128::new(1000000),
        },
        AssetEntry {
            info: AssetInfo::Token { contract_addr: Addr::unchecked("neutron1...") },
            amount: Uint128::new(1000000),
        },
    ],
    slippage_tolerance: Some(Decimal::percent(1)),
    auto_stake: Some(false),
    receiver: None,
};
```

### 4. Withdraw Liquidity

The `WithdrawLiquidity` operation removes liquidity from a trading pair by burning LP tokens and receiving underlying assets.

#### Parameters:
- **`pair_address`**: The address of the pair contract to withdraw from
- **LP tokens**: Must be sent as transaction funds (coins)

#### Process:
1. User sends LP tokens to socket contract as transaction funds
2. Socket forwards LP tokens to the pair contract's withdrawal function
3. Pair contract burns LP tokens and calculates proportional asset amounts
4. Pair contract returns underlying assets to socket
5. Socket automatically forwards received assets back to original sender

#### Asset Recovery:
The contract parses withdrawal events to identify returned assets and automatically transfers them back to the user:
- **Native tokens**: Direct bank transfers
- **CW20 tokens**: Contract-to-contract transfers

#### Important Notes:
- **Proportional withdrawal**: Receive assets proportional to LP token percentage of total supply
- **Current price**: Assets received at current pool ratio, not original deposit ratio
- **Automatic forwarding**: Socket automatically returns all withdrawn assets to sender
- **Event parsing**: Contract reads Astroport events to determine refund amounts

#### Example LP Token Format:
```
denom: "factory/neutron1.../lp_token"
amount: 500000
```

#### State Tracking:
The contract temporarily stores the original sender address (`WITHDRAWAL_STATE`) to ensure assets are returned to the correct user after the withdrawal completes.

```rust
let msg = ExecuteMsg::WithdrawLiquidity {
    pair_address: AndrAddr::from_string("neutron1..."),
};

// Send LP tokens as coins
let lp_coins = vec![cosmwasm_std::Coin {
    denom: "factory/neutron1.../lp_token".to_string(),
    amount: Uint128::new(500000),
}];
```

### Query Messages

#### Simulate Swap Operation

```rust
use andromeda_socket::astroport::{QueryMsg, SwapOperation};

let query_msg = QueryMsg::SimulateSwapOperation {
    offer_amount: Uint128::new(1000000),
    operations: vec![
        SwapOperation {
            offer_asset_info: Asset::NativeToken("untrn".to_string()),
            ask_asset_info: Asset::NativeToken("uatom".to_string()),
        }
    ],
};

// Response: SimulateSwapOperationResponse { amount: Uint128 }
```

## Detailed Liquidity Operations

### 1. Create Trading Pair

The `CreatePair` operation establishes a new trading pair on Astroport through the factory contract.

#### Parameters:
- **`pair_type`**: The type of AMM curve to use
  - `Xyk {}`: Standard constant product AMM (x*y=k)
  - `Stable {}`: Stable swap curve for assets with similar values
  - `Custom(String)`: Custom pair implementation
- **`asset_infos`**: Array of exactly 2 assets that will form the trading pair
- **`init_params`**: Optional binary-encoded parameters for custom pair types

#### Process:
1. The socket contract calls the Astroport factory with the pair creation parameters
2. Factory instantiates a new pair contract
3. The new pair contract address is extracted from the instantiation events
4. Socket contract emits success event with the new pair address

#### Important Notes:
- **No funds required**: Pair creation doesn't require sending tokens
- **Pair uniqueness**: Cannot create duplicate pairs with same assets and type
- **Asset ordering**: Astroport automatically orders assets consistently
- **Factory permissions**: Must use the configured factory address

#### Example Response Events:
```
- action: "create_pair_success"
- pair_address: "neutron1..."
- pair_type: "Xyk"
- asset_infos: "[NativeToken{untrn}, Token{neutron1...}]"
```

## Asset Types

The contract supports two main asset types:

### Native Tokens
```rust
Asset::NativeToken("untrn".to_string())
```

### CW20 Tokens
```rust
Asset::Cw20Token(AndrAddr::from_string("neutron1..."))
```

## Important Notes

1. **Native Token Swaps**: Send coins directly with the transaction
2. **CW20 Token Swaps**: Use the `Receive` hook pattern or approve the contract first
3. **Slippage**: `max_spread` represents slippage tolerance (e.g., 0.01 = 1%)
4. **Auto Stake**: When providing liquidity, LP tokens can be auto-staked in Astroport's Generator
5. **Pair Types**: 
   - `Xyk`: Standard AMM pairs (x*y=k)
   - `Stable`: Stable coin pairs with lower slippage
   - `Custom`: For specialized pair implementations

## Error Handling

Common errors and their meanings:

- `InvalidAsset`: Asset format is incorrect or missing
- `InsufficientFunds`: Not enough tokens for the operation
- `SlippageExceeded`: Swap would result in too much slippage
- `PairNotFound`: Trading pair doesn't exist
- `Unauthorized`: Sender doesn't have permission for restricted operations

## Testing

For testing integrations, use the test networks and ensure you have:

1. Test tokens in your wallet
2. Proper contract addresses for the target network
3. Sufficient gas for complex operations (pair creation, liquidity provision)

## Support

For questions and support:
- [Andromeda Documentation](https://docs.andromedaprotocol.io/)
- [Astroport Documentation](https://docs.astroport.fi/)
- [GitHub Issues](https://github.com/andromedaprotocol/andromeda-core/issues) 