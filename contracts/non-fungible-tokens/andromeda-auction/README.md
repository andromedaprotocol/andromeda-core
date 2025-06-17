# Andromeda Auction ADO

## Introduction

The Andromeda Auction ADO is a comprehensive NFT auction platform that enables users to create, manage, and participate in competitive bidding for non-fungible tokens. This contract provides complete auction lifecycle management including time-based bidding, instant purchase options, whitelist controls, and automatic fund transfers with built-in royalty and tax handling through the Andromeda ecosystem.

<b>Ado_type:</b> auction

## Why Auction ADO

The Auction ADO serves as a critical component for NFT marketplaces and trading platforms requiring:

- **NFT Trading Platforms**: Create competitive auction marketplaces for digital collectibles
- **Art Galleries**: Enable time-limited bidding for digital art and unique creations
- **Gaming Assets**: Auction rare in-game items, characters, and virtual real estate
- **Collectibles Markets**: Facilitate trading of limited edition digital collectibles
- **Community Sales**: Enable decentralized auction events for creator communities
- **Investment Opportunities**: Provide price discovery mechanisms for valuable NFTs
- **Instant Liquidity**: Offer buy-now options for immediate NFT purchases
- **Whitelisted Sales**: Control auction participation through address whitelisting
- **Fair Price Discovery**: Allow market-driven pricing through competitive bidding
- **Automated Settlement**: Handle complex payment flows with automatic fund distribution

The ADO supports both native tokens and CW20 tokens for bidding, enabling flexible payment options across the Cosmos ecosystem.

## Key Features

### **Auction Management**
- **Time-controlled auctions**: Configurable start and end times with automatic enforcement
- **Buy-now pricing**: Optional instant purchase options to bypass bidding
- **Auction updates**: Modify auction parameters before bidding begins
- **Cancellation support**: Cancel auctions and refund active bids
- **Multiple auction tracking**: Support multiple auctions per NFT over time

### **Bidding System**
- **Competitive bidding**: Place bids that automatically outbid previous offers
- **Minimum bid enforcement**: Set minimum starting bid amounts
- **Bid raise requirements**: Configure minimum increase amounts between bids
- **Automatic refunds**: Previous bids automatically returned when outbid
- **Bid history tracking**: Complete audit trail of all bidding activity

### **Payment Flexibility**
- **Native token support**: Accept blockchain native tokens for bidding
- **CW20 token integration**: Support custom tokens through CW20 standard
- **Mixed payment options**: Configure auctions for specific token types
- **Automatic validation**: Verify payment tokens against authorized lists

### **Access Control**
- **Whitelisting system**: Restrict auction participation to approved addresses
- **Token authorization**: Control which NFT contracts can create auctions
- **CW20 authorization**: Manage which tokens are accepted for bidding
- **Permission-based actions**: Secure sensitive operations through permission system

### **Advanced Features**
- **Claim mechanism**: Secure NFT and payment transfer after auction completion
- **Tax integration**: Automatic deduction of fees, royalties, and taxes
- **Recipient flexibility**: Send auction proceeds to custom recipients
- **Multi-format queries**: Comprehensive query system for auction data
- **State tracking**: Monitor auction status, claims, and cancellations

## Time Management

### **Auction Scheduling**
The contract provides flexible auction timing with automatic enforcement:
- **Immediate start**: Begin auctions immediately upon NFT deposit
- **Future scheduling**: Set specific start times for planned auctions
- **Duration control**: Configure auction end times for optimal bidding periods
- **Time validation**: Prevent invalid time configurations and past scheduling

### **Bidding Windows**
Precise control over when bidding is allowed:
- **Start time enforcement**: Prevent early bidding before auction begins
- **End time validation**: Automatically close bidding when time expires
- **Current time checking**: Real-time validation against blockchain timestamps
- **Grace period handling**: Secure handling of last-minute bids

## InstantiateMsg

```rust
pub struct InstantiateMsg {
    pub authorized_token_addresses: Option<Vec<AndrAddr>>,
    pub authorized_cw20_addresses: Option<Vec<AndrAddr>>,
}
```

```json
{
    "authorized_token_addresses": [
        "andr1nft_contract_address...",
        "andr1another_nft_contract..."
    ],
    "authorized_cw20_addresses": [
        "andr1token_contract_address...",
        "andr1another_token_contract..."
    ]
}
```

**Parameters**:
- **authorized_token_addresses**: Optional list of NFT contracts authorized to start auctions
- **authorized_cw20_addresses**: Optional list of CW20 token contracts accepted for bidding

**Authorization Model**:
- If `authorized_token_addresses` is provided, only listed NFT contracts can create auctions
- If `authorized_cw20_addresses` is provided, only listed tokens are accepted for CW20 bidding
- Empty lists allow all contracts (permissionless mode)
- Authorization can be managed post-deployment through execute messages

## ExecuteMsg

### ReceiveNft (CW721 Hook)
Receives an NFT and starts an auction with the specified parameters.

```rust
pub enum Cw721HookMsg {
    StartAuction {
        start_time: Option<Expiry>,
        end_time: Expiry,
        coin_denom: Asset,
        buy_now_price: Option<Uint128>,
        min_bid: Option<Uint128>,
        min_raise: Option<Uint128>,
        whitelist: Option<Vec<Addr>>,
        recipient: Option<Recipient>,
    },
}
```

```json
{
    "send_nft": {
        "contract": "andr1auction_contract...",
        "token_id": "unique_token_123",
        "msg": "eyJzdGFydF9hdWN0aW9uIjp7InN0YXJ0X3RpbWUiOm51bGwsImVuZF90aW1lIjp7ImF0X3RpbWUiOiIxNjcyNjE3NjAwMDAwIn0sImNvaW5fZGVub20iOiJ1YW5kciIsImJ1eV9ub3dfcHJpY2UiOiIxMDAwMDAwMDAwIiwibWluX2JpZCI6IjEwMDAwMDAwMCIsIm1pbl9yYWlzZSI6IjUwMDAwMDAwIn19"
    }
}
```

**Parameters**:
- **start_time**: Optional auction start time (defaults to immediate)
- **end_time**: Required auction expiration time
- **coin_denom**: Token denomination for bidding (native or CW20)
- **buy_now_price**: Optional instant purchase price
- **min_bid**: Optional minimum starting bid amount
- **min_raise**: Optional minimum bid increase requirement
- **whitelist**: Optional list of addresses allowed to bid
- **recipient**: Optional custom recipient for auction proceeds

**Validation**:
- NFT contract must be authorized (if authorization is enabled)
- End time must be in the future and after start time
- Buy-now price must be greater than minimum bid (if both specified)
- Token denomination must be valid and authorized
- Whitelist addresses must be valid

### Receive (CW20 Hook)
Receives CW20 tokens to place bids or buy NFTs instantly.

```rust
pub enum Cw20HookMsg {
    PlaceBid {
        token_id: String,
        token_address: String,
    },
    BuyNow {
        token_id: String,
        token_address: String,
    },
}
```

```json
{
    "send": {
        "contract": "andr1auction_contract...",
        "amount": "1000000000",
        "msg": "eyJwbGFjZV9iaWQiOnsidG9rZW5faWQiOiJ1bmlxdWVfdG9rZW5fMTIzIiwidG9rZW5fYWRkcmVzcyI6ImFuZHIxbmZ0X2NvbnRyYWN0Li4uIn19"
    }
}
```

**Requirements**:
- CW20 token must be authorized for the auction
- Token amount must meet minimum bid requirements
- Auction must be active and within bidding window
- Bidder cannot be the current highest bidder or auction owner

### PlaceBid
Places a bid using native tokens.

```rust
PlaceBid {
    token_id: String,
    token_address: String,
}
```

```json
{
    "place_bid": {
        "token_id": "unique_token_123",
        "token_address": "andr1nft_contract..."
    }
}
```

**Usage**: Send native tokens as funds with this message. The contract will:
1. Validate the auction is active and accepting bids
2. Check bid amount against minimum requirements
3. Refund the previous highest bidder
4. Update auction state with new highest bid
5. Record bid in the auction history

**Requirements**:
- Must send exactly one coin denomination as funds
- Coin denomination must match the auction's specified token
- Bid amount must exceed current highest bid by minimum raise amount
- Bidder cannot be the auction owner or current highest bidder

### BuyNow
Instantly purchases an NFT at the buy-now price.

```rust
BuyNow {
    token_id: String,
    token_address: String,
}
```

```json
{
    "buy_now": {
        "token_id": "unique_token_123",
        "token_address": "andr1nft_contract..."
    }
}
```

**Requirements**:
- Auction must have a configured buy-now price
- Must send exact buy-now amount as payment
- Auction must be active and not yet ended
- Payment automatically refunds any existing highest bidder

### UpdateAuction
Updates auction parameters before bidding begins.

```rust
UpdateAuction {
    token_id: String,
    token_address: String,
    start_time: Option<Expiry>,
    end_time: Expiry,
    coin_denom: Asset,
    whitelist: Option<Vec<Addr>>,
    min_bid: Option<Uint128>,
    min_raise: Option<Uint128>,
    buy_now_price: Option<Uint128>,
    recipient: Option<Recipient>,
}
```

```json
{
    "update_auction": {
        "token_id": "unique_token_123",
        "token_address": "andr1nft_contract...",
        "start_time": {
            "at_time": "1672617600000"
        },
        "end_time": {
            "at_time": "1672704000000"
        },
        "coin_denom": "uandr",
        "min_bid": "200000000",
        "min_raise": "100000000",
        "buy_now_price": "2000000000",
        "whitelist": ["andr1bidder1...", "andr1bidder2..."]
    }
}
```

**Authorization**: Only the auction owner can update parameters
**Restrictions**: Can only update before auction starts
**Validation**: Same validation rules as auction creation apply

### CancelAuction
Cancels an active auction and refunds any existing bids.

```rust
CancelAuction {
    token_id: String,
    token_address: String,
}
```

```json
{
    "cancel_auction": {
        "token_id": "unique_token_123",
        "token_address": "andr1nft_contract..."
    }
}
```

**Authorization**: Only the auction owner can cancel
**Requirements**:
- Auction must not have ended
- Auction must not have been purchased via buy-now
- Automatically refunds highest bidder (if any)
- Returns NFT to original owner

### Claim
Claims the NFT and distributes payments after auction completion.

```rust
Claim {
    token_id: String,
    token_address: String,
}
```

```json
{
    "claim": {
        "token_id": "unique_token_123",
        "token_address": "andr1nft_contract..."
    }
}
```

**Trigger Conditions**:
- Auction has ended (past end time)
- NFT has not been previously claimed
- Can be called by anyone (permissionless)

**Claim Process**:
1. **No bids**: Returns NFT to original owner
2. **Successful auction**: 
   - Transfers NFT to highest bidder
   - Processes taxes and royalties through Andromeda system
   - Sends remaining proceeds to specified recipient or owner

### AuthorizeContract / DeauthorizeContract
Manages authorization for NFT contracts and CW20 tokens.

```rust
AuthorizeContract {
    action: PermissionAction,
    addr: AndrAddr,
    expiration: Option<Expiry>,
}

DeauthorizeContract {
    action: PermissionAction,
    addr: AndrAddr,
}
```

```json
{
    "authorize_contract": {
        "action": "send_nft",
        "addr": "andr1new_nft_contract...",
        "expiration": {
            "at_time": "1672704000000"
        }
    }
}
```

**Actions**:
- `"send_nft"`: Authorize NFT contracts to create auctions
- `"send_cw20"`: Authorize CW20 tokens for bidding

## QueryMsg

### LatestAuctionState
Returns the most recent auction information for a specific NFT.

```rust
#[returns(AuctionStateResponse)]
LatestAuctionState {
    token_id: String,
    token_address: String,
}
```

```json
{
    "latest_auction_state": {
        "token_id": "unique_token_123",
        "token_address": "andr1nft_contract..."
    }
}
```

**Response:**
```json
{
    "start_time": "1672617600000",
    "end_time": "1672704000000",
    "high_bidder_addr": "andr1highest_bidder...",
    "high_bidder_amount": "1500000000",
    "auction_id": "42",
    "coin_denom": "uandr",
    "uses_cw20": false,
    "whitelist": ["andr1bidder1...", "andr1bidder2..."],
    "min_bid": "100000000",
    "min_raise": "50000000",
    "is_cancelled": false,
    "owner": "andr1nft_owner...",
    "recipient": {
        "address": "andr1proceeds_recipient...",
        "msg": null
    }
}
```

### AuctionState
Returns auction information for a specific auction ID.

```rust
#[returns(AuctionStateResponse)]
AuctionState { auction_id: Uint128 }
```

```json
{
    "auction_state": {
        "auction_id": "42"
    }
}
```

### AuctionIds
Returns all auction IDs for a specific NFT.

```rust
#[returns(AuctionIdsResponse)]
AuctionIds {
    token_id: String,
    token_address: String,
}
```

```json
{
    "auction_ids": {
        "token_id": "unique_token_123",
        "token_address": "andr1nft_contract..."
    }
}
```

**Response:**
```json
{
    "auction_ids": ["35", "38", "42"]
}
```

### AuctionInfosForAddress
Returns all auction information for a specific NFT contract.

```rust
#[returns(Vec<AuctionInfo>)]
AuctionInfosForAddress {
    token_address: String,
    start_after: Option<String>,
    limit: Option<u64>,
}
```

```json
{
    "auction_infos_for_address": {
        "token_address": "andr1nft_contract...",
        "start_after": "token_50",
        "limit": 10
    }
}
```

### Bids
Returns bidding history for a specific auction.

```rust
#[returns(BidsResponse)]
Bids {
    auction_id: Uint128,
    start_after: Option<u64>,
    limit: Option<u64>,
    order_by: Option<OrderBy>,
}
```

```json
{
    "bids": {
        "auction_id": "42",
        "start_after": 5,
        "limit": 10,
        "order_by": "desc"
    }
}
```

**Response:**
```json
{
    "bids": [
        {
            "bidder": "andr1highest_bidder...",
            "amount": "1500000000",
            "timestamp": "1672650000000"
        },
        {
            "bidder": "andr1previous_bidder...",
            "amount": "1200000000",
            "timestamp": "1672645000000"
        }
    ]
}
```

### Status Queries
Check auction status and state.

```rust
#[returns(IsCancelledResponse)]
IsCancelled {
    token_id: String,
    token_address: String,
}

#[returns(IsClosedResponse)]
IsClosed {
    token_id: String,
    token_address: String,
}

#[returns(IsClaimedResponse)]
IsClaimed {
    token_id: String,
    token_address: String,
}
```

### AuthorizedAddresses
Returns authorized contracts for specific actions.

```rust
#[returns(AuthorizedAddressesResponse)]
AuthorizedAddresses {
    action: PermissionAction,
    start_after: Option<String>,
    limit: Option<u32>,
    order_by: Option<OrderBy>,
}
```

## Usage Examples

### NFT Art Gallery Auction
```json
{
    "send_nft": {
        "contract": "andr1auction_contract...",
        "token_id": "genesis_art_001",
        "msg": "eyJzdGFydF9hdWN0aW9uIjp7InN0YXJ0X3RpbWUiOm51bGwsImVuZF90aW1lIjp7ImF0X3RpbWUiOiIxNjcyNzA0MDAwMDAwIn0sImNvaW5fZGVub20iOiJ1YW5kciIsIm1pbl9iaWQiOiIxMDAwMDAwMDAwIiwibWluX3JhaXNlIjoiMTAwMDAwMDAwIn19"
    }
}
```
_7-day auction with 1,000 ANDR minimum bid and 100 ANDR minimum raise._

### Gaming Asset Auction with Buy-Now
```json
{
    "send_nft": {
        "contract": "andr1auction_contract...",
        "token_id": "legendary_sword_999",
        "msg": "eyJzdGFydF9hdWN0aW9uIjp7InN0YXJ0X3RpbWUiOm51bGwsImVuZF90aW1lIjp7ImF0X3RpbWUiOiIxNjcyNjE3NjAwMDAwIn0sImNvaW5fZGVub20iOiJ1Z2FtZSIsImJ1eV9ub3dfcHJpY2UiOiI1MDAwMDAwMDAwIiwibWluX2JpZCI6IjEwMDAwMDAwMDAiLCJtaW5fcmFpc2UiOiI1MDAwMDAwMCJ9"
    }
}
```
_24-hour auction with buy-now option for instant purchase._

### Exclusive Collectible with Whitelist
```json
{
    "send_nft": {
        "contract": "andr1auction_contract...",
        "token_id": "exclusive_collectible_1",
        "msg": "eyJzdGFydF9hdWN0aW9uIjp7InN0YXJ0X3RpbWUiOnsiaW5fc2Vjb25kcyI6ODY0MDB9LCJlbmRfdGltZSI6eyJpbl9zZWNvbmRzIjo2MDQ4MDB9LCJjb2luX2Rlbm9tIjoidWFuZHIiLCJtaW5fYmlkIjoiNTAwMDAwMDAwIiwid2hpdGVsaXN0IjpbImFuZHIxdmlwMSIsImFuZHIxdmlwMiIsImFuZHIxdmlwMyJdfQ=="
    }
}
```
_VIP-only auction starting in 24 hours, ending in 7 days._

### CW20 Token Auction
```json
{
    "send_nft": {
        "contract": "andr1auction_contract...",
        "token_id": "premium_nft_456",
        "msg": "eyJzdGFydF9hdWN0aW9uIjp7InN0YXJ0X3RpbWUiOm51bGwsImVuZF90aW1lIjp7ImF0X3RpbWUiOiIxNjcyNjE3NjAwMDAwIn0sImNvaW5fZGVub20iOiJhbmRyMXByZW1pdW1fdG9rZW4uLi4iLCJtaW5fYmlkIjoiMTAwMDAwMDAwMCIsIm1pbl9yYWlzZSI6IjUwMDAwMDAwIn19"
    }
}
```
_Accept custom CW20 tokens for bidding on premium NFTs._

## Operational Examples

### Place Native Token Bid
```json
{
    "place_bid": {
        "token_id": "auction_item_789",
        "token_address": "andr1nft_contract..."
    }
}
```
_Send native tokens as funds with this message._

### Place CW20 Token Bid
```json
{
    "send": {
        "contract": "andr1auction_contract...",
        "amount": "2000000000",
        "msg": "eyJwbGFjZV9iaWQiOnsidG9rZW5faWQiOiJhdWN0aW9uX2l0ZW1fNzg5IiwidG9rZW5fYWRkcmVzcyI6ImFuZHIxbmZ0X2NvbnRyYWN0Li4uIn19"
    }
}
```

### Buy Now with Native Tokens
```json
{
    "buy_now": {
        "token_id": "instant_buy_item",
        "token_address": "andr1nft_contract..."
    }
}
```

### Update Auction Before Start
```json
{
    "update_auction": {
        "token_id": "auction_item_789",
        "token_address": "andr1nft_contract...",
        "end_time": {
            "at_time": "1672790400000"
        },
        "min_bid": "2000000000",
        "buy_now_price": "10000000000"
    }
}
```

### Cancel Active Auction
```json
{
    "cancel_auction": {
        "token_id": "auction_item_789",
        "token_address": "andr1nft_contract..."
    }
}
```

### Claim After Auction End
```json
{
    "claim": {
        "token_id": "auction_item_789",
        "token_address": "andr1nft_contract..."
    }
}
```

## Integration Patterns

### With App Contract
The Auction ADO can be integrated into App contracts for marketplace functionality:

```json
{
    "components": [
        {
            "name": "nft_auction_house",
            "ado_type": "auction",
            "component_type": {
                "new": {
                    "authorized_token_addresses": [
                        "./nft_collection"
                    ],
                    "authorized_cw20_addresses": [
                        "./platform_token"
                    ]
                }
            }
        }
    ]
}
```

### NFT Marketplace
For comprehensive marketplace solutions:

1. **Deploy auction contract** with authorized NFT collections
2. **Configure payment tokens** for bidding flexibility
3. **Implement UI integration** for auction creation and bidding
4. **Monitor auction events** for real-time marketplace updates

### Gaming Ecosystem
For in-game asset trading:

1. **Authorize game NFT contracts** to create auctions
2. **Configure game tokens** for in-ecosystem trading
3. **Implement auction triggers** from game events
4. **Handle claim events** to update game state

### Art Gallery Platform
For digital art auctions:

1. **Whitelist verified artists** for auction creation
2. **Configure premium tokens** for high-value art
3. **Implement bidding interfaces** with real-time updates
4. **Handle royalty distribution** through Andromeda tax system

## Advanced Features

### **Tax and Royalty Integration**
- **Automatic deduction**: Taxes and royalties automatically calculated and deducted
- **Multi-recipient distribution**: Supports complex payment routing through AMP
- **Configurable rates**: Leverages Andromeda's flexible tax system
- **Creator royalties**: Ensures creators receive ongoing revenue from secondary sales

### **Whitelist Management**
- **Per-auction control**: Each auction can have custom whitelist requirements
- **Address validation**: Comprehensive validation of whitelisted addresses
- **Permission checking**: Real-time validation during bidding process
- **Flexible access**: Combine whitelisting with other authorization mechanisms

### **Multi-Token Support**
- **Native tokens**: Support for blockchain native tokens (e.g., ANDR, JUNO)
- **CW20 integration**: Custom token support through CW20 standard
- **Authorization control**: Restrict which tokens are accepted per auction
- **Automatic validation**: Verify token types and amounts during bidding

### **Auction History**
- **Complete audit trail**: Track all bids, updates, and state changes
- **Multiple auctions**: Support multiple auctions for the same NFT over time
- **Bid tracking**: Detailed history of all bidding activity
- **Status monitoring**: Real-time status queries for auction state

## Security Features

### **Time Security**
- **Timestamp validation**: Secure validation against blockchain time
- **Start time enforcement**: Prevent early bidding before auction begins
- **End time validation**: Automatic closure when auction expires
- **Update restrictions**: Prevent parameter changes after bidding starts

### **Payment Security**
- **Amount validation**: Comprehensive validation of bid amounts
- **Token verification**: Strict validation of payment tokens
- **Refund automation**: Automatic refund of outbid amounts
- **Balance protection**: Secure handling of contract fund balances

### **Access Control**
- **Owner restrictions**: Only NFT owners can create and manage auctions
- **Bidder validation**: Prevent auction owners from bidding on their own auctions
- **Authorization checking**: Comprehensive permission validation for all operations
- **Whitelist enforcement**: Secure enforcement of bidding restrictions

### **State Protection**
- **Atomic operations**: Ensure auction state consistency across all operations
- **Invalid state prevention**: Comprehensive validation prevents invalid auction states
- **Cancellation security**: Secure handling of auction cancellations and refunds
- **Claim validation**: Prevent double claims and invalid settlement

## Important Notes

- **Single token per auction**: Each auction accepts only one token denomination
- **Automatic refunds**: Previous bids are automatically refunded when outbid
- **Tax integration**: Utilizes Andromeda's tax system for fee and royalty distribution
- **Permission inheritance**: Leverages Andromeda's permission system for access control
- **AMP compatibility**: Full integration with Andromeda Messaging Protocol
- **State consistency**: All operations maintain consistent auction state
- **Time enforcement**: Auction timing is strictly enforced by blockchain time
- **Gas optimization**: Efficient gas usage through optimized storage and operations

## Common Workflow

### 1. **Deploy Auction Contract**
```json
{
    "authorized_token_addresses": [
        "andr1my_nft_collection...",
        "andr1partner_collection..."
    ],
    "authorized_cw20_addresses": [
        "andr1platform_token...",
        "andr1premium_token..."
    ]
}
```

### 2. **Create Auction by Sending NFT**
```json
{
    "send_nft": {
        "contract": "andr1auction_contract...",
        "token_id": "my_nft_1",
        "msg": "eyJzdGFydF9hdWN0aW9uIjp7ImVuZF90aW1lIjp7ImF0X3RpbWUiOiIxNjcyNzA0MDAwMDAwIn0sImNvaW5fZGVub20iOiJ1YW5kciIsIm1pbl9iaWQiOiIxMDAwMDAwMDAwIn19"
    }
}
```

### 3. **Monitor Auction Progress**
```json
{
    "latest_auction_state": {
        "token_id": "my_nft_1",
        "token_address": "andr1my_nft_collection..."
    }
}
```

### 4. **Place Competitive Bids**
```json
{
    "place_bid": {
        "token_id": "my_nft_1",
        "token_address": "andr1my_nft_collection..."
    }
}
```
_Send funds as native tokens or use CW20 send message._

### 5. **Claim After Auction Completion**
```json
{
    "claim": {
        "token_id": "my_nft_1",
        "token_address": "andr1my_nft_collection..."
    }
}
```

The Auction ADO provides a comprehensive solution for NFT auctions with advanced features, security controls, and seamless integration with the broader Andromeda ecosystem for professional marketplace applications.