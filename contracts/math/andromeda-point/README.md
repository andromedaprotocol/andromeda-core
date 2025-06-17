# Andromeda Point ADO

## Introduction

The Andromeda Point ADO is a coordinate storage and management contract that provides secure storage of 2D and 3D point coordinates with configurable access control. This contract enables applications to store, update, and retrieve spatial coordinates with three access control modes: Private (owner-only), Public (read-only for all), and Restricted (configurable access permissions). The Point ADO supports high-precision decimal coordinates with optional z-axis for three-dimensional applications.

<b>Ado_type:</b> point

## Why Point ADO

The Point ADO serves as essential spatial data infrastructure for applications requiring:

- **Secure Coordinate Storage**: Store spatial coordinates with access control
- **Location Management**: Manage user or entity locations with privacy controls
- **Spatial Data Services**: Provide coordinate storage for location-based applications
- **Gaming Applications**: Store player positions, object locations, and world coordinates
- **Geographic Information**: Handle geographic coordinates with precision
- **Asset Tracking**: Track asset locations with configurable access permissions
- **Privacy-Controlled Location**: Store location data with fine-grained access control
- **Coordinate Validation**: Ensure coordinate data integrity and access permissions
- **Multi-dimensional Support**: Handle both 2D and 3D coordinate systems
- **Data Ownership**: Clear ownership model for stored coordinate data

The ADO provides secure coordinate storage with flexible access control and high-precision coordinate support.

## Key Features

### **Access Control Modes**
- **Private**: Only the owner can read and write coordinates
- **Public**: Anyone can read coordinates, only owner can write
- **Restricted**: Configurable access permissions for read/write operations
- **Owner permissions**: Coordinate owner has full control
- **Flexible security**: Adapt access control to application requirements

### **Coordinate Storage**
- **2D/3D support**: Optional z-coordinate for three-dimensional coordinates
- **High precision**: SignedDecimal support for accurate coordinate values
- **Data validation**: Ensure coordinate data integrity
- **Single point storage**: One coordinate per contract instance
- **Update capability**: Update existing coordinates with access control

### **Data Management**
- **Owner tracking**: Track who owns the stored coordinate data
- **Point updates**: Update coordinates while maintaining ownership
- **Point deletion**: Remove stored coordinates when needed
- **Access restriction updates**: Modify access control settings
- **Data integrity**: Ensure coordinate consistency and validation

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

**Parameters**:
- **restriction**: Access control mode for the coordinate data
  - **Private**: Only owner can read/write coordinates
  - **Public**: Anyone can read, only owner can write
  - **Restricted**: Custom access permissions (configurable)

## ExecuteMsg

### SetPoint
Sets or updates the stored coordinate point.

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
            "x_coordinate": "123.456",
            "y_coordinate": "-78.901",
            "z_coordinate": "45.123"
        }
    }
}
```

**Parameters**:
- **point**: Coordinate data to store
  - **x_coordinate**: X-axis coordinate value
  - **y_coordinate**: Y-axis coordinate value
  - **z_coordinate**: Optional Z-axis coordinate for 3D applications

### DeletePoint
Deletes the stored coordinate point (nonpayable).

```rust
DeletePoint {}
```

```json
{
    "delete_point": {}
}
```

**Authorization**: Only the coordinate owner can delete the point
**Effect**: Removes all stored coordinate data from the contract

### UpdateRestriction
Updates the access control restriction mode (restricted, nonpayable).

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

**Parameters**:
- **restriction**: New access control mode
**Authorization**: Restricted operation (typically owner-only)
**Effect**: Changes how others can access the stored coordinate

## QueryMsg

### GetPoint
Retrieves the stored coordinate point.

```rust
#[returns(PointCoordinate)]
GetPoint {}
```

```json
{
    "get_point": {}
}
```

**Response:**
```json
{
    "x_coordinate": "123.456",
    "y_coordinate": "-78.901", 
    "z_coordinate": "45.123"
}
```

**Access Control**: 
- **Private**: Only owner can query
- **Public**: Anyone can query
- **Restricted**: Based on configured permissions

### GetDataOwner
Returns the owner of the stored coordinate data.

```rust
#[returns(GetDataOwnerResponse)]
GetDataOwner {}

pub struct GetDataOwnerResponse {
    pub owner: AndrAddr,
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
    "owner": "andr1owneraddress..."
}
```

## Usage Examples

### Private Location Storage
```json
{
    "restriction": "Private"
}
```
Use case: Personal location data, private coordinates

### Public Location Sharing
```json
{
    "restriction": "Public"
}
```
Use case: Public landmarks, shared points of interest

### Store 2D Coordinate
```json
{
    "set_point": {
        "point": {
            "x_coordinate": "40.7128",
            "y_coordinate": "-74.0060",
            "z_coordinate": null
        }
    }
}
```

### Store 3D Coordinate
```json
{
    "set_point": {
        "point": {
            "x_coordinate": "100.5",
            "y_coordinate": "200.3",
            "z_coordinate": "150.8"
        }
    }
}
```

### Update Access Control
```json
{
    "update_restriction": {
        "restriction": "Public"
    }
}
```

### Query Location
```json
{
    "get_point": {}
}
```

### Check Ownership
```json
{
    "get_data_owner": {}
}
```

## Integration Patterns

### With App Contract
Point storage for application coordinates:

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
        }
    ]
}
```

### Location-Based Services
For location and position management:

1. **Store user locations** with appropriate privacy settings
2. **Track asset positions** with configurable access control
3. **Manage waypoints** for navigation applications
4. **Store points of interest** with public access
5. **Handle private coordinates** for sensitive location data

### Gaming Applications
For player and object positioning:

1. **Store player positions** with privacy controls
2. **Track NPC locations** with appropriate access
3. **Manage spawn points** with public/restricted access
4. **Store treasure locations** with private access
5. **Handle checkpoint coordinates** for game progression

### Asset Tracking
For tracking physical or digital assets:

1. **Store asset locations** with owner-controlled access
2. **Track delivery coordinates** with restricted access
3. **Manage facility locations** with public access
4. **Store checkpoint positions** for logistics
5. **Handle security coordinates** with private access

### Geographic Information Systems
For spatial data management:

1. **Store survey points** with professional access controls
2. **Manage landmark coordinates** with public access
3. **Track environmental data points** with restricted access
4. **Store measurement locations** with private access
5. **Handle reference coordinates** for mapping

## Advanced Features

### **Flexible Access Control**
- **Three access modes**: Private, Public, and Restricted permissions
- **Owner privileges**: Full control over coordinate data and access
- **Dynamic permissions**: Update access control as needed
- **Read/write separation**: Different permissions for reading vs writing
- **Security flexibility**: Adapt access control to application needs

### **High-Precision Coordinates**
- **SignedDecimal support**: High precision decimal coordinate values
- **2D/3D compatibility**: Optional third dimension for 3D applications
- **Negative coordinates**: Support for negative coordinate values
- **Coordinate validation**: Ensure data integrity and format compliance
- **Precision preservation**: Maintain coordinate accuracy through storage

### **Data Ownership and Management**
- **Clear ownership**: Track who owns the stored coordinate data
- **Owner control**: Owners can update coordinates and access restrictions
- **Data lifecycle**: Create, update, and delete coordinate data
- **Ownership queries**: Query coordinate ownership information
- **Access validation**: Enforce access control based on ownership

### **Spatial Data Integration**
- **Single point focus**: Optimized for storing one coordinate per contract
- **Coordinate updates**: Update stored coordinates while maintaining ownership
- **Data consistency**: Ensure coordinate data remains valid and accessible
- **Integration ready**: Easy integration with other spatial ADOs
- **Flexible usage**: Support various coordinate systems and applications

## Important Notes

- **Single coordinate**: Each contract instance stores one coordinate point
- **Access control**: Three modes (Private, Public, Restricted) for flexible security
- **Owner permissions**: Coordinate owner has full control over data and access
- **2D/3D support**: Z-coordinate is optional for 2D applications
- **High precision**: Uses SignedDecimal for accurate coordinate values
- **Data integrity**: Coordinates validated and stored securely
- **Dynamic access**: Access control can be updated by authorized users
- **Ownership tracking**: Clear ownership model for stored coordinates

## Common Workflow

### 1. **Deploy Point ADO**
```json
{
    "restriction": "Private"
}
```

### 2. **Store Coordinate**
```json
{
    "set_point": {
        "point": {
            "x_coordinate": "40.7128",
            "y_coordinate": "-74.0060",
            "z_coordinate": null
        }
    }
}
```

### 3. **Query Coordinate**
```json
{
    "get_point": {}
}
```

### 4. **Update Access Control**
```json
{
    "update_restriction": {
        "restriction": "Public"
    }
}
```

### 5. **Check Ownership**
```json
{
    "get_data_owner": {}
}
```

### 6. **Update Coordinate**
```json
{
    "set_point": {
        "point": {
            "x_coordinate": "40.7589",
            "y_coordinate": "-73.9851",
            "z_coordinate": "10.0"
        }
    }
}
```

The Point ADO provides secure coordinate storage infrastructure for the Andromeda ecosystem, enabling spatial data management with flexible access control, high precision coordinates, and clear ownership models.