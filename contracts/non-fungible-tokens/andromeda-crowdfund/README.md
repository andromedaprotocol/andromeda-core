# Andromeda Crowdfund ADO

## Introduction

The Andromeda Crowdfund ADO is a comprehensive crowdfunding platform that enables creators to raise funds for projects through NFT-based campaigns. This contract manages multi-tier funding campaigns where supporters receive unique NFTs as rewards for their contributions. The platform provides complete campaign lifecycle management including tier-based rewards, soft/hard caps, presale options, and automatic fund distribution with built-in success/failure handling.

<b>Ado_type:</b> crowdfund

## Why Crowdfund ADO

The Crowdfund ADO serves as an essential tool for modern fundraising campaigns requiring:

- **Creative Project Funding**: Enable artists, musicians, and creators to fund projects through community support
- **Product Development**: Raise capital for new product development with early adopter rewards
- **Community Building**: Create engaged communities around projects through exclusive NFT rewards
- **Decentralized Fundraising**: Bypass traditional crowdfunding platforms with blockchain-native solutions
- **Transparent Funding**: Provide complete transparency in fund collection and distribution
- **Tier-Based Rewards**: Offer multiple support levels with corresponding NFT rewards
- **Risk Management**: Built-in safeguards with soft/hard caps and automatic refunds
- **Presale Management**: Enable exclusive presale access for early supporters
- **Smart Automation**: Automatic campaign management and reward distribution
- **Global Access**: Enable worldwide participation without geographical restrictions

The ADO supports both native tokens and CW20 tokens for contributions, providing flexibility across different blockchain ecosystems.

## Key Features

### **Campaign Management**
- **Multi-tier structure**: Create campaigns with multiple reward tiers and pricing levels
- **Flexible timing**: Configure start and end times with optional presale periods
- **Stage-based lifecycle**: Clear campaign stages from setup to completion
- **Automatic transitions**: Smart contract handles campaign state transitions
- **Admin controls**: Campaign owners can manage tiers and campaign settings

### **Funding Controls**
- **Soft cap goals**: Minimum funding requirements for campaign success
- **Hard cap limits**: Maximum funding limits to prevent oversubscription
- **Automatic refunds**: Built-in refund mechanism for failed campaigns
- **Payment flexibility**: Accept both native tokens and CW20 tokens
- **Precise tracking**: Real-time tracking of funding progress and tier sales

### **Tier System**
- **Unlimited tiers**: Create multiple reward tiers with different prices and benefits
- **Tier limits**: Optional quantity limits for exclusive tiers
- **NFT rewards**: Each tier corresponds to unique NFT rewards for supporters
- **Metadata support**: Rich metadata support for NFT rewards
- **Tier management**: Add, update, and remove tiers before campaign launch

### **Presale Features**
- **Early access**: Configure presale orders for exclusive early supporters
- **Presale tracking**: Separate tracking for presale vs public sale contributions
- **Allocation management**: Pre-allocate specific tier quantities for presale participants
- **Seamless transition**: Automatic transition from presale to public campaign

### **Success Handling**
- **Automatic distribution**: Smart contract automatically mints and distributes NFT rewards
- **Fund withdrawal**: Automatic transfer of funds to campaign recipient upon success
- **Claim mechanism**: Supporters claim their NFT rewards after successful campaigns
- **Token integration**: Seamless integration with CW721 NFT contracts

### **Failure Handling**
- **Automatic refunds**: Failed campaigns automatically refund all contributions
- **Clear failure criteria**: Transparent failure conditions based on soft cap and timing
- **Secure refunds**: Safe refund mechanism for both native and CW20 tokens
- **Campaign cleanup**: Proper cleanup of failed campaign data

## Campaign Lifecycle

### **Stage Management**
The campaign progresses through distinct stages with automatic enforcement:

1. **READY**: Campaign setup phase where tiers can be configured
2. **ONGOING**: Active fundraising period accepting contributions
3. **SUCCESS**: Campaign reached soft cap and funds are distributed
4. **FAILED**: Campaign failed to reach soft cap before expiration
5. **DISCARDED**: Campaign was manually cancelled by owner

### **Stage Transitions**
- **READY → ONGOING**: When campaign is started with valid timing
- **ONGOING → SUCCESS**: When soft cap is reached (can be before end time)
- **ONGOING → FAILED**: When end time is reached without meeting soft cap
- **READY/ONGOING → DISCARDED**: When owner manually discards campaign
- **SUCCESS/FAILED → CLAIMED**: When supporters claim rewards or refunds

## InstantiateMsg

```rust
pub struct InstantiateMsg {
    pub campaign_config: CampaignConfig,
    pub tiers: Vec<Tier>,
}

pub struct CampaignConfig {
    pub title: Option<String>,
    pub description: Option<String>,
    pub banner: Option<String>,
    pub url: Option<String>,
    pub token_address: AndrAddr,
    pub denom: Asset,
    pub withdrawal_recipient: Recipient,
    pub soft_cap: Option<Uint128>,
    pub hard_cap: Option<Uint128>,
}

pub struct Tier {
    pub level: Uint64,
    pub label: String,
    pub price: Uint128,
    pub limit: Option<Uint128>,
    pub metadata: TierMetaData,
}
```

```json
{
    "campaign_config": {
        "title": "Revolutionary Art Collection",
        "description": "Funding for groundbreaking digital art series",
        "banner": "https://example.com/banner.jpg",
        "url": "https://example.com/campaign",
        "token_address": "andr1nft_contract_address...",
        "denom": "uandr",
        "withdrawal_recipient": {
            "address": "andr1creator_address...",
            "msg": null
        },
        "soft_cap": "10000000000",
        "hard_cap": "50000000000"
    },
    "tiers": [
        {
            "level": "1",
            "label": "Bronze Supporter",
            "price": "1000000000",
            "limit": "100",
            "metadata": {
                "token_uri": "https://example.com/bronze.json",
                "extension": {}
            }
        },
        {
            "level": "2",
            "label": "Silver Patron",
            "price": "5000000000",
            "limit": "50",
            "metadata": {
                "token_uri": "https://example.com/silver.json",
                "extension": {}
            }
        }
    ]
}
```

**Campaign Config Parameters**:
- **title**: Campaign title (maximum 64 characters)
- **description**: Detailed campaign description
- **banner**: URL to campaign banner image
- **url**: Official website or campaign page URL
- **token_address**: NFT contract address for reward distribution
- **denom**: Token denomination accepted for contributions (native or CW20)
- **withdrawal_recipient**: Address to receive funds upon successful campaign
- **soft_cap**: Minimum funding goal for campaign success
- **hard_cap**: Maximum funding limit to prevent oversubscription

**Tier Parameters**:
- **level**: Unique tier identifier (used for ordering)
- **label**: Human-readable tier name
- **price**: Cost per NFT in this tier
- **limit**: Optional maximum quantity for this tier
- **metadata**: NFT metadata including token URI and extensions

**Validation**:
- Title cannot exceed 64 characters
- Soft cap must be less than hard cap (if both specified)
- All tier prices must be greater than zero
- Tier labels must be non-empty and ≤64 characters
- Token and recipient addresses must be valid

## ExecuteMsg

### AddTier
Adds a new tier to the campaign (only available in READY stage).

```rust
AddTier { tier: Tier }
```

```json
{
    "add_tier": {
        "tier": {
            "level": "3",
            "label": "Gold Patron",
            "price": "10000000000",
            "limit": "25",
            "metadata": {
                "token_uri": "https://example.com/gold.json",
                "extension": {
                    "special_access": true
                }
            }
        }
    }
}
```

**Requirements**:
- Campaign must be in READY stage
- Tier level must be unique
- Price must be greater than zero
- Label must be non-empty and ≤64 characters

### UpdateTier
Updates an existing tier's information (only available in READY stage).

```rust
UpdateTier { tier: Tier }
```

```json
{
    "update_tier": {
        "tier": {
            "level": "2",
            "label": "Silver VIP Patron",
            "price": "4000000000",
            "limit": "75",
            "metadata": {
                "token_uri": "https://example.com/silver_vip.json",
                "extension": {}
            }
        }
    }
}
```

**Authorization**: Only campaign owner can update tiers
**Restrictions**: Only available before campaign starts (READY stage)

### RemoveTier
Removes a tier from the campaign (only available in READY stage).

```rust
RemoveTier { level: Uint64 }
```

```json
{
    "remove_tier": {
        "level": "3"
    }
}
```

**Authorization**: Only campaign owner can remove tiers
**Requirements**: Campaign must be in READY stage

### StartCampaign
Initiates the campaign with specified timing and optional presale orders.

```rust
StartCampaign {
    start_time: Option<Expiry>,
    end_time: Expiry,
    presale: Option<Vec<PresaleTierOrder>>,
}

pub struct PresaleTierOrder {
    pub level: Uint64,
    pub amount: Uint128,
    pub orderer: Addr,
}
```

```json
{
    "start_campaign": {
        "start_time": {
            "at_time": "1672617600000"
        },
        "end_time": {
            "at_time": "1675296000000"
        },
        "presale": [
            {
                "level": "2",
                "amount": "5",
                "orderer": "andr1early_supporter..."
            },
            {
                "level": "3",
                "amount": "2",
                "orderer": "andr1vip_backer..."
            }
        ]
    }
}
```

**Parameters**:
- **start_time**: Optional campaign start time (defaults to immediate)
- **end_time**: Required campaign expiration time
- **presale**: Optional list of presale orders to process immediately

**Validation**:
- Campaign must be in READY stage
- End time must be in the future
- Start time must be before end time (if specified)
- At least one tier must be configured
- Presale orders must reference valid tiers

### PurchaseTiers
Purchases specified tier quantities using native tokens.

```rust
PurchaseTiers { orders: Vec<SimpleTierOrder> }

pub struct SimpleTierOrder {
    pub level: Uint64,
    pub amount: Uint128,
}
```

```json
{
    "purchase_tiers": {
        "orders": [
            {
                "level": "1",
                "amount": "3"
            },
            {
                "level": "2",
                "amount": "1"
            }
        ]
    }
}
```

**Usage**: Send native tokens as funds with this message. The contract will:
1. Validate the campaign is in ONGOING stage and within timing window
2. Calculate total cost for all ordered tiers
3. Verify sufficient payment is provided
4. Update tier sales and campaign funding
5. Refund any excess payment automatically

**Requirements**:
- Campaign must be in ONGOING stage
- Must be within campaign timing window (after start, before end)
- Must send exactly one coin denomination as funds
- Coin denomination must match campaign's accepted token
- Payment must cover total cost of ordered tiers

### Receive (CW20 Hook)
Receives CW20 tokens to purchase tiers.

```rust
pub enum Cw20HookMsg {
    PurchaseTiers { orders: Vec<SimpleTierOrder> },
}
```

```json
{
    "send": {
        "contract": "andr1crowdfund_contract...",
        "amount": "15000000000",
        "msg": "eyJwdXJjaGFzZV90aWVycyI6eyJvcmRlcnMiOlt7ImxldmVsIjoiMSIsImFtb3VudCI6IjMifSx7ImxldmVsIjoiMiIsImFtb3VudCI6IjEifV19fQ=="
    }
}
```

**Requirements**:
- CW20 token must match campaign's configured token
- Same validation as native token purchases
- Automatic excess refund via CW20 transfer

### EndCampaign
Ends the campaign and determines success or failure.

```rust
EndCampaign {}
```

```json
{
    "end_campaign": {}
}
```

**Authorization**: Only campaign owner can end campaign
**Campaign Logic**:
- **Success**: If soft cap is reached (regardless of timing)
- **Success + Withdrawal**: Automatically transfers funds to withdrawal recipient
- **Failure**: If end time reached without meeting soft cap
- **Error**: If called before end time and soft cap not reached

**Automatic Actions**:
- Successful campaigns trigger automatic fund withdrawal
- Sets final campaign stage (SUCCESS or FAILED)
- Enables claim mechanism for supporters

### DiscardCampaign
Manually cancels and discards the campaign.

```rust
DiscardCampaign {}
```

```json
{
    "discard_campaign": {}
}
```

**Authorization**: Only campaign owner can discard
**Requirements**: Campaign must be in READY or ONGOING stage
**Effect**: Sets campaign stage to DISCARDED, enabling refunds for any contributions

### Claim
Claims NFT rewards (if successful) or refunds (if failed/discarded).

```rust
Claim {}
```

```json
{
    "claim": {}
}
```

**Claim Behavior**:
- **Successful campaigns**: Mints and transfers NFT rewards to claimer
- **Failed/Discarded campaigns**: Refunds all contributions to claimer
- **Automatic calculation**: Contract automatically calculates rewards/refunds
- **One-time action**: Each address can only claim once

**Requirements**:
- Campaign must be in SUCCESS, FAILED, or DISCARDED stage
- Claimer must have made purchases during the campaign
- Cannot claim multiple times from the same address

## QueryMsg

### CampaignSummary
Returns comprehensive campaign information and current status.

```rust
#[returns(CampaignSummaryResponse)]
CampaignSummary {}
```

```json
{
    "campaign_summary": {}
}
```

**Response:**
```json
{
    "title": "Revolutionary Art Collection",
    "description": "Funding for groundbreaking digital art series",
    "banner": "https://example.com/banner.jpg",
    "url": "https://example.com/campaign",
    "token_address": "andr1nft_contract_address...",
    "denom": "uandr",
    "withdrawal_recipient": {
        "address": "andr1creator_address...",
        "msg": null
    },
    "soft_cap": "10000000000",
    "hard_cap": "50000000000",
    "start_time": "1672617600000",
    "end_time": "1675296000000",
    "current_stage": "ONGOING",
    "current_capital": "25000000000"
}
```

### TierOrders
Returns purchase history for a specific supporter.

```rust
#[returns(TierOrdersResponse)]
TierOrders {
    orderer: String,
    start_after: Option<u64>,
    limit: Option<u32>,
    order_by: Option<OrderBy>,
}
```

```json
{
    "tier_orders": {
        "orderer": "andr1supporter_address...",
        "start_after": 1,
        "limit": 10,
        "order_by": "asc"
    }
}
```

**Response:**
```json
{
    "orders": [
        {
            "level": "1",
            "amount": "3"
        },
        {
            "level": "2",
            "amount": "1"
        }
    ]
}
```

### Tiers
Returns all available tiers with sales information.

```rust
#[returns(TiersResponse)]
Tiers {
    start_after: Option<u64>,
    limit: Option<u32>,
    order_by: Option<OrderBy>,
}
```

```json
{
    "tiers": {
        "start_after": 1,
        "limit": 5,
        "order_by": "asc"
    }
}
```

**Response:**
```json
{
    "tiers": [
        {
            "tier": {
                "level": "1",
                "label": "Bronze Supporter",
                "price": "1000000000",
                "limit": "100",
                "metadata": {
                    "token_uri": "https://example.com/bronze.json",
                    "extension": {}
                }
            },
            "sold_amount": "45"
        },
        {
            "tier": {
                "level": "2",
                "label": "Silver Patron",
                "price": "5000000000",
                "limit": "50",
                "metadata": {
                    "token_uri": "https://example.com/silver.json",
                    "extension": {}
                }
            },
            "sold_amount": "18"
        }
    ]
}
```

## Usage Examples

### Art Project Crowdfunding
```json
{
    "campaign_config": {
        "title": "Digital Renaissance Collection",
        "description": "Revolutionary NFT art collection combining classical themes with modern digital techniques",
        "token_address": "andr1art_nft_contract...",
        "denom": "uandr",
        "withdrawal_recipient": {
            "address": "andr1artist_collective...",
            "msg": null
        },
        "soft_cap": "50000000000",
        "hard_cap": "200000000000"
    },
    "tiers": [
        {
            "level": "1",
            "label": "Art Supporter",
            "price": "10000000000",
            "limit": "500",
            "metadata": {
                "token_uri": "https://artproject.com/supporter.json",
                "extension": {}
            }
        }
    ]
}
```
_Raise 50-200 ANDR for art collection with 500 supporter NFTs._

### Gaming Development Campaign
```json
{
    "campaign_config": {
        "title": "Epic Space Adventure Game",
        "description": "Next-generation blockchain gaming experience",
        "token_address": "andr1game_items_contract...",
        "denom": "andr1game_token_contract...",
        "withdrawal_recipient": {
            "address": "andr1game_studio...",
            "msg": null
        },
        "soft_cap": "100000000000",
        "hard_cap": "500000000000"
    },
    "tiers": [
        {
            "level": "1",
            "label": "Early Access",
            "price": "50000000000",
            "limit": "1000",
            "metadata": {
                "token_uri": "https://game.com/early_access.json",
                "extension": {
                    "game_perks": ["early_access", "beta_testing"]
                }
            }
        },
        {
            "level": "2",
            "label": "Premium Founder",
            "price": "200000000000",
            "limit": "100",
            "metadata": {
                "token_uri": "https://game.com/founder.json",
                "extension": {
                    "game_perks": ["founder_title", "exclusive_items", "governance_rights"]
                }
            }
        }
    ]
}
```
_Use custom game tokens for funding with exclusive in-game benefits._

### Music Album Production
```json
{
    "campaign_config": {
        "title": "Cosmic Symphony Album",
        "description": "Professional recording and production of ambient electronic album",
        "token_address": "andr1music_nft_contract...",
        "denom": "uusd",
        "withdrawal_recipient": {
            "address": "andr1musician_address...",
            "msg": null
        },
        "soft_cap": "25000000000",
        "hard_cap": "100000000000"
    },
    "tiers": [
        {
            "level": "1",
            "label": "Digital Album",
            "price": "15000000",
            "limit": null,
            "metadata": {
                "token_uri": "https://music.com/digital_album.json",
                "extension": {}
            }
        },
        {
            "level": "2",
            "label": "Signed Vinyl NFT",
            "price": "100000000",
            "limit": "250",
            "metadata": {
                "token_uri": "https://music.com/vinyl_nft.json",
                "extension": {
                    "physical_item": "signed_vinyl"
                }
            }
        }
    ]
}
```
_Accept USD stablecoins for music production funding._

## Operational Examples

### Purchase with Native Tokens
```json
{
    "purchase_tiers": {
        "orders": [
            {
                "level": "1",
                "amount": "2"
            },
            {
                "level": "3",
                "amount": "1"
            }
        ]
    }
}
```
_Send native tokens as funds with this message._

### Purchase with CW20 Tokens
```json
{
    "send": {
        "contract": "andr1crowdfund_contract...",
        "amount": "25000000000",
        "msg": "eyJwdXJjaGFzZV90aWVycyI6eyJvcmRlcnMiOlt7ImxldmVsIjoiMSIsImFtb3VudCI6IjIifSx7ImxldmVsIjoiMyIsImFtb3VudCI6IjEifV19fQ=="
    }
}
```

### Start Campaign with Presale
```json
{
    "start_campaign": {
        "start_time": null,
        "end_time": {
            "at_time": "1675296000000"
        },
        "presale": [
            {
                "level": "2",
                "amount": "10",
                "orderer": "andr1early_backer1..."
            },
            {
                "level": "3",
                "amount": "5",
                "orderer": "andr1vip_supporter..."
            }
        ]
    }
}
```

### Check Campaign Status
```json
{
    "campaign_summary": {}
}
```

### View Supporter's Orders
```json
{
    "tier_orders": {
        "orderer": "andr1supporter_address...",
        "limit": 20
    }
}
```

### End Successful Campaign
```json
{
    "end_campaign": {}
}
```

### Claim Rewards
```json
{
    "claim": {}
}
```

## Integration Patterns

### With App Contract
The Crowdfund ADO can be integrated into App contracts for complete project ecosystems:

```json
{
    "components": [
        {
            "name": "project_crowdfund",
            "ado_type": "crowdfund",
            "component_type": {
                "new": {
                    "campaign_config": {
                        "title": "Revolutionary Project",
                        "token_address": "./project_nfts",
                        "denom": "./project_token",
                        "withdrawal_recipient": {
                            "address": "./project_treasury",
                            "msg": null
                        },
                        "soft_cap": "100000000000",
                        "hard_cap": "1000000000000"
                    },
                    "tiers": []
                }
            }
        }
    ]
}
```

### Creative Projects
For artist and creator funding:

1. **Configure reward tiers** with meaningful benefits and pricing
2. **Set realistic funding goals** based on project requirements
3. **Create compelling metadata** for NFT rewards
4. **Launch with presale** for early supporters and community members

### Product Development
For product and service funding:

1. **Structure tiers around product features** and early access levels
2. **Use hard caps** to limit over-funding and maintain exclusivity
3. **Configure custom tokens** for ecosystem integration
4. **Plan reward distribution** timeline around development milestones

### Community Projects
For community-driven initiatives:

1. **Enable community participation** through accessible tier pricing
2. **Use transparent soft caps** to demonstrate funding requirements
3. **Implement automatic refunds** to build trust with supporters
4. **Leverage presale** for community leaders and early adopters

## Advanced Features

### **Multi-Token Support**
- **Native token campaigns**: Accept blockchain native tokens for broad accessibility
- **CW20 integration**: Support custom project tokens for ecosystem alignment
- **Automatic validation**: Contract validates payment tokens and amounts
- **Refund mechanisms**: Secure refund handling for both native and CW20 tokens

### **Presale Management**
- **Early supporter rewards**: Configure presale orders for VIP supporters
- **Allocation tracking**: Separate tracking for presale vs public contributions
- **Seamless integration**: Presale orders automatically included in campaign totals
- **Flexible timing**: Support immediate presale processing with campaign start

### **Success Mechanisms**
- **Soft cap flexibility**: Success determination based on minimum funding goals
- **Early success**: Campaigns can succeed before end time if soft cap is reached
- **Automatic withdrawal**: Successful campaigns automatically transfer funds to recipients
- **NFT distribution**: Automatic minting and distribution of reward NFTs

### **Failure Safeguards**
- **Automatic refunds**: Failed campaigns automatically refund all contributions
- **Clear criteria**: Transparent failure conditions based on timing and funding
- **Safe cleanup**: Proper state management for failed campaign cleanup
- **Manual override**: Campaign owners can discard campaigns if needed

## Security Features

### **Campaign Protection**
- **Stage enforcement**: Strict enforcement of campaign stages and transitions
- **Timing validation**: Secure validation of start and end times
- **Owner restrictions**: Only campaign owners can modify campaign settings
- **Fund protection**: Secure holding of contributions during campaign period

### **Payment Security**
- **Token validation**: Strict validation of payment token types and amounts
- **Amount verification**: Comprehensive verification of payment sufficiency
- **Refund safety**: Secure refund mechanisms for overpayments and failures
- **Balance tracking**: Accurate real-time tracking of campaign funding

### **Tier Management**
- **Limit enforcement**: Automatic enforcement of tier quantity limits
- **Price validation**: Validation of tier pricing and cost calculations
- **Metadata security**: Secure handling of NFT metadata and extensions
- **Update restrictions**: Tier modifications only allowed before campaign start

### **Claim Security**
- **One-time claims**: Prevention of multiple claims from same address
- **State validation**: Claims only allowed in appropriate campaign stages
- **Reward calculation**: Accurate calculation of rewards and refunds
- **Cleanup automation**: Automatic cleanup of claim data after processing

## Important Notes

- **Single campaign per contract**: Each contract instance manages one campaign
- **Stage-based restrictions**: Many operations are only available in specific campaign stages
- **Automatic refunds**: Excess payments and failed campaign contributions are automatically refunded
- **NFT integration**: Requires compatible CW721 contract for reward distribution
- **Timing enforcement**: Campaign timing is strictly enforced by blockchain time
- **Owner control**: Campaign owners have exclusive control over campaign management
- **Immutable rewards**: Tier rewards cannot be changed after campaign starts
- **Global claims**: Claim mechanism handles both rewards and refunds automatically

## Common Workflow

### 1. **Deploy Campaign**
```json
{
    "campaign_config": {
        "title": "My Project Campaign",
        "token_address": "andr1nft_contract...",
        "denom": "uandr",
        "withdrawal_recipient": {
            "address": "andr1project_wallet...",
            "msg": null
        },
        "soft_cap": "50000000000",
        "hard_cap": "200000000000"
    },
    "tiers": [
        {
            "level": "1",
            "label": "Basic Supporter",
            "price": "10000000000",
            "limit": "1000",
            "metadata": {
                "token_uri": "https://project.com/basic.json",
                "extension": {}
            }
        }
    ]
}
```

### 2. **Configure Additional Tiers**
```json
{
    "add_tier": {
        "tier": {
            "level": "2",
            "label": "Premium Backer",
            "price": "50000000000",
            "limit": "200",
            "metadata": {
                "token_uri": "https://project.com/premium.json",
                "extension": {}
            }
        }
    }
}
```

### 3. **Start Campaign**
```json
{
    "start_campaign": {
        "end_time": {
            "at_time": "1675296000000"
        },
        "presale": [
            {
                "level": "2",
                "amount": "5",
                "orderer": "andr1vip_supporter..."
            }
        ]
    }
}
```

### 4. **Supporters Purchase Tiers**
```json
{
    "purchase_tiers": {
        "orders": [
            {
                "level": "1",
                "amount": "3"
            }
        ]
    }
}
```
_Send tokens as funds with the transaction._

### 5. **Monitor Campaign Progress**
```json
{
    "campaign_summary": {}
}
```

### 6. **End Campaign**
```json
{
    "end_campaign": {}
}
```

### 7. **Supporters Claim Rewards**
```json
{
    "claim": {}
}
```

The Crowdfund ADO provides a comprehensive solution for blockchain-native crowdfunding with advanced features, security controls, and seamless integration with the broader Andromeda ecosystem for professional fundraising applications.