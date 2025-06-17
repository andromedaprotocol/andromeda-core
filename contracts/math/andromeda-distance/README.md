# Andromeda Distance ADO

## Introduction

The Andromeda Distance ADO is a mathematical utility contract that provides distance calculations between points in 2D and 3D coordinate systems. This contract offers both Euclidean (straight-line) and Manhattan distance calculations with configurable decimal precision, making it essential for spatial applications, location-based services, proximity calculations, and geometric analysis. The distance calculations support signed decimal coordinates and optional z-axis for three-dimensional calculations.

<b>Ado_type:</b> distance

## Why Distance ADO

The Distance ADO serves as essential mathematical infrastructure for applications requiring:

- **Spatial Analysis**: Calculate distances for geographic and spatial analysis
- **Location Services**: Implement location-based services and proximity features
- **Game Development**: Calculate distances for game mechanics and collision detection
- **Route Planning**: Support route planning and navigation systems
- **Proximity Detection**: Detect proximity between entities or locations
- **Clustering Analysis**: Support clustering algorithms with distance calculations
- **Geometric Calculations**: Perform geometric analysis and computations
- **Mapping Applications**: Support mapping and cartographic applications
- **Sensor Data Processing**: Process spatial sensor data and measurements
- **Resource Optimization**: Optimize resource allocation based on spatial proximity

The ADO provides precise distance calculations with multiple distance metrics and configurable precision.

## Key Features

### **Multiple Distance Metrics**
- **Euclidean distance**: Straight-line distance between two points
- **Manhattan distance**: Sum of absolute differences of coordinates (taxicab distance)
- **2D and 3D support**: Optional z-coordinate for three-dimensional calculations
- **Signed coordinates**: Support for positive and negative coordinate values
- **High precision**: Configurable decimal precision for accurate calculations

### **Flexible Coordinate System**
- **SignedDecimal coordinates**: Support for precise decimal coordinates
- **Optional third dimension**: Z-coordinate support for 3D calculations
- **Coordinate validation**: Automatic validation of coordinate inputs
- **Flexible precision**: Configurable decimal places for output formatting
- **Mathematical accuracy**: Precise calculations with overflow protection

### **Read-Only Operations**
- **Stateless calculations**: Pure mathematical functions without state storage
- **Gas efficient**: Minimal gas usage for distance calculations
- **No configuration**: Simple utility contract requiring no setup
- **Instant results**: Fast calculations with immediate responses
- **Public access**: Available to all users without restrictions

## Distance Calculation Methods

### **Euclidean Distance**
Calculates the straight-line distance between two points:
```
distance = √[(x₂-x₁)² + (y₂-y₁)² + (z₂-z₁)²]
```

### **Manhattan Distance**
Calculates the sum of absolute differences of coordinates:
```
distance = |x₂-x₁| + |y₂-y₁| + |z₂-z₁|
```

### **Coordinate Support**
- **2D coordinates**: x and y coordinates (z is optional)
- **3D coordinates**: x, y, and z coordinates
- **Signed values**: Support for negative coordinates
- **Decimal precision**: High precision decimal calculations

## InstantiateMsg

```rust
pub struct InstantiateMsg {}
```

```json
{}
```

**Parameters**: None - the distance ADO requires no configuration
**Deployment**: Simple deployment with standard Andromeda ADO parameters
**State**: Stateless contract - performs calculations only

## ExecuteMsg

The Distance ADO has no execute messages - it is a read-only utility contract.

```rust
pub enum ExecuteMsg {}
```

## QueryMsg

### GetDistanceBetween2Points
Calculates the Euclidean (straight-line) distance between two points.

```rust
#[returns(String)]
GetDistanceBetween2Points {
    point_1: Coordinate,
    point_2: Coordinate,
    decimal: u16,
}

pub struct Coordinate {
    pub x_coordinate: SignedDecimal,
    pub y_coordinate: SignedDecimal,
    pub z_coordinate: Option<SignedDecimal>,
}
```

```json
{
    "get_distance_between2_points": {
        "point_1": {
            "x_coordinate": "10.5",
            "y_coordinate": "20.3",
            "z_coordinate": "5.0"
        },
        "point_2": {
            "x_coordinate": "15.2",
            "y_coordinate": "25.1",
            "z_coordinate": "8.5"
        },
        "decimal": 2
    }
}
```

**Response:**
```json
"7.35"
```

**Parameters**:
- **point_1**: First coordinate point
- **point_2**: Second coordinate point  
- **decimal**: Number of decimal places for the result

### GetManhattanDistance
Calculates the Manhattan distance between two points.

```rust
#[returns(String)]
GetManhattanDistance {
    point_1: Coordinate,
    point_2: Coordinate,
    decimal: u16,
}
```

```json
{
    "get_manhattan_distance": {
        "point_1": {
            "x_coordinate": "0",
            "y_coordinate": "0",
            "z_coordinate": null
        },
        "point_2": {
            "x_coordinate": "3",
            "y_coordinate": "4",
            "z_coordinate": null
        },
        "decimal": 1
    }
}
```

**Response:**
```json
"7.0"
```

## Usage Examples

### 2D Euclidean Distance
```json
{
    "get_distance_between2_points": {
        "point_1": {
            "x_coordinate": "0",
            "y_coordinate": "0",
            "z_coordinate": null
        },
        "point_2": {
            "x_coordinate": "3",
            "y_coordinate": "4",
            "z_coordinate": null
        },
        "decimal": 2
    }
}
```
**Result**: "5.00" (3-4-5 triangle)

### 3D Euclidean Distance
```json
{
    "get_distance_between2_points": {
        "point_1": {
            "x_coordinate": "1",
            "y_coordinate": "2",
            "z_coordinate": "3"
        },
        "point_2": {
            "x_coordinate": "4",
            "y_coordinate": "6",
            "z_coordinate": "8"
        },
        "decimal": 3
    }
}
```

### Manhattan Distance with Decimals
```json
{
    "get_manhattan_distance": {
        "point_1": {
            "x_coordinate": "1.5",
            "y_coordinate": "2.3",
            "z_coordinate": "0.5"
        },
        "point_2": {
            "x_coordinate": "4.2",
            "y_coordinate": "1.8",
            "z_coordinate": "3.1"
        },
        "decimal": 4
    }
}
```

### Negative Coordinates
```json
{
    "get_distance_between2_points": {
        "point_1": {
            "x_coordinate": "-5",
            "y_coordinate": "-3",
            "z_coordinate": null
        },
        "point_2": {
            "x_coordinate": "2",
            "y_coordinate": "4",
            "z_coordinate": null
        },
        "decimal": 1
    }
}
```

## Integration Patterns

### With App Contract
Distance calculations can be integrated for spatial functionality:

```json
{
    "components": [
        {
            "name": "distance_calculator",
            "ado_type": "distance",
            "component_type": {
                "new": {}
            }
        }
    ]
}
```

### Location-Based Services
For proximity and location calculations:

1. **Store user locations** as coordinates
2. **Calculate distances** between users or points of interest
3. **Implement proximity alerts** using distance thresholds
4. **Find nearest locations** through distance comparisons
5. **Build location-aware features** using distance metrics

### Game Development
For game mechanics and spatial calculations:

1. **Calculate movement distances** for game characters
2. **Implement collision detection** using distance calculations
3. **Determine weapon ranges** and area effects
4. **Build spatial AI** for NPC behavior
5. **Create location-based gameplay** mechanics

### Route Planning
For navigation and route optimization:

1. **Calculate route segments** using distance calculations
2. **Compare route alternatives** based on total distance
3. **Implement pathfinding algorithms** with distance metrics
4. **Optimize delivery routes** using distance analysis
5. **Build navigation systems** with distance feedback

### Clustering Analysis
For data analysis and clustering:

1. **Calculate distances** between data points
2. **Implement clustering algorithms** using distance metrics
3. **Analyze spatial patterns** in data sets
4. **Group similar items** based on spatial proximity
5. **Build recommendation systems** using distance similarity

## Advanced Features

### **Multiple Distance Metrics**
- **Euclidean distance**: Most natural distance measurement
- **Manhattan distance**: Useful for grid-based calculations
- **3D support**: Full three-dimensional distance calculations
- **Metric selection**: Choose appropriate metric for use case
- **Mathematical accuracy**: Precise calculations with proper rounding

### **High Precision Calculations**
- **SignedDecimal support**: High precision decimal arithmetic
- **Configurable precision**: Control output decimal places
- **Overflow protection**: Safe arithmetic operations
- **Accurate rounding**: Proper rounding to specified decimal places
- **Mathematical consistency**: Consistent results across calculations

### **Flexible Coordinate System**
- **2D and 3D support**: Optional third dimension
- **Signed coordinates**: Support for negative values
- **Decimal precision**: High precision coordinate values
- **Coordinate validation**: Automatic input validation
- **Flexible input formats**: Support various coordinate formats

### **Performance Optimization**
- **Stateless operations**: No state storage overhead
- **Gas efficiency**: Minimal computational cost
- **Fast calculations**: Optimized mathematical operations
- **Direct responses**: Immediate calculation results
- **Scalable usage**: Suitable for high-frequency calculations

## Important Notes

- **Stateless contract**: Performs calculations only, no data storage
- **Decimal precision**: Output precision controlled by decimal parameter
- **3D calculations**: Z-coordinate is optional for 2D calculations
- **Signed coordinates**: Supports both positive and negative values
- **Mathematical accuracy**: Uses high-precision decimal arithmetic
- **No configuration**: Contract requires no setup or configuration
- **Public access**: Available to all users without restrictions
- **Gas efficient**: Minimal gas usage for calculations

## Common Workflow

### 1. **Deploy Distance ADO**
```json
{}
```

### 2. **Calculate 2D Distance**
```json
{
    "get_distance_between2_points": {
        "point_1": {
            "x_coordinate": "0",
            "y_coordinate": "0",
            "z_coordinate": null
        },
        "point_2": {
            "x_coordinate": "3",
            "y_coordinate": "4",
            "z_coordinate": null
        },
        "decimal": 2
    }
}
```

### 3. **Calculate Manhattan Distance**
```json
{
    "get_manhattan_distance": {
        "point_1": {
            "x_coordinate": "1",
            "y_coordinate": "1",
            "z_coordinate": null
        },
        "point_2": {
            "x_coordinate": "4",
            "y_coordinate": "5",
            "z_coordinate": null
        },
        "decimal": 1
    }
}
```

### 4. **Use in Application Logic**
```javascript
// Example application usage
const distance = await query({
    "get_distance_between2_points": {
        "point_1": userLocation,
        "point_2": targetLocation,
        "decimal": 2
    }
});

if (parseFloat(distance) < 10.0) {
    // User is within 10 units of target
    triggerProximityAlert();
}
```

The Distance ADO provides essential mathematical infrastructure for the Andromeda ecosystem, enabling precise spatial calculations, location-based services, and geometric analysis with high precision and multiple distance metrics.