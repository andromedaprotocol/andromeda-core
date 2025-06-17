# Andromeda Date Time ADO

## Introduction

The Andromeda Date Time ADO is a utility contract that provides comprehensive date and time functionality with timezone support for blockchain applications. This contract leverages blockchain time to offer reliable datetime calculations, timezone conversions, and time-based queries. The date-time system supports a wide range of global timezones and provides formatted datetime output with day-of-week information, making it essential for time-sensitive applications, scheduling systems, and global operations requiring accurate time calculations.

<b>Ado_type:</b> date-time

## Why Date Time ADO

The Date Time ADO serves as essential time infrastructure for applications requiring:

- **Time-Based Logic**: Implement time-dependent business logic and operations
- **Global Applications**: Support applications operating across multiple timezones
- **Scheduling Systems**: Calculate scheduling and timing for automated operations
- **Time Validation**: Validate time-based conditions and constraints
- **Timestamp Conversion**: Convert blockchain timestamps to human-readable formats
- **International Operations**: Handle timezone-aware operations for global users
- **Time Reporting**: Generate time-based reports and analytics
- **Calendar Integration**: Support calendar-based functionality and scheduling
- **Time-Sensitive Workflows**: Enable workflows that depend on specific times or dates
- **Audit and Logging**: Provide accurate timestamp information for audit trails

The ADO provides precise time calculations with comprehensive timezone support for reliable temporal operations.

## Key Features

### **Comprehensive Timezone Support**
- **Global timezone coverage**: Support for UTC-12 to UTC+14 timezones
- **Half-hour timezones**: Support for timezone offsets like UTC+5:30, UTC+9:30
- **Quarter-hour timezones**: Support for unusual timezone offsets like UTC+5:45
- **Automatic conversion**: Automatic conversion from blockchain time to local time
- **Default UTC**: Defaults to UTC if no timezone is specified

### **Formatted Date Time Output**
- **Standardized format**: Consistent YYYY-MM-DD HH-MM-SS format
- **Day of week**: Provides day of the week (Mon, Tue, Wed, etc.)
- **Zero-padded values**: Properly formatted with leading zeros
- **Human-readable**: Easy to parse and display in applications
- **Consistent formatting**: Same format across all timezone conversions

### **Blockchain Time Integration**
- **Blockchain timestamp**: Uses reliable blockchain time as source
- **Deterministic results**: Consistent results across all nodes
- **No external dependencies**: Self-contained time calculations
- **Timezone calculation**: Accurate timezone offset calculations
- **Overflow protection**: Safe arithmetic for timezone conversions

### **Query Interface**
- **Simple querying**: Easy-to-use query interface for time information
- **Optional timezone**: Can specify timezone or use default UTC
- **Instant results**: Fast calculations without state storage
- **Read-only operations**: No state modification, pure calculations
- **Gas efficient**: Minimal gas usage for time queries

## Timezone Support

### **Supported Timezones**
The ADO supports the following timezones with their UTC offsets:

**Western Timezones (UTC-)**:
- UTC-12 to UTC-1 (standard hour offsets)
- UTC-9:30 (Adelaide Daylight Time region)
- UTC-2:30 (Newfoundland Standard Time region)

**UTC**:
- UTC+0 (Coordinated Universal Time)

**Eastern Timezones (UTC+)**:
- UTC+1 to UTC+14 (standard hour offsets)
- UTC+3:30 (Iran Standard Time)
- UTC+4:30 (Afghanistan Time)  
- UTC+5:30 (India Standard Time)
- UTC+5:45 (Nepal Time)
- UTC+6:30 (Myanmar Time)
- UTC+8:45 (Australian Central Western Standard Time)
- UTC+9:30 (Australian Central Standard Time)
- UTC+10:30 (Lord Howe Standard Time)
- UTC+12:45 (Chatham Standard Time)

### **Timezone Calculation**
The contract calculates local time using:
1. **Get blockchain timestamp** in seconds since Unix epoch
2. **Extract timezone offset** from enum value (e.g., UTC+5:30 = 550)
3. **Convert to seconds** by multiplying by 36 (550 * 36 = 19,800 seconds)
4. **Add offset** to blockchain timestamp for local time
5. **Format result** into human-readable datetime string

## InstantiateMsg

```rust
pub struct InstantiateMsg {}
```

```json
{}
```

**Parameters**: None - the date-time ADO requires no configuration

**Deployment**: Simple deployment with standard Andromeda ADO parameters
**State**: Stateless contract - performs calculations using blockchain time

## ExecuteMsg

The Date Time ADO has no execute messages - it is a read-only utility contract.

```rust
pub enum ExecuteMsg {}
```

## QueryMsg

### GetDateTime
Returns the current date and time with optional timezone conversion.

```rust
#[returns(GetDateTimeResponse)]
GetDateTime {
    timezone: Option<Timezone>,
}

pub struct GetDateTimeResponse {
    pub day_of_week: String,
    pub date_time: String,
}

pub enum Timezone {
    UtcMinus12 = -1200,
    UtcMinus11 = -1100,
    // ... (see full list above)
    UtcPlus13 = 1300,
    UtcPlus14 = 1400,
}
```

```json
{
    "get_date_time": {
        "timezone": "UtcPlus5_30"
    }
}
```

**Response:**
```json
{
    "day_of_week": "Wed",
    "date_time": "2024-01-15 14-30-45"
}
```

**Parameters**:
- **timezone**: Optional timezone for conversion (defaults to UTC)

**Response Format**:
- **day_of_week**: Three-letter day abbreviation (Mon, Tue, Wed, Thu, Fri, Sat, Sun)
- **date_time**: Formatted as "YYYY-MM-DD HH-MM-SS"

## Usage Examples

### Get Current UTC Time
```json
{
    "get_date_time": {
        "timezone": null
    }
}
```

```json
{
    "get_date_time": {
        "timezone": "Utc"
    }
}
```

### Get Time in Different Timezones

**Eastern Standard Time (UTC-5):**
```json
{
    "get_date_time": {
        "timezone": "UtcMinus5"
    }
}
```

**Central European Time (UTC+1):**
```json
{
    "get_date_time": {
        "timezone": "UtcPlus1"
    }
}
```

**India Standard Time (UTC+5:30):**
```json
{
    "get_date_time": {
        "timezone": "UtcPlus5_30"
    }
}
```

**Japan Standard Time (UTC+9):**
```json
{
    "get_date_time": {
        "timezone": "UtcPlus9"
    }
}
```

**Australian Central Standard Time (UTC+9:30):**
```json
{
    "get_date_time": {
        "timezone": "UtcPlus9_30"
    }
}
```

### Response Examples

**UTC Response:**
```json
{
    "day_of_week": "Mon",
    "date_time": "2024-01-15 12-00-00"
}
```

**India Standard Time Response:**
```json
{
    "day_of_week": "Mon", 
    "date_time": "2024-01-15 17-30-00"
}
```

## Integration Patterns

### With App Contract
Date time can be integrated for time-based functionality:

```json
{
    "components": [
        {
            "name": "datetime_service",
            "ado_type": "date-time",
            "component_type": {
                "new": {}
            }
        }
    ]
}
```

### Time-Based Scheduling
For scheduling and time validation:

1. **Query current time** in appropriate timezone
2. **Parse datetime response** for scheduling logic
3. **Compare with target times** for scheduling decisions
4. **Use day-of-week** for weekly scheduling patterns
5. **Implement time-based conditions** using formatted output

### Global Application Support
For applications serving global users:

1. **Query user's local time** using their timezone
2. **Display localized times** in user interfaces
3. **Schedule operations** based on local business hours
4. **Coordinate global activities** across timezones
5. **Provide timezone-aware functionality** throughout application

### Audit and Logging
For timestamping and audit trails:

1. **Generate timestamps** for all significant events
2. **Use consistent timezone** (typically UTC) for logs
3. **Include day-of-week** for human-readable logs
4. **Format timestamps** consistently across systems
5. **Provide timezone context** for global operations

### Time Validation
For validating time-based conditions:

1. **Check current time** against business hour rules
2. **Validate scheduling constraints** using datetime queries
3. **Implement time-based access controls** with timezone awareness
4. **Coordinate time-sensitive operations** across regions
5. **Ensure timezone compliance** for regulatory requirements

## Advanced Features

### **Comprehensive Timezone Coverage**
- **Global support**: Coverage for all major world timezones
- **Unusual offsets**: Support for non-standard timezone offsets
- **Automatic conversion**: Seamless conversion between timezones
- **Accurate calculations**: Precise offset calculations and conversions
- **Consistent formatting**: Uniform output format across all timezones

### **Blockchain Time Integration**
- **Deterministic time**: Uses blockchain time for consistency
- **No external dependencies**: Self-contained time calculations
- **Network consensus**: Time based on blockchain consensus
- **Reliable source**: Tamper-proof time source
- **Cross-chain consistency**: Consistent across blockchain networks

### **Efficient Calculations**
- **Fast queries**: Minimal computation for time calculations
- **Gas optimization**: Low gas usage for all operations
- **No state storage**: Stateless calculations reduce storage costs
- **Arithmetic safety**: Protected arithmetic operations
- **Error handling**: Robust error handling for edge cases

### **Developer-Friendly Interface**
- **Simple API**: Easy-to-use query interface
- **Consistent format**: Predictable response format
- **Human-readable**: Output designed for easy parsing and display
- **Flexible timezone**: Optional timezone parameter with sensible defaults
- **Clear documentation**: Well-documented timezone support

## Security Features

### **Deterministic Operations**
- **Blockchain time**: Uses consensus-based blockchain time
- **Consistent results**: Same input always produces same output
- **No external calls**: Self-contained calculations prevent manipulation
- **Arithmetic safety**: Protected arithmetic prevents overflow/underflow
- **Input validation**: Validates timezone parameters

### **Read-Only Operations**
- **No state modification**: Pure calculation contract
- **No storage requirements**: Stateless operation reduces attack surface
- **Gas efficiency**: Minimal gas usage for operations
- **No permission required**: Public access to time information
- **Safe calculations**: No side effects from time queries

### **Timezone Validation**
- **Enum constraints**: Timezone limited to predefined valid values
- **Offset validation**: Valid timezone offsets prevent calculation errors
- **Default handling**: Safe defaults for missing parameters
- **Error prevention**: Prevents invalid timezone specifications
- **Calculation bounds**: Ensures calculations stay within valid ranges

## Important Notes

- **Blockchain time**: Uses blockchain consensus time, not real-world atomic time
- **Timezone static**: Timezone definitions are static (no daylight saving time)
- **Format consistency**: Always uses YYYY-MM-DD HH-MM-SS format
- **UTC default**: Defaults to UTC when no timezone specified
- **Read-only**: Contract performs calculations only, no state modification
- **Day calculation**: Day of week calculated from converted local time
- **Offset precision**: Supports quarter-hour timezone precision
- **No validation**: Does not validate if timezone matches actual location

## Common Workflow

### 1. **Deploy Date Time ADO**
```json
{}
```

### 2. **Query Current UTC Time**
```json
{
    "get_date_time": {
        "timezone": "Utc"
    }
}
```

### 3. **Query Local Time**
```json
{
    "get_date_time": {
        "timezone": "UtcPlus5_30"
    }
}
```

### 4. **Use in Time-Based Logic**
```javascript
// Example application logic
const response = await query({
    "get_date_time": {
        "timezone": "UtcPlus9"
    }
});

const [year, month, day] = response.date_time.split(' ')[0].split('-');
const [hour, minute, second] = response.date_time.split(' ')[1].split('-');

// Use parsed datetime for business logic
if (hour >= 9 && hour < 17 && response.day_of_week !== "Sat" && response.day_of_week !== "Sun") {
    // Business hours logic
}
```

### 5. **Generate Timestamps**
```json
{
    "get_date_time": {
        "timezone": "Utc"
    }
}
```

### 6. **Timezone Comparison**
```javascript
// Query multiple timezones for comparison
const utc_time = await query({"get_date_time": {"timezone": "Utc"}});
const local_time = await query({"get_date_time": {"timezone": "UtcPlus5_30"}});
```

The Date Time ADO provides essential temporal infrastructure for the Andromeda ecosystem, enabling accurate datetime calculations, timezone conversions, and time-based logic with comprehensive global timezone support and reliable blockchain time integration.