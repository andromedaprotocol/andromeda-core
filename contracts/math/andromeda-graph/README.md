# Andromeda Graph ADO

## Introduction

The Andromeda Graph ADO is a sophisticated coordinate mapping and spatial data management contract that provides configurable 2D/3D coordinate storage with user tracking capabilities. It enables applications to create spatial maps, track user locations, store coordinate points with timestamps, and manage spatial data with precision control and boundary validation.

<b>Ado_type:</b> graph

## Why Graph ADO

The Graph ADO serves as a powerful spatial data engine for applications requiring:

- **Location Tracking**: Track user positions in 2D/3D space with timestamp support
- **Gaming Worlds**: Manage player locations, object positions, and spatial game mechanics
- **Geographic Information Systems**: Store and manage geographic coordinate data
- **Asset Tracking**: Monitor physical or digital asset locations over time
- **Spatial Analytics**: Perform spatial analysis and coordinate-based calculations
- **Virtual Reality**: Manage object positions in VR/AR environments
- **Supply Chain**: Track item locations throughout logistics networks
- **Scientific Data**: Store experimental coordinate data with temporal information
- **Map Applications**: Create custom maps with user-defined coordinate systems
- **IoT Device Tracking**: Monitor sensor and device locations in real-time

The ADO supports both 2D and 3D coordinate systems with configurable boundaries, decimal precision, and optional negative coordinate support.

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
        "map_decimal": 6
    }
}
```

- **map_info**: Configuration for the coordinate system
  - **map_size**: Defines the boundaries of the coordinate space
    - **x_width**: Maximum X-axis boundary (positive direction)
    - **y_width**: Maximum Y-axis boundary (positive direction)  
    - **z_width**: Optional maximum Z-axis boundary for 3D coordinates
  - **allow_negative**: Whether negative coordinates are allowed
  - **map_decimal**: Decimal precision for coordinate values (0-18)

## ExecuteMsg

### UpdateMap
Updates the map configuration including boundaries and precision settings.

_**Note:** Only contract owner can execute this operation._

```rust
UpdateMap { 
    map_info: MapInfo 
}
```

```json
{
    "update_map": {
        "map_info": {
            "map_size": {
                "x_width": 2000,
                "y_width": 2000,
                "z_width": 200
            },
            "allow_negative": false,
            "map_decimal": 8
        }
    }
}
```

### StoreCoordinate
Stores a coordinate point in the map with optional timestamp recording.

_**Note:** Only contract owner can execute this operation._

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
            "x_coordinate": "45.123456",
            "y_coordinate": "-122.654321",
            "z_coordinate": "100.5"
        },
        "is_timestamp_allowed": true
    }
}
```

### StoreUserCoordinate
Associates coordinate data with specific user addresses using location paths.

_**Note:** Only contract owner can execute this operation._

```rust
StoreUserCoordinate { 
    user_location_paths: Vec<AndrAddr> 
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
Removes stored coordinate data for a specific user.

_**Note:** Only contract owner can execute this operation._

```rust
DeleteUserCoordinate { 
    user: AndrAddr 
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
Returns the current map configuration including boundaries and settings.

```rust
pub enum QueryMsg {
    #[returns(GetMapInfoResponse)]
    GetMapInfo {},
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
        "map_decimal": 6
    }
}
```

### GetMaxPointNumber
Returns the maximum number of points that can be stored in the current map configuration.

```rust
pub enum QueryMsg {
    #[returns(GetMaxPointNumberResponse)]
    GetMaxPointNumber {},
}
```

```json
{
    "get_max_point_number": {}
}
```

**Response:**
```json
{
    "max_point_number": "200000000"
}
```

### GetAllPoints
Returns all stored coordinate points with pagination support.

```rust
pub enum QueryMsg {
    #[returns(GetAllPointsResponse)]
    GetAllPoints {
        start: Option<u128>,
        limit: Option<u32>,
    },
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

**Response:**
```json
{
    "points": [
        [
            {
                "x": "45.123456",
                "y": "-122.654321",
                "z": "100.5"
            },
            {
                "timestamp": 1640995200
            }
        ]
    ]
}
```

### GetUserCoordinate
Returns the coordinate information for a specific user.

```rust
pub enum QueryMsg {
    #[returns(CoordinateInfo)]
    GetUserCoordinate { user: AndrAddr },
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
    "x": "45.123456",
    "y": "-122.654321",
    "z": "100.5"
}
```

## Coordinate System Examples

### 2D Geographic Map
```json
{
    "map_info": {
        "map_size": {
            "x_width": 180,
            "y_width": 90
        },
        "allow_negative": true,
        "map_decimal": 6
    }
}
```
Suitable for latitude/longitude coordinates (-180 to +180, -90 to +90).

### 3D Game World
```json
{
    "map_info": {
        "map_size": {
            "x_width": 1000,
            "y_width": 1000,
            "z_width": 256
        },
        "allow_negative": false,
        "map_decimal": 2
    }
}
```
Suitable for a 1000x1000x256 block game world with centimeter precision.

### Indoor Positioning System
```json
{
    "map_info": {
        "map_size": {
            "x_width": 100,
            "y_width": 50,
            "z_width": 10
        },
        "allow_negative": false,
        "map_decimal": 3
    }
}
```
Suitable for tracking positions within a 100x50x10 meter building.

### Scientific Data Collection
```json
{
    "map_info": {
        "map_size": {
            "x_width": 1000000,
            "y_width": 1000000,
            "z_width": 1000000
        },
        "allow_negative": true,
        "map_decimal": 12
    }
}
```
Suitable for high-precision scientific measurements with micrometer accuracy.

## Usage Examples

### Store Location Data
```json
{
    "store_coordinate": {
        "coordinate": {
            "x_coordinate": "37.7749",
            "y_coordinate": "-122.4194"
        },
        "is_timestamp_allowed": true
    }
}
```

### Track User Movement
```json
{
    "store_user_coordinate": {
        "user_location_paths": ["andr1player1..."]
    }
}
```

### Query User Position
```json
{
    "get_user_coordinate": {
        "user": "andr1player1..."
    }
}
```

## Integration Patterns

### With App Contract
The Graph ADO can be integrated into App contracts for spatial data management:

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
                            "x_width": 1000,
                            "y_width": 1000,
                            "z_width": 100
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
For multiplayer games with spatial mechanics:

1. **Initialize game world** with appropriate boundaries
2. **Track player positions** using StoreUserCoordinate
3. **Query nearby players** using GetAllPoints with pagination
4. **Update world boundaries** as the game world expands

### Asset Tracking
For supply chain and logistics:

1. **Configure map** for geographic tracking
2. **Store asset locations** with timestamps
3. **Query asset history** using GetAllPoints
4. **Track multiple assets** per user/company

### IoT and Sensor Networks
For device monitoring:

1. **Set up coordinate system** for sensor deployment area
2. **Store sensor locations** during deployment
3. **Track mobile devices** with user coordinate updates
4. **Monitor coverage areas** with spatial queries

## Boundary Validation

### Coordinate Limits
- **X-axis**: 0 to x_width (or -x_width to +x_width if negative allowed)
- **Y-axis**: 0 to y_width (or -y_width to +y_width if negative allowed)  
- **Z-axis**: 0 to z_width (or -z_width to +z_width if negative allowed)

### Precision Control
- **map_decimal**: Controls decimal places (0-18)
- **Higher precision**: Better accuracy but more storage
- **Lower precision**: Less storage but reduced accuracy

## Important Notes

- **Owner-Controlled**: Only contract owner can modify map data and configuration
- **Timestamp Support**: Optional timestamp recording for temporal tracking
- **Pagination**: Large datasets supported through paginated queries
- **3D Optional**: Z-coordinate is optional for 2D applications
- **Boundary Enforcement**: Coordinates must fall within configured map boundaries
- **User Association**: Coordinates can be associated with specific user addresses
- **Data Persistence**: All coordinate data persists until explicitly deleted

## Performance Considerations

- **Large Maps**: Consider pagination for maps with many coordinate points
- **Precision Trade-offs**: Higher decimal precision increases storage requirements
- **Query Optimization**: Use pagination limits to manage query response sizes
- **Memory Usage**: Map size affects maximum storage capacity

The Graph ADO provides a comprehensive solution for spatial data management in blockchain applications, offering the flexibility and precision needed for complex coordinate-based systems while maintaining efficient storage and query capabilities.