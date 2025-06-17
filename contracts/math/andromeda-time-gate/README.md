# Andromeda Time Gate ADO

## Introduction

The Andromeda Time Gate ADO is a sophisticated time-based routing and access control contract that cycles through multiple addresses based on configurable time intervals. It provides automatic address rotation functionality, making it ideal for time-based access patterns, rotating services, scheduled operations, and cyclic workflows that require deterministic address selection based on time.

<b>Ado_type:</b> time-gate

## Why Time Gate ADO

The Time Gate ADO serves as a powerful temporal routing system for applications requiring:

- **Rotating Access Control**: Cycle through different authorized addresses over time
- **Time-Based Service Discovery**: Route requests to different services based on time cycles
- **Scheduled Operations**: Execute operations at specific time intervals with different handlers
- **Load Balancing**: Distribute workload across multiple addresses over time periods
- **Governance Cycles**: Implement rotating governance or validation responsibilities
- **Security Rotation**: Automatically rotate access keys or service endpoints
- **Maintenance Windows**: Schedule maintenance operations across different time slots
- **Resource Allocation**: Distribute resources across different addresses cyclically
- **Consensus Mechanisms**: Implement time-based consensus or validation rotation
- **Service Orchestration**: Coordinate multiple services with time-based scheduling

The ADO maintains a list of gate addresses and automatically determines which address is currently active based on the cycle start time and configured intervals.

## InstantiateMsg

```rust
pub struct InstantiateMsg {
    pub gate_addresses: Vec<AndrAddr>,
    pub cycle_start_time: Option<Expiry>,
    pub time_interval: Option<u64>,
}
```

```json
{
    "gate_addresses": [
        "andr1address1...",
        "andr1address2...",
        "andr1address3..."
    ],
    "cycle_start_time": {
        "at_time": "1640995200000000000"
    },
    "time_interval": 3600
}
```

- **gate_addresses**: List of addresses to cycle through
- **cycle_start_time**: Optional starting time for the cycling (defaults to instantiation time if not provided)
  - Can be specified as timestamp or block height
- **time_interval**: Optional time interval in seconds between address rotations (defaults to system setting if not provided)

## ExecuteMsg

### UpdateCycleStartTime
Updates the starting time for the address cycling.

_**Note:** Only contract owner can execute this operation._

```rust
UpdateCycleStartTime { 
    cycle_start_time: Option<Expiry> 
}
```

```json
{
    "update_cycle_start_time": {
        "cycle_start_time": {
            "at_time": "1640995200000000000"
        }
    }
}
```

### UpdateGateAddresses
Updates the list of addresses to cycle through.

_**Note:** Only contract owner can execute this operation._

```rust
UpdateGateAddresses { 
    new_gate_addresses: Vec<AndrAddr> 
}
```

```json
{
    "update_gate_addresses": {
        "new_gate_addresses": [
            "andr1newaddress1...",
            "andr1newaddress2...",
            "andr1newaddress3...",
            "andr1newaddress4..."
        ]
    }
}
```

### UpdateTimeInterval
Updates the time interval between address rotations.

_**Note:** Only contract owner can execute this operation._

```rust
UpdateTimeInterval { 
    time_interval: u64 
}
```

```json
{
    "update_time_interval": {
        "time_interval": 7200
    }
}
```

## QueryMsg

### GetCurrentAdoPath
Returns the currently active address based on the current time and cycle configuration.

```rust
pub enum QueryMsg {
    #[returns(Addr)]
    GetCurrentAdoPath {},
}
```

```json
{
    "get_current_ado_path": {}
}
```

**Response:**
```json
"andr1currentactiveaddress..."
```

### GetCycleStartTime
Returns the cycle start time configuration.

```rust
pub enum QueryMsg {
    #[returns((Expiration, Milliseconds))]
    GetCycleStartTime {},
}
```

```json
{
    "get_cycle_start_time": {}
}
```

**Response:**
```json
[
    {"at_time": "1640995200000000000"},
    "1640995200000"
]
```

### GetGateAddresses
Returns the list of addresses configured for cycling.

```rust
pub enum QueryMsg {
    #[returns(Vec<AndrAddr>)]
    GetGateAddresses {},
}
```

```json
{
    "get_gate_addresses": {}
}
```

**Response:**
```json
[
    "andr1address1...",
    "andr1address2...",
    "andr1address3..."
]
```

### GetTimeInterval
Returns the configured time interval between address rotations.

```rust
pub enum QueryMsg {
    #[returns(String)]
    GetTimeInterval {},
}
```

```json
{
    "get_time_interval": {}
}
```

**Response:**
```json
"3600"
```

## Time Cycling Logic

### Address Selection Algorithm
The Time Gate ADO calculates the current active address using:

1. **Calculate elapsed time** since cycle start time
2. **Determine cycle position** by dividing elapsed time by interval
3. **Select address** using modulo operation with number of addresses
4. **Return active address** from the gate addresses list

### Formula
```
current_index = (elapsed_time_seconds / time_interval) % number_of_addresses
active_address = gate_addresses[current_index]
```

### Example Calculation
- **Start time**: 1640995200 (Unix timestamp)
- **Current time**: 1641002400 (2 hours later)
- **Interval**: 3600 seconds (1 hour)
- **Addresses**: ["addr1", "addr2", "addr3"]

Calculation:
- Elapsed time: 1641002400 - 1640995200 = 7200 seconds
- Cycle position: 7200 / 3600 = 2
- Address index: 2 % 3 = 2
- **Active address**: "addr3"

## Usage Examples

### Hourly Rotation (3 Services)
```json
{
    "gate_addresses": [
        "andr1service1...",
        "andr1service2...",
        "andr1service3..."
    ],
    "cycle_start_time": {
        "at_time": "1640995200000000000"
    },
    "time_interval": 3600
}
```
Each service is active for 1 hour, then rotates to the next.

### Daily Governance Rotation
```json
{
    "gate_addresses": [
        "andr1validator1...",
        "andr1validator2...",
        "andr1validator3...",
        "andr1validator4...",
        "andr1validator5..."
    ],
    "time_interval": 86400
}
```
Each validator has governance authority for 24 hours.

### Load Balancing (15-minute intervals)
```json
{
    "gate_addresses": [
        "andr1server1...",
        "andr1server2..."
    ],
    "time_interval": 900
}
```
Traffic alternates between two servers every 15 minutes.

### Maintenance Windows
```json
{
    "gate_addresses": [
        "andr1production...",
        "andr1maintenance..."
    ],
    "time_interval": 604800
}
```
Switches between production and maintenance mode weekly.

## Integration Patterns

### With App Contract
The Time Gate ADO can be integrated into App contracts for time-based routing:

```json
{
    "components": [
        {
            "name": "service_router",
            "ado_type": "time-gate",
            "component_type": {
                "new": {
                    "gate_addresses": [
                        "andr1service1...",
                        "andr1service2...",
                        "andr1service3..."
                    ],
                    "time_interval": 3600
                }
            }
        }
    ]
}
```

### Service Discovery
For microservice architectures:

1. **Configure service endpoints** as gate addresses
2. **Set rotation interval** based on load balancing needs
3. **Query current active service** before making requests
4. **Route traffic** to the currently active endpoint

### Governance Systems
For decentralized governance:

1. **List governance addresses** (validators, committees, etc.)
2. **Set governance periods** with appropriate intervals
3. **Query current authority** for governance decisions
4. **Rotate responsibilities** automatically over time

### Security Rotation
For key and credential management:

1. **Configure security endpoints** or key holders
2. **Set rotation intervals** for security best practices
3. **Query active security provider** for operations
4. **Automatically rotate** without manual intervention

### Maintenance Scheduling
For system maintenance:

1. **Set production and maintenance addresses**
2. **Configure maintenance windows** with intervals
3. **Route traffic** based on current maintenance status
4. **Automate** maintenance scheduling

## Time Configuration

### Expiry Types
The ADO supports different expiry configurations:

#### At Time (Unix Timestamp)
```json
{
    "at_time": "1640995200000000000"
}
```

#### At Height (Block Height)
```json
{
    "at_height": 1000000
}
```

#### Never (No Expiration)
```json
"never"
```

### Time Interval Units
- **Seconds**: Standard Unix time intervals
- **Examples**: 
  - 60 = 1 minute
  - 3600 = 1 hour  
  - 86400 = 1 day
  - 604800 = 1 week

## Important Notes

- **Deterministic**: Address selection is deterministic based on time
- **Automatic Rotation**: No manual intervention required for rotation
- **Configurable**: All parameters can be updated by contract owner
- **Timezone Independent**: Uses Unix timestamps for consistency
- **Gas Efficient**: Calculations are simple and gas-optimized
- **Predictable**: Future active addresses can be calculated in advance

## Example Timeline

For a 3-address rotation with 1-hour intervals starting at time 0:

| Time Range | Active Address | Notes |
|------------|----------------|-------|
| 0:00 - 0:59 | address[0] | First hour |
| 1:00 - 1:59 | address[1] | Second hour |
| 2:00 - 2:59 | address[2] | Third hour |
| 3:00 - 3:59 | address[0] | Cycle repeats |
| 4:00 - 4:59 | address[1] | Continues cycling |

## Error Handling

### Common Scenarios
- **Empty address list**: Must provide at least one gate address
- **Invalid time configuration**: Start time and interval must be valid
- **Permission errors**: Only owner can update configuration
- **Time calculation errors**: Handles edge cases in time arithmetic

The Time Gate ADO provides a robust, automated solution for time-based address routing and access control, enabling sophisticated temporal workflows while maintaining simplicity and predictability in its operation.