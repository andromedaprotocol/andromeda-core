# Andromeda Distance ADO

## Introduction

The Andromeda Distance ADO is a mathematical utility contract that provides distance calculations between coordinate points. It supports both 2D and 3D coordinate systems and can calculate Euclidean (straight-line) distances and Manhattan distances with configurable decimal precision.

<b>Ado_type:</b> distance

## Why Distance ADO

The Distance ADO serves as a fundamental mathematical tool for applications requiring:

- **Location-Based Services**: Calculate distances between geographic coordinates
- **Gaming Applications**: Determine distances between players, objects, or locations in game worlds
- **Logistics and Routing**: Calculate travel distances for delivery optimization
- **Spatial Analysis**: Analyze spatial relationships in data visualization
- **Physics Simulations**: Calculate distances for collision detection or movement
- **Recommendation Systems**: Use distance metrics for similarity calculations
- **Geofencing**: Determine if points are within specified distance ranges
- **Navigation Systems**: Calculate routes and proximity-based features
- **Scientific Computing**: Perform spatial analysis and geometric calculations

The ADO supports both **Euclidean distance** (straight-line) and **Manhattan distance** (grid-based) calculations with customizable precision.

## Mathematical Formulas

### Euclidean Distance (Straight-Line)
- **2D**: `d = √((x₂-x₁)² + (y₂-y₁)²)`
- **3D**: `d = √((x₂-x₁)² + (y₂-y₁)² + (z₂-z₁)²)`

### Manhattan Distance (Grid-Based)
- **2D**: `d = |x₂-x₁| + |y₂-y₁|`
- **3D**: `d = |x₂-x₁| + |y₂-y₁| + |z₂-z₁|`

## InstantiateMsg

```rust
pub struct InstantiateMsg {}
```

```json
{}
```

The Distance ADO requires no configuration parameters during instantiation. It's a stateless utility that provides distance calculation services.

## ExecuteMsg

The Distance ADO has no execute messages - it's a read-only utility contract that provides distance calculations through queries only.

## QueryMsg

### GetDistanceBetween2Points
Calculates the Euclidean (straight-line) distance between two coordinate points.

```rust
pub enum QueryMsg {
    #[returns(String)]
    GetDistanceBetween2Points {
        point_1: Coordinate,
        point_2: Coordinate,
        decimal: u16,
    },
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
            "x_coordinate": "1.5",
            "y_coordinate": "2.0",
            "z_coordinate": "3.0"
        },
        "point_2": {
            "x_coordinate": "4.5",
            "y_coordinate": "6.0",
            "z_coordinate": "7.0"
        },
        "decimal": 2
    }
}
```

**Response:** Returns distance as a string with specified decimal precision
```json
"6.48"
```

### GetManhattanDistance
Calculates the Manhattan (grid-based) distance between two coordinate points.

```rust
pub enum QueryMsg {
    #[returns(String)]
    GetManhattanDistance {
        point_1: Coordinate,
        point_2: Coordinate,
        decimal: u16,
    },
}
```

```json
{
    "get_manhattan_distance": {
        "point_1": {
            "x_coordinate": "1.0",
            "y_coordinate": "1.0"
        },
        "point_2": {
            "x_coordinate": "4.0",
            "y_coordinate": "5.0"
        },
        "decimal": 1
    }
}
```

**Response:** Returns Manhattan distance as a string
```json
"7.0"
```

## Coordinate System

### 2D Coordinates
For 2D calculations, omit the `z_coordinate` field:
```json
{
    "x_coordinate": "10.5",
    "y_coordinate": "-5.2"
}
```

### 3D Coordinates
For 3D calculations, include the `z_coordinate` field:
```json
{
    "x_coordinate": "10.5",
    "y_coordinate": "-5.2",
    "z_coordinate": "7.8"
}
```

### Coordinate Properties
- **x_coordinate**: Signed decimal value for X-axis position
- **y_coordinate**: Signed decimal value for Y-axis position
- **z_coordinate**: Optional signed decimal value for Z-axis position (3D calculations)
- **decimal**: Number of decimal places in the result (0-18)

## Usage Examples

### 2D Euclidean Distance
Calculate straight-line distance between two 2D points:
```json
{
    "get_distance_between2_points": {
        "point_1": {
            "x_coordinate": "0",
            "y_coordinate": "0"
        },
        "point_2": {
            "x_coordinate": "3",
            "y_coordinate": "4"
        },
        "decimal": 2
    }
}
```
Result: `"5.00"` (classic 3-4-5 triangle)

### 3D Euclidean Distance
Calculate straight-line distance in 3D space:
```json
{
    "get_distance_between2_points": {
        "point_1": {
            "x_coordinate": "1.0",
            "y_coordinate": "1.0",
            "z_coordinate": "1.0"
        },
        "point_2": {
            "x_coordinate": "2.0",
            "y_coordinate": "2.0",
            "z_coordinate": "2.0"
        },
        "decimal": 3
    }
}
```
Result: `"1.732"` (√3)

### Manhattan Distance (Grid Movement)
Calculate grid-based distance (like moving on city blocks):
```json
{
    "get_manhattan_distance": {
        "point_1": {
            "x_coordinate": "0",
            "y_coordinate": "0"
        },
        "point_2": {
            "x_coordinate": "3",
            "y_coordinate": "4"
        },
        "decimal": 0
    }
}
```
Result: `"7"` (3 + 4 = 7 blocks)

### Geographic Coordinates
Using decimal degrees for latitude/longitude:
```json
{
    "get_distance_between2_points": {
        "point_1": {
            "x_coordinate": "-74.006",
            "y_coordinate": "40.7128"
        },
        "point_2": {
            "x_coordinate": "-118.2437",
            "y_coordinate": "34.0522"
        },
        "decimal": 2
    }
}
```

## Integration Patterns

### With App Contract
The Distance ADO can be integrated into App contracts for spatial calculations:

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

### Gaming Applications
For game mechanics involving movement and positioning:

1. **Calculate movement range** using Euclidean distance
2. **Determine grid movement** using Manhattan distance
3. **Implement proximity detection** for game objects
4. **Calculate attack ranges** and ability areas of effect

### Location Services
For geographic and mapping applications:

1. **Find nearby locations** using coordinate distance
2. **Calculate delivery routes** using Manhattan distance for city grids
3. **Implement geofencing** by checking if points are within distance thresholds
4. **Rank search results** by proximity

### Physics and Simulation
For scientific and physics applications:

1. **Collision detection** using distance thresholds
2. **Force calculations** based on distance between objects
3. **Spatial clustering** for data analysis
4. **Path optimization** for movement systems

## Distance Type Comparison

| Scenario | Euclidean Distance | Manhattan Distance |
|----------|-------------------|-------------------|
| **Straight-line travel** | ✓ Accurate | ✗ Overestimate |
| **City grid movement** | ✗ Underestimate | ✓ Accurate |
| **Flight distance** | ✓ Accurate | ✗ Not applicable |
| **Taxi distance** | ✗ Underestimate | ✓ Accurate |
| **Game grid movement** | ✗ Unrealistic | ✓ Realistic |
| **Geometric analysis** | ✓ Standard | ✗ Special cases |

## Important Notes

- **Read-Only Contract**: The Distance ADO has no execute messages and maintains no state
- **Precision Control**: Decimal parameter controls output precision (0-18 decimal places)
- **Signed Coordinates**: Supports negative coordinate values for full coordinate plane coverage
- **3D Support**: Automatically detects 2D vs 3D based on presence of z_coordinate
- **String Output**: Results returned as strings to preserve decimal precision
- **Performance**: Calculations are performed on-chain with high precision

## Example Calculations

### 2D Examples
- **Origin to (3,4)**: Euclidean = 5.0, Manhattan = 7.0
- **(-1,-1) to (2,3)**: Euclidean = 5.0, Manhattan = 7.0
- **(0,0) to (1,1)**: Euclidean = 1.414, Manhattan = 2.0

### 3D Examples
- **(0,0,0) to (1,1,1)**: Euclidean = 1.732, Manhattan = 3.0
- **(1,2,3) to (4,6,8)**: Euclidean = 7.071, Manhattan = 12.0

The Distance ADO provides accurate, reliable distance calculations for any application requiring spatial mathematics, making it a valuable utility for games, mapping, logistics, and scientific applications.