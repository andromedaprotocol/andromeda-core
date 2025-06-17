# Andromeda Fixed Multisig ADO

## Introduction

The Andromeda Fixed Multisig ADO is a governance contract that enables multiple parties to collectively control and execute transactions through a democratic voting process. This contract implements a fixed-voter multisignature system where a predefined set of voters with assigned weights must reach consensus before executing proposals. The multisig provides secure shared ownership, decentralized decision-making, and transparent governance for critical operations in DeFi protocols, DAOs, and multi-party applications.

<b>Ado_type:</b> fixed-multisig

## Why Fixed Multisig ADO

The Fixed Multisig ADO serves as essential governance infrastructure for applications requiring:

- **Shared Ownership**: Multiple parties controlling critical assets and operations
- **Decentralized Governance**: Democratic decision-making through weighted voting
- **Security Enhancement**: Protection against single points of failure and unauthorized actions
- **Treasury Management**: Collective control over fund movements and allocations
- **Protocol Governance**: Managing protocol upgrades and parameter changes
- **Multi-Party Custody**: Shared custody of high-value assets and smart contracts
- **Risk Mitigation**: Preventing unilateral actions through consensus requirements
- **Transparent Operations**: Public voting records and proposal tracking
- **Flexible Thresholds**: Configurable voting requirements for different use cases
- **Time-Based Controls**: Expiration-based proposal management for timely decisions

The ADO provides robust voting mechanisms, proposal management, and execution controls with full transparency and auditability.

## Key Features

### **Weighted Voting System**
- **Fixed voter set**: Predefined list of voters with assigned voting weights
- **Weighted decisions**: Each voter's influence proportional to their assigned weight
- **Threshold-based approval**: Configurable voting thresholds for proposal passage
- **Democratic process**: Fair and transparent voting mechanisms
- **Weight validation**: Automatic validation of voter weights and thresholds

### **Proposal Management**
- **Proposal creation**: Any voter can create proposals with title, description, and actions
- **Action bundling**: Multiple cosmos messages can be bundled in a single proposal
- **Expiration control**: Time-limited voting periods with configurable durations
- **Status tracking**: Real-time proposal status updates (Open, Passed, Rejected, Executed)
- **Execution automation**: Automatic execution of passed proposals

### **Voting Mechanisms**
- **Secure voting**: Only registered voters can participate in voting
- **Vote options**: Support for Yes, No, and Abstain votes
- **Single vote enforcement**: Prevent duplicate voting on the same proposal
- **Real-time tallying**: Automatic vote counting and threshold evaluation
- **Vote transparency**: Public voting records for full auditability

### **Security Features**
- **Access control**: Only voters can create proposals and vote
- **Execution protection**: Only passed proposals can be executed
- **Expiration enforcement**: Expired proposals cannot be voted on or executed
- **State validation**: Comprehensive validation of proposal states and transitions
- **Weight verification**: Continuous validation of voter weights and permissions

## Threshold Types

### **Absolute Count**
Fixed number of votes required regardless of total weight:
```rust
Threshold::AbsoluteCount { weight: 3 }
```

### **Absolute Percentage** 
Percentage of total voting weight required:
```rust
Threshold::AbsolutePercentage { 
    percentage: Decimal::percent(51) 
}
```

### **Threshold Majority**
More than 50% of total voting weight:
```rust
Threshold::ThresholdQuorum { 
    threshold: Decimal::percent(51),
    quorum: Decimal::percent(40) 
}
```

## InstantiateMsg

```rust
pub struct InstantiateMsg {
    pub voters: Vec<Voter>,
    pub threshold: Threshold,
    pub max_voting_period: Duration,
}

pub struct Voter {
    pub addr: AndrAddr,
    pub weight: u64,
}
```

```json
{
    "voters": [
        {
            "addr": "andr1voter1...",
            "weight": 1
        },
        {
            "addr": "andr1voter2...",
            "weight": 2
        },
        {
            "addr": "andr1voter3...",
            "weight": 1
        }
    ],
    "threshold": {
        "absolute_count": {
            "weight": 3
        }
    },
    "max_voting_period": {
        "time": 604800
    }
}
```

**Parameters**:
- **voters**: List of authorized voters with their voting weights
  - **addr**: Voter's address (must be valid Andromeda address)
  - **weight**: Voter's weight in the voting system (must be > 0)
- **threshold**: Voting threshold required for proposal passage
- **max_voting_period**: Maximum duration for voting on proposals

**Validation**:
- At least one voter must be specified
- All voter weights must be greater than zero
- Threshold must be achievable with the given total weight
- Max voting period must be valid duration

## ExecuteMsg

### Propose
Creates a new proposal for the multisig to vote on.

```rust
Propose {
    title: String,
    description: String,
    msgs: Vec<CosmosMsg>,
    latest: Option<Expiration>,
}
```

```json
{
    "propose": {
        "title": "Treasury Fund Allocation",
        "description": "Allocate 100,000 ANDR to development team for Q2 milestones",
        "msgs": [
            {
                "bank": {
                    "send": {
                        "to_address": "andr1dev_team...",
                        "amount": [
                            {
                                "denom": "uandr",
                                "amount": "100000000000"
                            }
                        ]
                    }
                }
            }
        ],
        "latest": {
            "at_time": "1672617600000000000"
        }
    }
}
```

**Parameters**:
- **title**: Short descriptive title for the proposal
- **description**: Detailed description of the proposal and its purpose
- **msgs**: Array of cosmos messages to execute if proposal passes
- **latest**: Optional custom expiration time (defaults to max_voting_period)

**Authorization**: Only registered voters can create proposals
**Automatic Actions**: Creator automatically votes "Yes" on the proposal

### Vote
Casts a vote on an existing proposal.

```rust
Vote {
    proposal_id: u64,
    vote: Vote,
}

pub enum Vote {
    Yes,
    No,
    Abstain,
}
```

```json
{
    "vote": {
        "proposal_id": 1,
        "vote": "yes"
    }
}
```

**Parameters**:
- **proposal_id**: ID of the proposal to vote on
- **vote**: Vote choice (yes, no, or abstain)

**Requirements**:
- Only registered voters with weight ≥ 1 can vote
- Cannot vote on expired proposals
- Cannot change vote once cast
- Proposal must be in Open, Passed, or Rejected status

### Execute
Executes a passed proposal, triggering all bundled messages.

```rust
Execute {
    proposal_id: u64,
}
```

```json
{
    "execute": {
        "proposal_id": 1
    }
}
```

**Parameters**:
- **proposal_id**: ID of the proposal to execute

**Requirements**:
- Proposal must have "Passed" status
- Proposal must not be expired
- Anyone can execute a passed proposal

**Effects**:
- Changes proposal status to "Executed"
- Executes all cosmos messages in the proposal
- Cannot be executed again

### Close
Closes an expired proposal that has not passed.

```rust
Close {
    proposal_id: u64,
}
```

```json
{
    "close": {
        "proposal_id": 1
    }
}
```

**Parameters**:
- **proposal_id**: ID of the proposal to close

**Requirements**:
- Proposal must be expired
- Proposal must not have "Passed" status
- Cannot close already executed or rejected proposals

**Effects**:
- Changes proposal status to "Rejected"
- Prevents further voting or execution

## QueryMsg

### Threshold
Returns the current voting threshold configuration.

```rust
#[returns(ThresholdResponse)]
Threshold {}

pub struct ThresholdResponse {
    pub threshold: Threshold,
    pub total_weight: u64,
}
```

```json
{
    "threshold": {}
}
```

**Response:**
```json
{
    "threshold": {
        "absolute_count": {
            "weight": 3
        }
    },
    "total_weight": 4
}
```

### Proposal
Returns detailed information about a specific proposal.

```rust
#[returns(ProposalResponse)]
Proposal { proposal_id: u64 }

pub struct ProposalResponse {
    pub id: u64,
    pub title: String,
    pub description: String,
    pub msgs: Vec<CosmosMsg>,
    pub status: Status,
    pub expires: Expiration,
    pub deposit: Option<Coin>,
    pub proposer: Addr,
    pub threshold: ThresholdResponse,
}
```

```json
{
    "proposal": {
        "proposal_id": 1
    }
}
```

**Response:**
```json
{
    "id": 1,
    "title": "Treasury Fund Allocation",
    "description": "Allocate 100,000 ANDR to development team for Q2 milestones",
    "msgs": [
        {
            "bank": {
                "send": {
                    "to_address": "andr1dev_team...",
                    "amount": [
                        {
                            "denom": "uandr",
                            "amount": "100000000000"
                        }
                    ]
                }
            }
        }
    ],
    "status": "passed",
    "expires": {
        "at_time": "1672617600000000000"
    },
    "deposit": null,
    "proposer": "andr1proposer...",
    "threshold": {
        "threshold": {
            "absolute_count": {
                "weight": 3
            }
        },
        "total_weight": 4
    }
}
```

### ListProposals
Returns a paginated list of all proposals.

```rust
#[returns(ProposalListResponse)]
ListProposals {
    start_after: Option<u64>,
    limit: Option<u32>,
}
```

```json
{
    "list_proposals": {
        "start_after": null,
        "limit": 10
    }
}
```

### Vote
Returns voting information for a specific voter on a specific proposal.

```rust
#[returns(VoteResponse)]
Vote { 
    proposal_id: u64, 
    voter: AndrAddr 
}
```

```json
{
    "vote": {
        "proposal_id": 1,
        "voter": "andr1voter..."
    }
}
```

**Response:**
```json
{
    "vote": {
        "proposal_id": 1,
        "voter": "andr1voter...",
        "vote": "yes",
        "weight": 2
    }
}
```

### ListVotes
Returns all votes cast on a specific proposal.

```rust
#[returns(VoteListResponse)]
ListVotes {
    proposal_id: u64,
    start_after: Option<String>,
    limit: Option<u32>,
}
```

```json
{
    "list_votes": {
        "proposal_id": 1,
        "start_after": null,
        "limit": 10
    }
}
```

### Voter
Returns voting weight information for a specific voter.

```rust
#[returns(VoterResponse)]
Voter { address: AndrAddr }
```

```json
{
    "voter": {
        "address": "andr1voter..."
    }
}
```

**Response:**
```json
{
    "weight": 2
}
```

### ListVoters
Returns a paginated list of all voters and their weights.

```rust
#[returns(VoterListResponse)]
ListVoters {
    start_after: Option<String>,
    limit: Option<u32>,
}
```

```json
{
    "list_voters": {
        "start_after": null,
        "limit": 10
    }
}
```

## Usage Examples

### Simple Treasury Multisig
```json
{
    "voters": [
        {
            "addr": "andr1ceo...",
            "weight": 1
        },
        {
            "addr": "andr1cto...",
            "weight": 1
        },
        {
            "addr": "andr1cfo...",
            "weight": 1
        }
    ],
    "threshold": {
        "absolute_count": {
            "weight": 2
        }
    },
    "max_voting_period": {
        "time": 259200
    }
}
```

### Weighted Council Governance
```json
{
    "voters": [
        {
            "addr": "andr1lead_dev...",
            "weight": 3
        },
        {
            "addr": "andr1senior_dev1...",
            "weight": 2
        },
        {
            "addr": "andr1senior_dev2...",
            "weight": 2
        },
        {
            "addr": "andr1community_rep...",
            "weight": 1
        }
    ],
    "threshold": {
        "absolute_percentage": {
            "percentage": "0.6"
        }
    },
    "max_voting_period": {
        "time": 604800
    }
}
```

### Protocol Upgrade Proposal
```json
{
    "propose": {
        "title": "Upgrade Protocol to v2.0",
        "description": "Upgrade the core protocol contract to version 2.0 with new features and bug fixes",
        "msgs": [
            {
                "wasm": {
                    "migrate": {
                        "contract_addr": "andr1protocol_contract...",
                        "new_code_id": 42,
                        "msg": "e30="
                    }
                }
            }
        ],
        "latest": {
            "at_time": "1672617600000000000"
        }
    }
}
```

### Fund Transfer Proposal
```json
{
    "propose": {
        "title": "Emergency Fund Transfer",
        "description": "Transfer emergency funds to cover critical infrastructure costs",
        "msgs": [
            {
                "bank": {
                    "send": {
                        "to_address": "andr1infrastructure_wallet...",
                        "amount": [
                            {
                                "denom": "uandr",
                                "amount": "50000000000"
                            }
                        ]
                    }
                }
            }
        ],
        "latest": null
    }
}
```

## Voting Workflow Examples

### 1. Create Proposal
```json
{
    "propose": {
        "title": "Marketing Budget Allocation",
        "description": "Allocate 25,000 ANDR for Q3 marketing campaigns",
        "msgs": [
            {
                "bank": {
                    "send": {
                        "to_address": "andr1marketing_team...",
                        "amount": [
                            {
                                "denom": "uandr",
                                "amount": "25000000000"
                            }
                        ]
                    }
                }
            }
        ],
        "latest": null
    }
}
```

### 2. Cast Votes
```json
{
    "vote": {
        "proposal_id": 1,
        "vote": "yes"
    }
}
```

### 3. Execute Passed Proposal
```json
{
    "execute": {
        "proposal_id": 1
    }
}
```

### 4. Close Expired Proposal
```json
{
    "close": {
        "proposal_id": 2
    }
}
```

## Query Examples

### Check Proposal Status
```json
{
    "proposal": {
        "proposal_id": 1
    }
}
```

### View Voting Results
```json
{
    "list_votes": {
        "proposal_id": 1,
        "start_after": null,
        "limit": 50
    }
}
```

### Check Voter Information
```json
{
    "voter": {
        "address": "andr1voter..."
    }
}
```

### List All Active Proposals
```json
{
    "list_proposals": {
        "start_after": null,
        "limit": 20
    }
}
```

## Integration Patterns

### With App Contract
Fixed multisig can be integrated for governance functionality:

```json
{
    "components": [
        {
            "name": "governance",
            "ado_type": "fixed-multisig",
            "component_type": {
                "new": {
                    "voters": [
                        {
                            "addr": "andr1admin1...",
                            "weight": 1
                        },
                        {
                            "addr": "andr1admin2...",
                            "weight": 1
                        }
                    ],
                    "threshold": {
                        "absolute_count": {
                            "weight": 2
                        }
                    },
                    "max_voting_period": {
                        "time": 432000
                    }
                }
            }
        }
    ]
}
```

### Treasury Management
For managing protocol treasuries:

1. **Deploy multisig** with trusted stakeholders as voters
2. **Set appropriate threshold** for security vs efficiency balance
3. **Create spending proposals** for fund allocations
4. **Vote on proposals** through decentralized governance
5. **Execute approved transfers** automatically

### Protocol Governance
For managing protocol upgrades:

1. **Establish governance council** with weighted voting rights
2. **Propose protocol changes** with detailed descriptions
3. **Allow voting period** for stakeholder input
4. **Execute upgrades** once consensus is reached
5. **Maintain transparency** through public voting records

### Multi-Party Custody
For shared asset custody:

1. **Set up multisig** with key stakeholders
2. **Require majority approval** for asset movements
3. **Use time-limited proposals** for urgent decisions
4. **Maintain audit trail** through proposal history
5. **Enable emergency procedures** through governance

## Advanced Features

### **Weighted Governance**
- **Proportional influence**: Voting power proportional to assigned weights
- **Flexible thresholds**: Multiple threshold types for different requirements
- **Democratic consensus**: Fair representation through weighted voting
- **Scalable voting**: Support for any number of voters with custom weights

### **Proposal Lifecycle**
- **Creation phase**: Any voter can create proposals with action bundles
- **Voting phase**: Time-limited voting with real-time status updates
- **Execution phase**: Automatic execution upon reaching consensus
- **Closure phase**: Proper handling of expired or rejected proposals

### **Security Mechanisms**
- **Access validation**: Continuous validation of voter permissions
- **State protection**: Robust state management and transition validation
- **Execution safety**: Safe execution of complex message bundles
- **Audit transparency**: Complete audit trail for all governance actions

### **Time Management**
- **Flexible durations**: Configurable voting periods for different proposal types
- **Expiration handling**: Automatic handling of expired proposals
- **Deadline enforcement**: Strict enforcement of voting deadlines
- **Time-based decisions**: Support for time-sensitive governance decisions

## Security Features

### **Access Control**
- **Voter validation**: Only registered voters can create proposals and vote
- **Weight verification**: Continuous validation of voter weights and permissions
- **Threshold enforcement**: Strict enforcement of voting thresholds
- **Execution authorization**: Only passed proposals can be executed

### **State Integrity**
- **Atomic operations**: All state changes are atomic to prevent corruption
- **Validation checks**: Comprehensive validation of all state transitions
- **Consistency maintenance**: Maintain consistent state across all operations
- **Error handling**: Graceful handling of edge cases and errors

### **Proposal Security**
- **Content validation**: Validation of proposal content and messages
- **Execution safety**: Safe execution of bundled cosmos messages
- **Status tracking**: Accurate tracking of proposal status and transitions
- **Duplicate prevention**: Prevent duplicate voting and execution

### **Governance Protection**
- **Democratic process**: Fair and transparent voting mechanisms
- **Consensus requirement**: Require consensus before executing critical actions
- **Audit trail**: Complete audit trail for all governance decisions
- **Transparency**: Public visibility of all votes and proposals

## Important Notes

- **Fixed voter set**: Voter list is fixed at instantiation and cannot be changed
- **Weight immutability**: Voter weights cannot be modified after deployment
- **Single vote rule**: Each voter can only vote once per proposal
- **Execution requirement**: Proposals must reach "Passed" status before execution
- **Expiration enforcement**: Expired proposals cannot be voted on or executed
- **Public execution**: Anyone can execute a passed proposal, not just voters
- **Irreversible execution**: Executed proposals cannot be undone or re-executed
- **Threshold validation**: Threshold must be achievable with total weight

## Common Workflow

### 1. **Deploy Multisig**
```json
{
    "voters": [
        {"addr": "andr1voter1...", "weight": 1},
        {"addr": "andr1voter2...", "weight": 1},
        {"addr": "andr1voter3...", "weight": 1}
    ],
    "threshold": {"absolute_count": {"weight": 2}},
    "max_voting_period": {"time": 604800}
}
```

### 2. **Create Proposal**
```json
{
    "propose": {
        "title": "Budget Allocation",
        "description": "Allocate development budget",
        "msgs": [...],
        "latest": null
    }
}
```

### 3. **Cast Votes**
```json
{
    "vote": {
        "proposal_id": 1,
        "vote": "yes"
    }
}
```

### 4. **Check Status**
```json
{
    "proposal": {
        "proposal_id": 1
    }
}
```

### 5. **Execute Proposal**
```json
{
    "execute": {
        "proposal_id": 1
    }
}
```

### 6. **Verify Execution**
```json
{
    "proposal": {
        "proposal_id": 1
    }
}
```

The Fixed Multisig ADO provides essential governance infrastructure for decentralized decision-making, enabling secure, transparent, and democratic control over critical operations in blockchain applications.