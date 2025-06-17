# Andromeda Curve ADO

## Introduction

The Andromeda Curve ADO is a sophisticated mathematical utility contract that provides exponential curve calculations with configurable parameters. It supports both growth and decay curves, making it ideal for applications requiring dynamic pricing, bonding curves, reward scaling, or any scenario where exponential mathematical functions are needed.

<b>Ado_type:</b> curve

## Why Curve ADO

The Curve ADO serves as a powerful mathematical engine for applications requiring:

- **Dynamic Pricing Systems**: Implement bonding curves for token pricing based on supply
- **Reward Scaling**: Calculate exponentially increasing or decreasing rewards over time
- **Economic Models**: Model inflation, deflation, or compound interest scenarios
- **Gaming Mechanics**: Implement level-up costs, experience curves, or difficulty scaling
- **DeFi Applications**: Create automated market maker curves or liquidity pool pricing
- **Supply/Demand Modeling**: Calculate prices based on exponential supply and demand functions

The ADO supports both **Growth** curves (exponentially increasing) and **Decay** curves (exponentially decreasing), with customizable base values, multipliers, and constants.

## Mathematical Formula

The Curve ADO calculates Y values from X inputs using the exponential formula:

- **Growth Curve**: `Y = constant * (base^(multiplier * X))`
- **Decay Curve**: `Y = 1 / (constant * (base^(multiplier * X)))`

Where:
- **base_value**: The base of the exponential function
- **multiple_variable_value**: Multiplier for the X input (defaults to 1)
- **constant_value**: Constant multiplier (defaults to 1)

## InstantiateMsg

```rust
pub struct InstantiateMsg {
    pub curve_config: CurveConfig,
    pub authorized_operator_addresses: Option<Vec<AndrAddr>>,
}

pub enum CurveConfig {
    ExpConfig {
        curve_type: CurveType,
        base_value: u64,
        multiple_variable_value: Option<u64>,
        constant_value: Option<u64>,
    },
}

pub enum CurveType {
    Growth,
    Decay,
}
```

```json
{
    "curve_config": {
        "exp_config": {
            "curve_type": "Growth",
            "base_value": 2,
            "multiple_variable_value": 1,
            "constant_value": 1
        }
    },
    "authorized_operator_addresses": ["andr1..."]
}
```

- **curve_config**: Configuration for the exponential curve
  - **curve_type**: Either `"Growth"` (exponentially increasing) or `"Decay"` (exponentially decreasing)
  - **base_value**: Base of the exponential function (must be > 0)
  - **multiple_variable_value**: Multiplier for X input (defaults to 1 if not provided)
  - **constant_value**: Constant multiplier (defaults to 1 if not provided)
- **authorized_operator_addresses**: Optional list of addresses that can update the curve config

## ExecuteMsg

### UpdateCurveConfig
Updates the curve configuration parameters.

_**Note:** Only contract owner or authorized operators can execute this operation._

```rust
UpdateCurveConfig { 
    curve_config: CurveConfig 
}
```

```json
{
    "update_curve_config": {
        "curve_config": {
            "exp_config": {
                "curve_type": "Decay",
                "base_value": 3,
                "multiple_variable_value": 2,
                "constant_value": 5
            }
        }
    }
}
```

### Reset
Removes the current curve configuration from storage.

_**Note:** Only contract owner or authorized operators can execute this operation._

```rust
Reset {}
```

```json
{
    "reset": {}
}
```

## QueryMsg

### GetCurveConfig
Returns the current curve configuration.

```rust
pub enum QueryMsg {
    #[returns(GetCurveConfigResponse)]
    GetCurveConfig {},
}
```

```json
{
    "get_curve_config": {}
}
```

**Response:**
```json
{
    "curve_config": {
        "exp_config": {
            "curve_type": "Growth",
            "base_value": 2,
            "multiple_variable_value": 1,
            "constant_value": 1
        }
    }
}
```

### GetPlotYFromX
Calculates the Y value for a given X input using the configured curve.

```rust
pub enum QueryMsg {
    #[returns(GetPlotYFromXResponse)]
    GetPlotYFromX { x_value: u64 },
}
```

```json
{
    "get_plot_y_from_x": {
        "x_value": 3
    }
}
```

**Response:**
```json
{
    "y_value": "8.000000000000000000"
}
```

## Usage Examples

### Simple Growth Curve (Doubling)
```json
{
    "curve_config": {
        "exp_config": {
            "curve_type": "Growth",
            "base_value": 2,
            "multiple_variable_value": 1,
            "constant_value": 1
        }
    }
}
```
This creates a curve where Y = 2^X (1, 2, 4, 8, 16, ...)

### Token Bonding Curve
```json
{
    "curve_config": {
        "exp_config": {
            "curve_type": "Growth",
            "base_value": 2,
            "multiple_variable_value": 1,
            "constant_value": 100
        }
    }
}
```
This creates a pricing curve where Y = 100 * 2^X, useful for token bonding curves starting at 100 units.

### Decay Curve for Rewards
```json
{
    "curve_config": {
        "exp_config": {
            "curve_type": "Decay",
            "base_value": 2,
            "multiple_variable_value": 1,
            "constant_value": 1000
        }
    }
}
```
This creates a decay curve where rewards decrease exponentially: Y = 1000 / 2^X

### Custom Scaling Curve
```json
{
    "curve_config": {
        "exp_config": {
            "curve_type": "Growth",
            "base_value": 3,
            "multiple_variable_value": 2,
            "constant_value": 10
        }
    }
}
```
This creates Y = 10 * 3^(2*X), allowing for more complex scaling scenarios.

## Integration Patterns

### With App Contract
The Curve ADO can be integrated into App contracts for dynamic pricing or reward calculations:

```json
{
    "components": [
        {
            "name": "pricing_curve",
            "ado_type": "curve",
            "component_type": {
                "new": {
                    "curve_config": {
                        "exp_config": {
                            "curve_type": "Growth",
                            "base_value": 2,
                            "multiple_variable_value": 1,
                            "constant_value": 100
                        }
                    }
                }
            }
        }
    ]
}
```

### DeFi Integration
For automated market makers or bonding curve implementations:

1. **Query current supply** from token contract
2. **Calculate price** using `GetPlotYFromX` with supply as X
3. **Execute trades** based on calculated pricing

### Gaming Applications
For level-up cost calculations or experience curves:

1. **Query player level** from game state
2. **Calculate required XP** using curve with level as input
3. **Determine rewards** using decay curves

## Important Notes

- **Base value must be greater than 0** or instantiation will fail
- **Exponential calculations** can grow very large very quickly - test your parameters carefully
- **Decay curves** use division, so very small results may have precision limitations
- **Maximum exponent** is limited to u32::MAX to prevent overflow
- **Authorized operators** can update curve parameters if specified during instantiation

## Mathematical Behavior Examples

| X | Growth (2^X) | Decay (1/2^X) |
|---|--------------|---------------|
| 0 | 1           | 1.0           |
| 1 | 2           | 0.5           |
| 2 | 4           | 0.25          |
| 3 | 8           | 0.125         |
| 4 | 16          | 0.0625        |
| 10| 1024        | 0.0009765625  |