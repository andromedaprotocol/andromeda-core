# Andromeda Graph ADO

## Introduction

The Andromeda Graph ADO is a spatial mapping and coordinate management contract that provides 2D and 3D coordinate storage, user location tracking, and map configuration management. This contract enables applications to store and manage spatial data with configurable map boundaries, coordinate validation, and timestamp tracking. The Graph ADO supports both 2D and 3D coordinate systems with optional negative coordinate support and configurable decimal precision for accurate spatial data management.

<b>Ado_type:</b> graph

## Why Graph ADO

The Graph ADO serves as essential spatial infrastructure for applications requiring:

- **Spatial Data Management**: Store and manage 2D/3D coordinates with validation
- **User Location Tracking**: Track user positions and location history  
- **Map Configuration**: Define configurable map boundaries and coordinate systems
- **Gaming Applications**: Manage player positions and world coordinates
- **Location Services**: Build location-aware applications and services
- **Spatial Analysis**: Support spatial analysis and geographic information systems
- **Coordinate Validation**: Ensure coordinates fall within defined map boundaries
- **Timestamp Tracking**: Track when coordinates were stored or updated
- **Multi-dimensional Support**: Handle both 2D and 3D coordinate systems
- **Precision Control**: Configure decimal precision for coordinate accuracy

The ADO provides comprehensive spatial data management with flexible map configuration and coordinate validation.

## Key Features

### **Configurable Map System**
- **Map boundaries**: Define x, y, and optional z-axis boundaries
- **Coordinate validation**: Automatic validation against map boundaries
- **Negative coordinate support**: Optional support for negative coordinates
- **Decimal precision**: Configurable decimal places for coordinate accuracy
- **2D/3D support**: Optional third dimension for 3D coordinate systems

### **Coordinate Storage**
- **User coordinates**: Store coordinates associated with user addresses
- **Timestamp tracking**: Optional timestamp recording for coordinate history
- **Coordinate validation**: Ensure coordinates fall within map boundaries
- **Batch operations**: Store multiple user coordinates efficiently
- **Coordinate updates**: Update existing user coordinates with validation

### **Spatial Queries**
- **Map information**: Query current map configuration and boundaries
- **User coordinates**: Retrieve coordinates for specific users
- **All coordinates**: Paginated retrieval of all stored coordinates
- **Point counting**: Get maximum number of stored points
- **Coordinate history**: Access coordinate storage timestamps

## InstantiateMsg

```rust
pub struct InstantiateMsg {
    pub map_info: MapInfo,
}

pub struct MapInfo {
    pub map_size: MapSize,
    pub allow_negative: bool,
    pub map_decimal: u16,
}

pub struct MapSize {
    pub x_width: u64,
    pub y_width: u64,
    pub z_width: Option<u64>,
}
```

```json
{
    "map_info": {
        "map_size": {
            "x_width": 1000,
            "y_width": 1000,
            "z_width": 100
        },
        "allow_negative": true,
        "map_decimal": 2
    }
}
```

**Parameters**:
- **map_info**: Map configuration and boundaries
  - **map_size**: Coordinate system boundaries
    - **x_width**: Maximum x-coordinate value
    - **y_width**: Maximum y-coordinate value  
    - **z_width**: Optional maximum z-coordinate for 3D maps
  - **allow_negative**: Whether negative coordinates are allowed
  - **map_decimal**: Number of decimal places for coordinate precision

## ExecuteMsg

### UpdateMap
Updates the map configuration and boundaries (restricted).

```rust
UpdateMap {
    map_info: MapInfo,
}
```

```json
{
    "update_map": {
        "map_info": {
            "map_size": {
                "x_width": 2000,
                "y_width": 2000,
                "z_width": null
            },
            "allow_negative": false,
            "map_decimal": 3
        }
    }
}
```

### StoreCoordinate
Stores a coordinate with optional timestamp tracking (restricted).

```rust
StoreCoordinate {
    coordinate: Coordinate,
    is_timestamp_allowed: bool,
}

pub struct Coordinate {
    pub x_coordinate: SignedDecimal,
    pub y_coordinate: SignedDecimal,
    pub z_coordinate: Option<SignedDecimal>,
}
```

```json
{
    "store_coordinate": {
        "coordinate": {
            "x_coordinate": "150.25",
            "y_coordinate": "300.75",
            "z_coordinate": "50.0"
        },
        "is_timestamp_allowed": true
    }
}
```

### StoreUserCoordinate
Stores coordinates for multiple users from specified location paths (restricted).

```rust
StoreUserCoordinate {
    user_location_paths: Vec<AndrAddr>,
}
```

```json
{
    "store_user_coordinate": {
        "user_location_paths": [
            "andr1user1address...",
            "andr1user2address..."
        ]
    }
}
```

### DeleteUserCoordinate
Deletes stored coordinates for a specific user (restricted).

```rust
DeleteUserCoordinate {
    user: AndrAddr,
}
```

```json
{
    "delete_user_coordinate": {
        "user": "andr1useraddress..."
    }
}
```

## QueryMsg

### GetMapInfo
Returns the current map configuration and boundaries.

```rust
#[returns(GetMapInfoResponse)]
GetMapInfo {}

pub struct GetMapInfoResponse {
    pub map_info: MapInfo,
}
```

```json
{
    "get_map_info": {}
}
```

**Response:**
```json
{
    "map_info": {
        "map_size": {
            "x_width": 1000,
            "y_width": 1000,
            "z_width": 100
        },
        "allow_negative": true,
        "map_decimal": 2
    }
}
```

### GetMaxPointNumber
Returns the maximum number of points that can be stored.

```rust
#[returns(GetMaxPointNumberResponse)]
GetMaxPointNumber {}

pub struct GetMaxPointNumberResponse {
    pub max_point_number: u128,
}
```

```json
{
    "get_max_point_number": {}
}
```

### GetAllPoints
Returns all stored coordinates with pagination support.

```rust
#[returns(GetAllPointsResponse)]
GetAllPoints {
    start: Option<u128>,
    limit: Option<u32>,
}

pub struct GetAllPointsResponse {
    pub points: Vec<(CoordinateInfo, StoredDate)>,
}

pub struct CoordinateInfo {
    pub x: String,
    pub y: String,
    pub z: Option<String>,
}

pub struct StoredDate {
    pub timestamp: Option<u64>,
}
```

```json
{
    "get_all_points": {
        "start": 0,
        "limit": 50
    }
}
```

### GetUserCoordinate
Returns stored coordinates for a specific user.

```rust
#[returns(CoordinateInfo)]
GetUserCoordinate {
    user: AndrAddr,
}
```

```json
{
    "get_user_coordinate": {
        "user": "andr1useraddress..."
    }
}
```

**Response:**
```json
{
    "x": "150.25",
    "y": "300.75",
    "z": "50.0"
}
```

## Usage Examples

### Gaming World Coordinates
```json
{
    "map_info": {
        "map_size": {
            "x_width": 10000,
            "y_width": 10000,
            "z_width": 1000
        },
        "allow_negative": false,
        "map_decimal": 1
    }
}
```

### Geographic Coordinate System
```json
{
    "map_info": {
        "map_size": {
            "x_width": 180,
            "y_width": 90,
            "z_width": null
        },
        "allow_negative": true,
        "map_decimal": 6
    }
}
```

### Store Player Position
```json
{
    "store_coordinate": {
        "coordinate": {
            "x_coordinate": "2500.5",
            "y_coordinate": "1750.3",
            "z_coordinate": "125.0"
        },
        "is_timestamp_allowed": true
    }
}
```

### Query All Player Positions
```json
{
    "get_all_points": {
        "start": 0,
        "limit": 100
    }
}
```

## Integration Patterns

### With App Contract
Spatial data management for applications:

```json
{
    "components": [
        {
            "name": "world_map",
            "ado_type": "graph",
            "component_type": {
                "new": {
                    "map_info": {
                        "map_size": {
                            "x_width": 5000,
                            "y_width": 5000,
                            "z_width": 500
                        },
                        "allow_negative": false,
                        "map_decimal": 2
                    }
                }
            }
        }
    ]
}
```

### Gaming Applications
For game world and player management:

1. **Define game world** with appropriate boundaries
2. **Store player positions** with timestamp tracking
3. **Track player movement** through coordinate updates
4. **Query nearby players** using coordinate ranges
5. **Manage world boundaries** with coordinate validation

### Location Services
For location-based applications:

1. **Set up coordinate system** for geographic area
2. **Store user locations** with privacy controls
3. **Track location history** with timestamp data
4. **Query user proximity** using coordinate comparison
5. **Validate coordinates** against defined boundaries

### Spatial Analysis
For geographic information systems:

1. **Configure map boundaries** for analysis area
2. **Store spatial data points** with high precision
3. **Query coordinate ranges** for analysis
4. **Track data timestamps** for temporal analysis
5. **Validate coordinate integrity** with boundary checks

## Advanced Features

### **Map Configuration Management**
- **Dynamic boundaries**: Update map boundaries as needed
- **Coordinate validation**: Automatic validation against boundaries
- **Precision control**: Configurable decimal precision
- **Dimension flexibility**: Support for 2D and 3D coordinates
- **Negative coordinate support**: Optional negative coordinate handling

### **Coordinate Storage System**
- **User association**: Link coordinates to user addresses
- **Timestamp tracking**: Record when coordinates are stored
- **Batch operations**: Store multiple coordinates efficiently
- **Coordinate updates**: Update existing coordinates with validation
- **Data persistence**: Reliable coordinate storage and retrieval

### **Spatial Query Capabilities**
- **Pagination support**: Efficient retrieval of large coordinate sets
- **User-specific queries**: Get coordinates for specific users
- **Map information access**: Query current map configuration
- **Point counting**: Track number of stored coordinates
- **History access**: Retrieve coordinate storage timestamps

### **Access Control**
- **Restricted operations**: Coordinate storage requires authorization
- **User-specific data**: Coordinates associated with user addresses
- **Administrative control**: Map configuration restricted to authorized users
- **Data integrity**: Validation ensures coordinate consistency
- **Privacy controls**: User-specific coordinate access

## Important Notes

- **Coordinate validation**: All coordinates validated against map boundaries
- **Precision handling**: Decimal precision controlled by map configuration
- **3D support**: Z-coordinate is optional for 2D applications
- **Negative coordinates**: Support controlled by allow_negative setting
- **Timestamp tracking**: Optional timestamp recording for coordinate history
- **Access restrictions**: Most operations require proper authorization
- **Map boundaries**: Coordinates must fall within defined map boundaries
- **User association**: Coordinates linked to specific user addresses

## Common Workflow

### 1. **Initialize Graph ADO**
```json
{
    "map_info": {
        "map_size": {
            "x_width": 1000,
            "y_width": 1000,
            "z_width": 100
        },
        "allow_negative": true,
        "map_decimal": 2
    }
}
```

### 2. **Store User Coordinate**
```json
{
    "store_coordinate": {
        "coordinate": {
            "x_coordinate": "250.50",
            "y_coordinate": "375.25",
            "z_coordinate": "50.75"
        },
        "is_timestamp_allowed": true
    }
}
```

### 3. **Query User Position**
```json
{
    "get_user_coordinate": {
        "user": "andr1useraddress..."
    }
}
```

### 4. **Update Map Configuration**
```json
{
    "update_map": {
        "map_info": {
            "map_size": {
                "x_width": 2000,
                "y_width": 2000,
                "z_width": 200
            },
            "allow_negative": true,
            "map_decimal": 3
        }
    }
}
```

The Graph ADO provides comprehensive spatial data management infrastructure for the Andromeda ecosystem, enabling coordinate storage, map configuration, and spatial analysis with flexible boundary validation and precision control.