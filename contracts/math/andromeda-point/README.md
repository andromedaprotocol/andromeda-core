# Andromeda Point ADO

## Introduction

The Andromeda Point ADO is a versatile coordinate storage contract that provides secure 2D/3D point storage with configurable access control. It enables applications to store, retrieve, and manage single coordinate points with data ownership tracking, making it ideal for location services, spatial markers, user positions, and any application requiring persistent coordinate data with permission management.

<b>Ado_type:</b> point

## Why Point ADO

The Point ADO serves as a fundamental building block for applications requiring:

- **User Location Storage**: Store individual user positions in 2D or 3D space
- **Landmark Management**: Mark important locations with precise coordinates
- **Gaming Applications**: Store player spawn points, item locations, or checkpoint coordinates
- **Asset Positioning**: Track fixed asset locations with coordinate precision
- **Mapping Services**: Store points of interest, waypoints, or navigation markers
- **Scientific Data**: Record measurement locations or observation points
- **Virtual Environments**: Manage object positions in VR/AR applications
- **Geolocation Services**: Store user preferences for location-based services
- **Spatial Anchors**: Create reference points for spatial applications
- **Data Visualization**: Store coordinate points for charts and graphs

The ADO supports both 2D and 3D coordinates with three access control modes: **Private** (owner only), **Public** (anyone can set), and **Restricted** (configurable permissions).

## InstantiateMsg

```rust
pub struct InstantiateMsg {
    pub restriction: PointRestriction,
}

pub enum PointRestriction {
    Private,
    Public,
    Restricted,
}
```

```json
{
    "restriction": "Private"
}
```

- **restriction**: Controls who can modify the stored point
  - `"Private"`: Only contract owner can set the point
  - `"Public"`: Anyone can set the point
  - `"Restricted"`: Uses Andromeda's advanced permission system for access control

## ExecuteMsg

### SetPoint
Sets the coordinate point stored in the contract.

_**Note:** Permission requirements depend on the restriction setting configured during instantiation._

```rust
SetPoint {
    point: PointCoordinate,
}

pub struct PointCoordinate {
    pub x_coordinate: SignedDecimal,
    pub y_coordinate: SignedDecimal,
    pub z_coordinate: Option<SignedDecimal>,
}
```

```json
{
    "set_point": {
        "point": {
            "x_coordinate": "37.7749",
            "y_coordinate": "-122.4194",
            "z_coordinate": "150.5"
        }
    }
}
```

### DeletePoint
Removes the stored coordinate point from the contract.

_**Note:** Permission requirements depend on the restriction setting._

```rust
DeletePoint {}
```

```json
{
    "delete_point": {}
}
```

### UpdateRestriction
Updates the access restriction for point operations.

_**Note:** Only contract owner can execute this operation._

```rust
UpdateRestriction {
    restriction: PointRestriction,
}
```

```json
{
    "update_restriction": {
        "restriction": "Public"
    }
}
```

## QueryMsg

### GetPoint
Returns the current coordinate point stored in the contract.

```rust
pub enum QueryMsg {
    #[returns(PointCoordinate)]
    GetPoint {},
}
```

```json
{
    "get_point": {}
}
```

**Response:**
```json
{
    "x_coordinate": "37.7749",
    "y_coordinate": "-122.4194",
    "z_coordinate": "150.5"
}
```

### GetDataOwner
Returns the address that currently owns the point data (who set the point).

```rust
pub enum QueryMsg {
    #[returns(GetDataOwnerResponse)]
    GetDataOwner {},
}
```

```json
{
    "get_data_owner": {}
}
```

**Response:**
```json
{
    "owner": "andr1useraddress..."
}
```

## Coordinate Systems

### 2D Coordinates
For 2D applications, omit the `z_coordinate` field:
```json
{
    "x_coordinate": "longitude_or_x",
    "y_coordinate": "latitude_or_y"
}
```

### 3D Coordinates
For 3D applications, include the `z_coordinate` field:
```json
{
    "x_coordinate": "longitude_or_x",
    "y_coordinate": "latitude_or_y", 
    "z_coordinate": "altitude_or_z"
}
```

### Coordinate Properties
- **x_coordinate**: Signed decimal value for X-axis position (supports negative values)
- **y_coordinate**: Signed decimal value for Y-axis position (supports negative values)
- **z_coordinate**: Optional signed decimal value for Z-axis position (3D coordinates)
- **High Precision**: Supports high-precision decimal values for accurate positioning

## Access Control Modes

### Private Mode
- **Who can set point**: Only contract owner
- **Use case**: Personal location storage, private markers, owner-controlled points
- **Example**: User's home location, private bookmarks

```json
{
    "restriction": "Private"
}
```

### Public Mode
- **Who can set point**: Anyone
- **Use case**: Community markers, shared locations, collaborative mapping
- **Example**: Public landmarks, community points of interest

```json
{
    "restriction": "Public"
}
```

### Restricted Mode
- **Who can set point**: Uses Andromeda's permission system
- **Use case**: Role-based access, managed locations, controlled sharing
- **Example**: Team collaboration points, managed asset locations

```json
{
    "restriction": "Restricted"
}
```

## Usage Examples

### Geographic Location (San Francisco)
```json
{
    "set_point": {
        "point": {
            "x_coordinate": "-122.4194",
            "y_coordinate": "37.7749"
        }
    }
}
```

### 3D Game Position
```json
{
    "set_point": {
        "point": {
            "x_coordinate": "1250.75",
            "y_coordinate": "890.25",
            "z_coordinate": "64.0"
        }
    }
}
```

### Scientific Measurement Point
```json
{
    "set_point": {
        "point": {
            "x_coordinate": "0.000001",
            "y_coordinate": "-0.000002",
            "z_coordinate": "0.000003"
        }
    }
}
```

### Building Floor Position
```json
{
    "set_point": {
        "point": {
            "x_coordinate": "150.5",
            "y_coordinate": "75.2",
            "z_coordinate": "3.0"
        }
    }
}
```

## Integration Patterns

### With App Contract
The Point ADO can be integrated into App contracts for coordinate management:

```json
{
    "components": [
        {
            "name": "user_location",
            "ado_type": "point",
            "component_type": {
                "new": {
                    "restriction": "Private"
                }
            }
        },
        {
            "name": "shared_marker",
            "ado_type": "point",
            "component_type": {
                "new": {
                    "restriction": "Public"
                }
            }
        }
    ]
}
```

### Location-Based Services
For mapping and navigation applications:

1. **Deploy Point ADO** for each user or location marker
2. **Set restriction mode** based on privacy requirements
3. **Store user locations** or points of interest
4. **Query coordinates** for proximity calculations or navigation

### Gaming Applications
For game world management:

1. **Create Point ADOs** for spawn points, checkpoints, or item locations
2. **Set coordinates** for fixed game elements
3. **Track player positions** with private Point ADOs
4. **Share discoveries** using public Point ADOs

### Asset Tracking
For physical or digital asset management:

1. **Deploy Point ADO** for each tracked asset
2. **Store asset coordinates** with appropriate access control
3. **Update locations** as assets move
4. **Query ownership** for audit trails

### Scientific Data Collection
For research and measurement applications:

1. **Store measurement locations** with high precision
2. **Track data collection points** over time
3. **Share research coordinates** with collaborators
4. **Maintain data ownership** records

## Coordinate Precision

### High-Precision Support
- **Decimal places**: Supports high precision decimal values
- **Scientific notation**: Can handle very small or large coordinate values
- **Signed values**: Supports both positive and negative coordinates
- **Range**: Full range of signed decimal values supported

### Precision Examples
- **GPS coordinates**: 6-8 decimal places for meter/centimeter accuracy
- **Scientific measurements**: 12+ decimal places for micrometer accuracy
- **Game coordinates**: 2-3 decimal places for smooth movement
- **Building positions**: 3-4 decimal places for millimeter accuracy

## State Management

### Point Lifecycle
1. **Unset**: Initial state, no point stored
2. **Set**: Coordinate point stored (2D or 3D)
3. **Updated**: Point coordinates modified
4. **Deleted**: Point removed, returns to unset state

### Data Ownership
- **Data Owner**: Address that most recently set the point
- **Contract Owner**: Address that owns the contract (different from data owner)
- **Permissions**: Based on restriction mode and contract ownership

## Error Handling

### Common Errors
- **Unauthorized**: Attempting to set point without proper permissions
- **No Point Set**: Querying point when none has been set
- **Invalid Coordinates**: Malformed decimal values
- **Permission Denied**: Restriction mode prevents operation

### Permission Validation
The contract validates permissions based on:
1. **Restriction mode** set during instantiation
2. **Contract ownership** for restricted operations
3. **Andromeda permission system** for advanced access control

## Important Notes

- **Single Point Storage**: Stores only one coordinate point per contract instance
- **Data Ownership Tracking**: Tracks who set the current point
- **Flexible Precision**: Supports any precision level for coordinates
- **2D/3D Support**: Automatically handles 2D or 3D based on z_coordinate presence
- **Immutable Until Updated**: Point remains constant until explicitly changed
- **No History**: Only current point is stored, no historical tracking

## Example Workflows

### Personal Location Storage
```bash
# Deploy with private restriction
# User sets their location
{"set_point": {"point": {"x_coordinate": "10.5", "y_coordinate": "20.3"}}}

# Query current location
{"get_point": {}}
# Response: {"x_coordinate": "10.5", "y_coordinate": "20.3", "z_coordinate": null}

# Update location
{"set_point": {"point": {"x_coordinate": "15.7", "y_coordinate": "25.1"}}}
```

### Shared Landmark
```bash
# Deploy with public restriction
# User A sets landmark
{"set_point": {"point": {"x_coordinate": "100.0", "y_coordinate": "200.0"}}}

# User B updates landmark (overwrites A's point)
{"set_point": {"point": {"x_coordinate": "101.0", "y_coordinate": "201.0"}}}

# Check who set the current point
{"get_data_owner": {}}
# Response: {"owner": "andr1...user_b_address"}
```

### 3D Asset Position
```bash
# Set 3D position for an asset
{"set_point": {"point": {
    "x_coordinate": "45.123456",
    "y_coordinate": "-122.654321", 
    "z_coordinate": "150.5"
}}}

# Query 3D position
{"get_point": {}}
# Response: {"x_coordinate": "45.123456", "y_coordinate": "-122.654321", "z_coordinate": "150.5"}
```

The Point ADO provides a simple, secure way to store and manage coordinate points in blockchain applications, with flexible access controls and precision that can adapt to various use cases from personal location storage to high-precision scientific measurements.