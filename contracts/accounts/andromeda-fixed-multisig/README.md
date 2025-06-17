# Andromeda Fixed Multisig ADO

## Introduction

The Andromeda Fixed Multisig ADO is a secure multi-signature wallet contract that enables shared control over assets and operations through decentralized governance. Based on the CW3 specification, it provides a robust framework for DAOs, partnerships, and organizations requiring collective decision-making with configurable voting thresholds and time-based proposal management.

<b>Ado_type:</b> fixed-multisig

## Why Fixed Multisig ADO

The Fixed Multisig ADO serves as a critical security and governance tool for applications requiring:

- **DAO Governance**: Implement decentralized decision-making for protocol management
- **Treasury Management**: Secure multi-party control over organizational funds
- **Partnership Operations**: Joint control between business partners or collaborators
- **Investment Committees**: Collective investment decisions with voting mechanisms
- **Project Management**: Multi-stakeholder approval for project milestones and payments
- **Security Enhancement**: Eliminate single points of failure in critical operations
- **Compliance Requirements**: Meet regulatory requirements for multi-party authorization
- **Smart Contract Administration**: Secure management of smart contract upgrades and parameters
- **Fund Escrow**: Third-party mediated transactions requiring multiple approvals
- **Community Governance**: Democratic governance for community-owned assets and decisions

The ADO provides weighted voting, configurable thresholds, time-based proposal expiration, and complete proposal lifecycle management with CW3 compatibility for ecosystem interoperability.

## Key Features

### **Fixed Voter Set**
- **Immutable membership**: Voter list set at instantiation and cannot be changed
- **Weighted voting**: Each voter assigned a specific voting weight
- **Member validation**: Only registered voters can create proposals and vote
- **Clear ownership**: Transparent and predictable governance structure

### **Flexible Threshold System**
- **Absolute threshold**: Minimum number of votes required
- **Percentage threshold**: Minimum percentage of total weight required
- **Quorum support**: Configurable participation requirements
- **Threshold validation**: Automatic validation during instantiation

### **Proposal Management**
- **Proposal creation**: Members can create proposals with title, description, and messages
- **Vote tracking**: Comprehensive tracking of votes and weights
- **Status management**: Automatic status updates based on votes and time
- **Message execution**: Execute arbitrary CosmosMsg on successful proposals

### **Time-Based Controls**
- **Voting periods**: Configurable maximum voting duration
- **Automatic expiration**: Proposals automatically expire after voting period
- **Status enforcement**: Time-sensitive status transitions
- **Deadline management**: Flexible deadline setting with maximum limits

## Proposal Lifecycle

### **1. Creation (Open)**
- Member creates proposal with title, description, and messages
- Proposer automatically votes "Yes" with their weight
- Proposal receives unique ID and starts in "Open" status
- Voting period begins, ending at specified expiration time

### **2. Voting (Open/Passed/Rejected)**
- Members vote "Yes", "No", or "Abstain" with their voting weight
- Each member can vote only once per proposal
- Proposal status updates automatically based on current vote tally
- Voting continues until expiration or execution

### **3. Execution (Passed → Executed)**
- Anyone can execute a "Passed" proposal
- All messages in the proposal are executed atomically
- Proposal status changes to "Executed"
- Original messages are executed with multisig as sender

### **4. Closure (Open/Rejected → Rejected)**
- Anyone can close expired proposals that haven't passed
- Prevents execution of expired proposals
- Final status set to "Rejected"
- Cleanup operation for proposal lifecycle

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
            "addr": "andr1member1...",
            "weight": 3
        },
        {
            "addr": "andr1member2...",
            "weight": 2
        },
        {
            "addr": "andr1member3...",
            "weight": 1
        }
    ],
    "threshold": {
        "absolute_count": {
            "weight": 4
        }
    },
    "max_voting_period": {
        "time": 604800
    }
}
```

**Parameters**:
- **voters**: List of multisig members with their voting weights
  - **addr**: Member's address (must be valid Andromeda address)
  - **weight**: Voting power (must be ≥ 1)
- **threshold**: Voting threshold required for proposal passage
  - **AbsoluteCount**: Minimum total weight needed
  - **AbsolutePercentage**: Minimum percentage of total weight needed
  - **ThresholdQuorum**: Combination of threshold and quorum requirements
- **max_voting_period**: Maximum time proposals can remain open
  - **Time**: Duration in seconds
  - **Height**: Duration in block heights

## ExecuteMsg

### Propose
Creates a new proposal for multisig members to vote on.

_**Note:** Only multisig members can create proposals._

```rust
Propose {
    title: String,
    description: String,
    msgs: Vec<CosmosMsg<Empty>>,
    latest: Option<Expiration>,
}
```

```json
{
    "propose": {
        "title": "Treasury Allocation for Development",
        "description": "Allocate 100,000 tokens from treasury to development fund for Q4 roadmap implementation.",
        "msgs": [
            {
                "bank": {
                    "send": {
                        "to_address": "andr1development_fund...",
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
            "at_time": "1641081600000000000"
        }
    }
}
```

**Parameters**:
- **title**: Short descriptive title for the proposal
- **description**: Detailed explanation of the proposal's purpose
- **msgs**: Array of CosmosMsg to execute if proposal passes
- **latest**: Optional custom expiration (cannot exceed max_voting_period)

### Vote
Cast a vote on an existing proposal.

_**Note:** Only multisig members can vote, and each member can vote only once per proposal._

```rust
Vote {
    proposal_id: u64,
    vote: Vote,
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

**Vote Options**:
- `"yes"`: Support the proposal
- `"no"`: Oppose the proposal  
- `"abstain"`: Abstain from voting

### Execute
Executes a passed proposal, triggering all contained messages.

_**Note:** Anyone can execute a passed proposal._

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

### Close
Closes an expired proposal that has not passed.

_**Note:** Anyone can close expired proposals._

```rust
Close {
    proposal_id: u64,
}
```

```json
{
    "close": {
        "proposal_id": 2
    }
}
```

## QueryMsg

### Threshold
Returns the voting threshold configuration.

```rust
pub enum QueryMsg {
    #[returns(ThresholdResponse)]
    Threshold {},
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
            "weight": 4
        }
    },
    "total_weight": 6
}
```

### Proposal
Returns detailed information about a specific proposal.

```rust
pub enum QueryMsg {
    #[returns(ProposalResponse)]
    Proposal { proposal_id: u64 },
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
    "title": "Treasury Allocation for Development",
    "description": "Allocate 100,000 tokens...",
    "msgs": [...],
    "status": "passed",
    "expires": {
        "at_time": "1641081600000000000"
    },
    "threshold": {
        "absolute_count": {
            "weight": 4
        }
    },
    "total_weight": 6,
    "votes": {
        "yes": 4,
        "no": 1,
        "abstain": 0
    },
    "proposer": "andr1member1..."
}
```

### ListProposals
Returns a paginated list of proposals in chronological order.

```rust
pub enum QueryMsg {
    #[returns(ProposalListResponse)]
    ListProposals {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
}
```

```json
{
    "list_proposals": {
        "start_after": 5,
        "limit": 10
    }
}
```

### Vote
Returns voting information for a specific voter on a specific proposal.

```rust
pub enum QueryMsg {
    #[returns(VoteResponse)]
    Vote { proposal_id: u64, voter: AndrAddr },
}
```

```json
{
    "vote": {
        "proposal_id": 1,
        "voter": "andr1member1..."
    }
}
```

**Response:**
```json
{
    "vote": {
        "proposal_id": 1,
        "voter": "andr1member1...",
        "vote": "yes",
        "weight": 3
    }
}
```

### Voter
Returns information about a specific multisig member.

```rust
pub enum QueryMsg {
    #[returns(VoterResponse)]
    Voter { address: AndrAddr },
}
```

```json
{
    "voter": {
        "address": "andr1member1..."
    }
}
```

**Response:**
```json
{
    "addr": "andr1member1...",
    "weight": 3
}
```

### ListVoters
Returns a paginated list of all multisig members.

```rust
pub enum QueryMsg {
    #[returns(VoterListResponse)]
    ListVoters {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}
```

```json
{
    "list_voters": {
        "start_after": "andr1member1...",
        "limit": 20
    }
}
```

## Threshold Types

### Absolute Count
Requires a minimum total weight of votes.

```json
{
    "absolute_count": {
        "weight": 4
    }
}
```
**Example**: Requires at least 4 total weight to pass

### Absolute Percentage
Requires a minimum percentage of total voting weight.

```json
{
    "absolute_percentage": {
        "percentage": "0.67"
    }
}
```
**Example**: Requires at least 67% of total weight to pass

### Threshold Quorum
Combines threshold and quorum requirements.

```json
{
    "threshold_quorum": {
        "threshold": "0.5",
        "quorum": "0.4"
    }
}
```
**Example**: Requires 50% of participating votes AND 40% total participation

## Usage Examples

### DAO Treasury Management
```json
{
    "voters": [
        {"addr": "andr1founder...", "weight": 3},
        {"addr": "andr1cto...", "weight": 2},
        {"addr": "andr1community_rep...", "weight": 2},
        {"addr": "andr1advisor...", "weight": 1}
    ],
    "threshold": {
        "absolute_count": {"weight": 5}
    },
    "max_voting_period": {"time": 604800}
}
```
**Result**: Requires 5/8 total weight (62.5%) to pass proposals

### Partnership Agreement
```json
{
    "voters": [
        {"addr": "andr1partner_a...", "weight": 1},
        {"addr": "andr1partner_b...", "weight": 1},
        {"addr": "andr1partner_c...", "weight": 1}
    ],
    "threshold": {
        "absolute_count": {"weight": 2}
    },
    "max_voting_period": {"time": 259200}
}
```
**Result**: Requires 2/3 partners (67%) to approve decisions

### Investment Committee
```json
{
    "voters": [
        {"addr": "andr1lead_investor...", "weight": 4},
        {"addr": "andr1investor_2...", "weight": 3},
        {"addr": "andr1investor_3...", "weight": 2},
        {"addr": "andr1advisor...", "weight": 1}
    ],
    "threshold": {
        "absolute_percentage": {"percentage": "0.6"}
    },
    "max_voting_period": {"time": 1209600}
}
```
**Result**: Requires 60% of total weight (6/10) to approve investments

### Security Council
```json
{
    "voters": [
        {"addr": "andr1security_lead...", "weight": 2},
        {"addr": "andr1tech_lead...", "weight": 2},
        {"addr": "andr1ops_lead...", "weight": 1},
        {"addr": "andr1community_lead...", "weight": 1}
    ],
    "threshold": {
        "threshold_quorum": {
            "threshold": "0.67",
            "quorum": "0.5"
        }
    },
    "max_voting_period": {"time": 172800}
}
```

## Integration Patterns

### With App Contract
The Fixed Multisig can be integrated into App contracts for governance:

```json
{
    "components": [
        {
            "name": "governance_multisig",
            "ado_type": "fixed-multisig",
            "component_type": {
                "new": {
                    "voters": [
                        {"addr": "andr1member1...", "weight": 3},
                        {"addr": "andr1member2...", "weight": 2}
                    ],
                    "threshold": {
                        "absolute_count": {"weight": 3}
                    },
                    "max_voting_period": {"time": 604800}
                }
            }
        }
    ]
}
```

### Treasury Management
For organizational fund management:

1. **Set up multisig** with key stakeholders as voters
2. **Configure threshold** appropriate for fund security
3. **Create proposals** for fund allocations and expenses
4. **Vote and execute** approved financial decisions

### Protocol Governance
For smart contract and protocol management:

1. **Establish governance multisig** with protocol stakeholders
2. **Create upgrade proposals** for contract improvements
3. **Vote on parameter changes** and protocol updates
4. **Execute governance decisions** through multisig authority

### Partnership Operations
For joint venture management:

1. **Include all partners** as multisig voters with appropriate weights
2. **Set partnership threshold** for major decisions
3. **Propose partnership activities** and investments
4. **Execute joint decisions** through collective approval

## Security Features

### **Fixed Membership**
- **Immutable voters**: Voter set cannot be changed after instantiation
- **Known participants**: All voters are known and verified at setup
- **Weight transparency**: Voting weights are publicly visible
- **No membership changes**: Eliminates governance attack vectors

### **Threshold Enforcement**
- **Configurable thresholds**: Flexible threshold requirements
- **Automatic validation**: Threshold validation during instantiation
- **Vote counting**: Precise vote weight calculations
- **Status enforcement**: Automatic proposal status updates

### **Time-Based Security**
- **Voting periods**: Limited time for proposal consideration
- **Expiration enforcement**: Automatic proposal expiration
- **Status transitions**: Time-sensitive status management
- **Deadline protection**: Prevents indefinite open proposals

### **Execution Controls**
- **Passed proposals only**: Only passed proposals can be executed
- **Anyone can execute**: Prevents execution blocking by members
- **Atomic execution**: All messages execute together or fail together
- **Status tracking**: Clear execution status for transparency

## Important Notes

- **Fixed voters**: Voter list cannot be modified after contract instantiation
- **Weighted voting**: Each voter's influence is determined by their weight
- **Single vote**: Each member can vote only once per proposal
- **Automatic status**: Proposal status updates automatically based on votes
- **Anyone executes**: Any address can execute passed proposals
- **Expiration required**: All proposals must have expiration times
- **CW3 compatible**: Full compatibility with CW3 ecosystem standards
- **Threshold validation**: Thresholds are validated during instantiation

## Common Workflow

### 1. **Create Proposal**
```json
{
    "propose": {
        "title": "Fund Development Team",
        "description": "Allocate Q4 development budget",
        "msgs": [...]
    }
}
```

### 2. **Members Vote**
```json
{
    "vote": {
        "proposal_id": 1,
        "vote": "yes"
    }
}
```

### 3. **Execute When Passed**
```json
{
    "execute": {
        "proposal_id": 1
    }
}
```

### 4. **Close If Expired**
```json
{
    "close": {
        "proposal_id": 2
    }
}
```

The Fixed Multisig ADO provides a robust, secure foundation for decentralized governance and multi-party asset management, offering the flexibility and security controls needed for professional DAOs, partnerships, and organizational management while maintaining full CW3 ecosystem compatibility.