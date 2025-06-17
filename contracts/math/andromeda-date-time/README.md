# Andromeda DateTime ADO

## Introduction

The Andromeda DateTime ADO is a utility contract that provides timezone-aware date and time information based on blockchain block time. It supports multiple timezone offsets and returns formatted date-time strings along with the day of the week, making it useful for applications that need to display or process time-based information.

<b>Ado_type:</b> date-time

## Why DateTime ADO

The DateTime ADO serves as a reliable time service for applications requiring:

- **Timezone-Aware Applications**: Display time in users' local timezones
- **Event Scheduling**: Schedule events with proper timezone handling
- **Time-Based Logic**: Implement time-sensitive business logic
- **Logging and Audit**: Timestamp events with human-readable formats
- **Global Applications**: Support users across different time zones
- **Calendar Integration**: Display dates and times in calendar applications
- **Trading Applications**: Show market hours in different timezones
- **Reporting Systems**: Generate time-stamped reports with timezone information

The ADO converts blockchain timestamps to human-readable formats across 25+ supported timezones, from UTC-12 to UTC+14.

## InstantiateMsg

```rust
pub struct InstantiateMsg {}
```

```json
{}
```

The DateTime ADO requires no configuration parameters during instantiation. It's a stateless utility that provides time conversion services.

## ExecuteMsg

The DateTime ADO has no execute messages - it's a read-only utility contract that provides time information through queries only.

## QueryMsg

### GetDateTime
Returns the current date and time in the specified timezone, along with the day of the week.

```rust
pub enum QueryMsg {
    #[returns(GetDateTimeResponse)]
    GetDateTime { timezone: Option<Timezone> },
}
```

```json
{
    "get_date_time": {
        "timezone": "UtcPlus8"
    }
}
```

**Response:**
```json
{
    "day_of_week": "Monday",
    "date_time": "2024-01-15 14:30:25"
}
```

- **timezone**: Optional timezone offset. If not provided, defaults to UTC
- **day_of_week**: String representation of the day (Monday, Tuesday, etc.)
- **date_time**: Formatted date-time string in YYYY-MM-DD HH:MM:SS format

## Supported Timezones

The DateTime ADO supports the following timezone offsets:

### Western Hemisphere (UTC-)
- **UtcMinus12** - UTC-12:00 (Baker Island)
- **UtcMinus11** - UTC-11:00 (American Samoa)
- **UtcMinus10** - UTC-10:00 (Hawaii)
- **UtcMinus9_30** - UTC-09:30 (Marquesas Islands)
- **UtcMinus9** - UTC-09:00 (Alaska)
- **UtcMinus8** - UTC-08:00 (Pacific Time)
- **UtcMinus7** - UTC-07:00 (Mountain Time)
- **UtcMinus6** - UTC-06:00 (Central Time)
- **UtcMinus5** - UTC-05:00 (Eastern Time)
- **UtcMinus4** - UTC-04:00 (Atlantic Time)
- **UtcMinus3** - UTC-03:00 (Argentina, Brazil)
- **UtcMinus2_30** - UTC-02:30 (Newfoundland)
- **UtcMinus2** - UTC-02:00 (South Georgia)
- **UtcMinus1** - UTC-01:00 (Azores)

### Universal Coordinated Time
- **Utc** - UTC+00:00 (Greenwich Mean Time)

### Eastern Hemisphere (UTC+)
- **UtcPlus1** - UTC+01:00 (Central European Time)
- **UtcPlus2** - UTC+02:00 (Eastern European Time)
- **UtcPlus3** - UTC+03:00 (Moscow Time)
- **UtcPlus3_30** - UTC+03:30 (Iran)
- **UtcPlus4** - UTC+04:00 (Gulf Time)
- **UtcPlus4_30** - UTC+04:30 (Afghanistan)
- **UtcPlus5** - UTC+05:00 (Pakistan)
- **UtcPlus5_30** - UTC+05:30 (India)
- **UtcPlus5_45** - UTC+05:45 (Nepal)
- **UtcPlus6** - UTC+06:00 (Bangladesh)
- **UtcPlus6_30** - UTC+06:30 (Myanmar)
- **UtcPlus7** - UTC+07:00 (Thailand, Vietnam)
- **UtcPlus8** - UTC+08:00 (China, Singapore)
- **UtcPlus8_45** - UTC+08:45 (Eucla, Australia)
- **UtcPlus9** - UTC+09:00 (Japan, Korea)
- **UtcPlus9_30** - UTC+09:30 (Adelaide, Australia)
- **UtcPlus10** - UTC+10:00 (Sydney, Australia)
- **UtcPlus10_30** - UTC+10:30 (Lord Howe Island)
- **UtcPlus11** - UTC+11:00 (Solomon Islands)
- **UtcPlus12** - UTC+12:00 (New Zealand)
- **UtcPlus12_45** - UTC+12:45 (Chatham Islands)
- **UtcPlus13** - UTC+13:00 (Tonga)
- **UtcPlus14** - UTC+14:00 (Kiribati)

## Usage Examples

### Get Current UTC Time
```json
{
    "get_date_time": {}
}
```
or
```json
{
    "get_date_time": {
        "timezone": "Utc"
    }
}
```

### Get Time in Eastern US
```json
{
    "get_date_time": {
        "timezone": "UtcMinus5"
    }
}
```

### Get Time in Tokyo
```json
{
    "get_date_time": {
        "timezone": "UtcPlus9"
    }
}
```

### Get Time in India
```json
{
    "get_date_time": {
        "timezone": "UtcPlus5_30"
    }
}
```

## Integration Patterns

### With App Contract
The DateTime ADO can be integrated into App contracts for time-sensitive applications:

```json
{
    "components": [
        {
            "name": "time_service",
            "ado_type": "date-time",
            "component_type": {
                "new": {}
            }
        }
    ]
}
```

### Event Scheduling
For scheduling and time-tracking applications:

1. **Query current time** in the relevant timezone
2. **Calculate time differences** for event scheduling
3. **Display user-friendly timestamps** in local timezone

### Trading Applications
For displaying market hours and trading times:

1. **Query time in market timezone** (e.g., NYSE: UtcMinus5)
2. **Display trading hours** in user's local timezone
3. **Calculate market open/close times**

### Audit and Logging
For timestamping events and creating audit trails:

1. **Query current time** when events occur
2. **Store formatted timestamps** with timezone information
3. **Generate reports** with consistent time formatting

## Important Notes

- **Read-Only Contract**: The DateTime ADO has no execute messages and maintains no state
- **Blockchain Time**: Uses the blockchain's block timestamp as the time source
- **Timezone Accuracy**: Timezone offsets are fixed and don't account for daylight saving time
- **Format Consistency**: All timestamps are returned in YYYY-MM-DD HH:MM:SS format
- **Day of Week**: Always provided in English (Monday, Tuesday, etc.)
- **No State**: The contract is stateless and doesn't store any historical time data

## Example Response Format

```json
{
    "day_of_week": "Wednesday",
    "date_time": "2024-03-15 09:45:30"
}
```

The DateTime ADO provides a simple, reliable way to get timezone-aware time information in blockchain applications, making it easier to build user-friendly interfaces that display time in familiar formats.