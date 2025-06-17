# Andromeda Time Gate ADO

## Introduction

The Andromeda Time Gate ADO is a sophisticated time-based access control contract that cycles through a predefined list of addresses at regular intervals, providing deterministic time-based routing and access management. This contract enables the creation of time-controlled systems where different addresses have access or authority during specific time windows. The time gate automatically calculates which address is currently active based on the configured cycle start time, time intervals, and the current blockchain time, making it ideal for rotating access controls, time-based governance, scheduled operations, and cyclic administrative systems.

<b>Ado_type:</b> time-gate

## Why Time Gate ADO

The Time Gate ADO serves as essential time-based infrastructure for applications requiring:

- **Rotating Access Control**: Cycle through different administrators or operators on a schedule
- **Time-Based Governance**: Implement governance systems with rotating authority
- **Scheduled Operations**: Enable different services or contracts to operate during specific windows
- **Cyclic Administration**: Distribute administrative responsibilities across time periods
- **Automated Rotation**: Automatically rotate permissions without manual intervention
- **Time-Controlled Routing**: Route requests to different handlers based on time
- **Shift Management**: Implement shift-based systems for operators or validators
- **Load Distribution**: Distribute load across different services over time
- **Fault Tolerance**: Rotate between backup systems on scheduled intervals
- **Fair Access Systems**: Ensure fair distribution of access rights over time

The ADO provides precise time calculations with configurable intervals and automated rotation for reliable time-based access management.

## Key Features

### **Automated Time Cycling**
- **Deterministic rotation**: Calculate current active address based on time and configuration
- **Configurable intervals**: Set custom time intervals for address rotation
- **Continuous cycling**: Automatically cycle through addresses without manual intervention
- **Real-time calculation**: Calculate current active address in real-time
- **Precise timing**: Use blockchain time for accurate and verifiable calculations

### **Flexible Configuration**
- **Address list management**: Configure and update the list of rotating addresses
- **Start time control**: Set or update the cycle start time
- **Interval adjustment**: Adjust time intervals for rotation cycles
- **Dynamic updates**: Update configuration parameters through administrative functions
- **Validation checks**: Ensure configuration changes are valid and different from current settings

### **Mathematical Precision**
- **Time delta calculation**: Precise calculation of elapsed time since cycle start
- **Modular arithmetic**: Use modular arithmetic for seamless cycling through addresses
- **Overflow protection**: Safe arithmetic operations with overflow/underflow protection
- **Nanosecond precision**: High precision time calculations using blockchain time
- **Index computation**: Accurate index calculation for address selection

### **Query and Monitoring**
- **Current address query**: Query the currently active address
- **Configuration queries**: Query all configuration parameters
- **Time status checking**: Check cycle start time and current position
- **Address list retrieval**: Retrieve the complete list of gate addresses
- **Interval information**: Query current time interval settings

## Time Gate Mathematics

### **Current Address Calculation**
The time gate uses the following formula to determine the current active address:

```
time_delta = current_time - cycle_start_time
cycle_index = (time_delta / time_interval) % number_of_addresses
current_address = gate_addresses[cycle_index]
```

### **Calculation Steps**
1. **Verify cycle started**: Ensure current time is after cycle start time
2. **Calculate time delta**: Subtract cycle start time from current time
3. **Determine cycle number**: Divide time delta by time interval
4. **Calculate index**: Use modulo operation to get address index
5. **Return address**: Retrieve address at calculated index

### **Example Calculation**
- **Gate addresses**: ["addr1", "addr2", "addr3"] (3 addresses)
- **Time interval**: 3600 seconds (1 hour)
- **Cycle start**: January 1, 2024, 00:00:00 UTC
- **Current time**: January 1, 2024, 07:30:00 UTC

**Calculation**:
- Time delta: 7.5 hours = 27,000 seconds
- Cycle number: 27,000 / 3,600 = 7.5 → 7 (integer division)
- Address index: 7 % 3 = 1
- Current address: "addr2" (index 1)

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
        "andr1admin1...",
        "andr1admin2...",
        "andr1admin3..."
    ],
    "cycle_start_time": {
        "at_time": "1672617600000000000"
    },
    "time_interval": 3600
}
```

**Parameters**:
- **gate_addresses**: List of addresses that will be cycled through
- **cycle_start_time**: Optional start time for the cycling (defaults to deployment time)
- **time_interval**: Optional interval in seconds between rotations (defaults to 3600 seconds/1 hour)

**Configuration Rules**:
- Must provide at least one gate address
- Time interval must be greater than zero
- Cycle start time can be in the future or past
- If no start time provided, cycling begins immediately

**Default Values**:
- **time_interval**: 3600 seconds (1 hour) if not specified
- **cycle_start_time**: Current blockchain time if not specified

## ExecuteMsg

### UpdateCycleStartTime
Updates the cycle start time (owner-only).

```rust
UpdateCycleStartTime {
    cycle_start_time: Option<Expiry>,
}
```

```json
{
    "update_cycle_start_time": {
        "cycle_start_time": {
            "at_time": "1704067200000000000"
        }
    }
}
```

**Parameters**:
- **cycle_start_time**: New start time for the cycling (null for current time)

**Authorization**: Only contract owner can execute
**Validation**: New start time must be different from current start time
**Effect**: Resets the cycling calculation with new start time

### UpdateGateAddresses
Updates the list of addresses in the rotation (owner-only).

```rust
UpdateGateAddresses {
    new_gate_addresses: Vec<AndrAddr>,
}
```

```json
{
    "update_gate_addresses": {
        "new_gate_addresses": [
            "andr1new_admin1...",
            "andr1new_admin2...",
            "andr1new_admin3...",
            "andr1new_admin4..."
        ]
    }
}
```

**Authorization**: Only contract owner can execute
**Validation**: New address list must be different from current list
**Effect**: Updates the rotation to use new addresses
**Impact**: Changes the cycling behavior and current active address

### UpdateTimeInterval
Updates the time interval between rotations (owner-only).

```rust
UpdateTimeInterval {
    time_interval: u64,
}
```

```json
{
    "update_time_interval": {
        "time_interval": 1800
    }
}
```

**Parameters**:
- **time_interval**: New interval in seconds (must be greater than zero)

**Authorization**: Only contract owner can execute
**Validation**: New interval must be positive and different from current interval
**Effect**: Changes how frequently addresses rotate

## QueryMsg

### GetCurrentAdoPath
Returns the currently active address based on time calculations.

```rust
#[returns(Addr)]
GetCurrentAdoPath {}
```

```json
{
    "get_current_ado_path": {}
}
```

**Response:**
```json
"andr1admin2..."
```

**Calculation**: Uses the time gate mathematics to determine which address is currently active
**Requirements**: Cycle must have started (current time >= start time)

### GetCycleStartTime
Returns the cycle start time information.

```rust
#[returns((Expiration, Milliseconds))]
GetCycleStartTime {}
```

```json
{
    "get_cycle_start_time": {}
}
```

**Response:**
```json
[
    {
        "at_time": "1672617600000000000"
    },
    "1672617600000"
]
```

**Returns**: Tuple of (Expiration, Milliseconds) representing start time in different formats

### GetGateAddresses
Returns the list of addresses in the rotation.

```rust
#[returns(Vec<AndrAddr>)]
GetGateAddresses {}
```

```json
{
    "get_gate_addresses": {}
}
```

**Response:**
```json
[
    "andr1admin1...",
    "andr1admin2...",
    "andr1admin3..."
]
```

### GetTimeInterval
Returns the current time interval setting.

```rust
#[returns(String)]
GetTimeInterval {}
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

**Format**: Time interval in seconds as a string

## Usage Examples

### Rotating Administrator System
```json
{
    "gate_addresses": [
        "andr1admin_morning...",
        "andr1admin_afternoon...",
        "andr1admin_evening..."
    ],
    "cycle_start_time": {
        "at_time": "1672617600000000000"
    },
    "time_interval": 28800
}
```
_8-hour shifts: morning (0-8h), afternoon (8-16h), evening (16-24h)_

### Hourly Service Rotation
```json
{
    "gate_addresses": [
        "andr1service_a...",
        "andr1service_b...",
        "andr1service_c...",
        "andr1service_d..."
    ],
    "cycle_start_time": null,
    "time_interval": 3600
}
```
_Hourly rotation between 4 services starting immediately_

### Daily Governance Rotation
```json
{
    "gate_addresses": [
        "andr1dao_monday...",
        "andr1dao_tuesday...",
        "andr1dao_wednesday...",
        "andr1dao_thursday...",
        "andr1dao_friday...",
        "andr1dao_saturday...",
        "andr1dao_sunday..."
    ],
    "cycle_start_time": {
        "at_time": "1672617600000000000"
    },
    "time_interval": 86400
}
```
_Daily rotation for 7-day governance cycle_

### Rapid Rotation Testing
```json
{
    "gate_addresses": [
        "andr1test1...",
        "andr1test2..."
    ],
    "cycle_start_time": null,
    "time_interval": 60
}
```
_1-minute rotation for testing purposes_

## Operational Examples

### Check Current Active Address
```json
{
    "get_current_ado_path": {}
}
```

### Query Configuration
```json
{
    "get_gate_addresses": {}
}
```

```json
{
    "get_time_interval": {}
}
```

```json
{
    "get_cycle_start_time": {}
}
```

### Update Rotation Interval
```json
{
    "update_time_interval": {
        "time_interval": 7200
    }
}
```
_Change to 2-hour intervals_

### Add More Addresses
```json
{
    "update_gate_addresses": {
        "new_gate_addresses": [
            "andr1admin1...",
            "andr1admin2...",
            "andr1admin3...",
            "andr1admin4...",
            "andr1admin5..."
        ]
    }
}
```

### Reset Cycle Start Time
```json
{
    "update_cycle_start_time": {
        "cycle_start_time": null
    }
}
```
_Reset to current time_

## Integration Patterns

### With App Contract
Time gate can be integrated for time-based access control:

```json
{
    "components": [
        {
            "name": "admin_rotation",
            "ado_type": "time-gate",
            "component_type": {
                "new": {
                    "gate_addresses": [
                        "andr1admin_shift1...",
                        "andr1admin_shift2...",
                        "andr1admin_shift3..."
                    ],
                    "cycle_start_time": {
                        "at_time": "1672617600000000000"
                    },
                    "time_interval": 28800
                }
            }
        }
    ]
}
```

### Rotating Access Control
For time-based administrative systems:

1. **Deploy time gate** with administrator addresses and shift schedules
2. **Query current admin** before executing administrative functions
3. **Validate permissions** based on current active address
4. **Update schedules** as administrative needs change
5. **Monitor transitions** for smooth handover between shifts

### Service Load Balancing
For time-based service distribution:

1. **Configure service addresses** with appropriate time intervals
2. **Route requests** to currently active service based on time gate
3. **Distribute load evenly** across all configured services
4. **Handle service updates** through address list modifications
5. **Maintain service availability** through automatic rotation

### Governance Time Slots
For time-based governance systems:

1. **Define governance periods** with specific durations
2. **Assign governance addresses** for each time slot
3. **Rotate decision-making authority** automatically
4. **Ensure fair representation** through equal time allocations
5. **Manage governance transitions** smoothly between periods

### Automated Scheduling
For scheduled operations and tasks:

1. **Set up operation schedules** with time-based triggers
2. **Route operations** to appropriate handlers based on time
3. **Ensure continuous operation** through automatic rotation
4. **Handle schedule changes** through configuration updates
5. **Monitor scheduling accuracy** through query interfaces

## Advanced Features

### **Precise Time Calculations**
- **Nanosecond precision**: High-precision time calculations using blockchain time
- **Arithmetic safety**: Protected against overflow and underflow errors
- **Mathematical accuracy**: Precise modular arithmetic for cycling calculations
- **Time delta computation**: Accurate elapsed time calculations
- **Index determination**: Reliable index calculation for address selection

### **Dynamic Configuration**
- **Real-time updates**: Update configuration without redeploying contract
- **Validation checks**: Ensure all configuration changes are valid
- **Change prevention**: Prevent setting identical values to current configuration
- **Administrative control**: Owner-only configuration modification
- **Immediate effect**: Configuration changes take effect immediately

### **Cycle Management**
- **Flexible start times**: Support past, present, or future start times
- **Continuous cycling**: Seamless rotation through all addresses
- **Automatic reset**: Cycle automatically restarts after completing all addresses
- **Status monitoring**: Query current cycle status and position
- **Transition tracking**: Track transitions between active addresses

### **Integration Support**
- **Address validation**: Comprehensive validation of all addresses
- **Query interface**: Rich query interface for integration monitoring
- **State consistency**: Maintain consistent state across all operations
- **Event tracking**: Track configuration changes through transaction attributes
- **Error handling**: Comprehensive error handling for all edge cases

## Security Features

### **Access Control**
- **Owner restrictions**: Only contract owner can modify configuration
- **Address validation**: Comprehensive validation of all addresses
- **Permission verification**: Verify permissions before configuration changes
- **Unauthorized prevention**: Prevent unauthorized access to administrative functions
- **Administrative oversight**: Maintain administrative control over all operations

### **Mathematical Security**
- **Overflow protection**: Safe arithmetic operations with overflow detection
- **Division safety**: Protected division operations with zero-check
- **Underflow prevention**: Safe subtraction operations with underflow protection
- **Index bounds**: Ensure array access is always within bounds
- **Calculation verification**: Verify all calculations are mathematically sound

### **Configuration Protection**
- **Change validation**: Validate all configuration changes before applying
- **Duplicate prevention**: Prevent setting identical values to current configuration
- **Parameter validation**: Ensure all parameters meet minimum requirements
- **State consistency**: Maintain consistent contract state across updates
- **Error recovery**: Safe error handling without state corruption

### **Time Security**
- **Blockchain time**: Use verifiable blockchain time for all calculations
- **Start time validation**: Validate cycle start times are reasonable
- **Interval validation**: Ensure time intervals are positive and meaningful
- **Time consistency**: Maintain time consistency across all operations
- **Precision maintenance**: Maintain high precision in all time calculations

## Important Notes

- **Time dependency**: Current active address depends on blockchain time
- **Automatic rotation**: Addresses rotate automatically without manual intervention
- **Configuration impact**: Changes to configuration affect current active address immediately
- **Owner privileges**: Only contract owner can modify time gate configuration
- **Cycle requirements**: Cycle must have started for address calculation to work
- **Mathematical precision**: Uses nanosecond precision for accurate calculations
- **Continuous operation**: Gate operates continuously once cycle has started
- **Index calculation**: Uses modular arithmetic for seamless cycling

## Common Workflow

### 1. **Deploy Time Gate**
```json
{
    "gate_addresses": [
        "andr1admin1...",
        "andr1admin2...",
        "andr1admin3..."
    ],
    "cycle_start_time": {
        "at_time": "1672617600000000000"
    },
    "time_interval": 3600
}
```

### 2. **Query Current Active Address**
```json
{
    "get_current_ado_path": {}
}
```

### 3. **Check Configuration**
```json
{
    "get_gate_addresses": {}
}
```

```json
{
    "get_time_interval": {}
}
```

### 4. **Update Time Interval**
```json
{
    "update_time_interval": {
        "time_interval": 7200
    }
}
```

### 5. **Add More Addresses**
```json
{
    "update_gate_addresses": {
        "new_gate_addresses": [
            "andr1admin1...",
            "andr1admin2...",
            "andr1admin3...",
            "andr1admin4..."
        ]
    }
}
```

### 6. **Monitor Active Address Changes**
```json
{
    "get_current_ado_path": {}
}
```
_Query periodically to see address rotation_

### 7. **Reset Cycle**
```json
{
    "update_cycle_start_time": {
        "cycle_start_time": null
    }
}
```

The Time Gate ADO provides sophisticated time-based access control infrastructure for the Andromeda ecosystem, enabling automated rotation, scheduled operations, and time-controlled governance with mathematical precision and comprehensive configuration management.