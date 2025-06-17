# Andromeda Curve ADO

## Introduction

The Andromeda Curve ADO is a sophisticated mathematical contract that implements exponential curve calculations for financial modeling, pricing mechanisms, and algorithmic functions. This contract enables precise computation of exponential growth and decay curves, supporting both upward and downward trending mathematical functions. The curve ADO is essential for implementing bonding curves, pricing algorithms, token economics, and any application requiring advanced mathematical curve calculations with configurable parameters.

<b>Ado_type:</b> curve

## Why Curve ADO

The Curve ADO serves as essential mathematical infrastructure for applications requiring:

- **Bonding Curves**: Implement token bonding curves for automated market makers and token pricing
- **Pricing Algorithms**: Dynamic pricing based on exponential mathematical functions
- **Token Economics**: Model token supply, inflation, and deflation mechanisms
- **DeFi Protocols**: Advanced DeFi features requiring curve-based calculations
- **Incentive Mechanisms**: Design incentive structures based on exponential rewards or penalties
- **Growth Modeling**: Model exponential growth patterns for business metrics
- **Decay Functions**: Implement decay mechanisms for time-based value reduction
- **Risk Management**: Calculate risk curves and probability distributions
- **Algorithmic Trading**: Implement sophisticated trading algorithms with curve-based logic
- **Economic Simulations**: Model complex economic behaviors through mathematical curves

The ADO provides precise mathematical calculations, configurable curve parameters, and operator-based access controls for reliable curve operations.

## Key Features

### **Exponential Curve Support**
- **Growth curves**: Exponential functions that increase over time
- **Decay curves**: Exponential functions that decrease over time
- **Configurable parameters**: Customizable base, multiple, and constant values
- **High precision**: Decimal-based calculations for accurate results
- **Mathematical safety**: Overflow and underflow protection for all operations

### **Flexible Configuration**
- **Base value**: Configure the base of the exponential function
- **Multiple variable**: Adjust the coefficient for the variable
- **Constant value**: Set the constant multiplier for the result
- **Curve type**: Choose between growth and decay behaviors
- **Dynamic updates**: Modify curve parameters without redeployment

### **Advanced Access Control**
- **Operator permissions**: Designated operators can modify curve configuration
- **Permission granularity**: Separate permissions for configuration updates and resets
- **Multi-operator support**: Multiple authorized operators for collaborative management
- **Owner privileges**: Contract owner maintains administrative control
- **Action-based permissions**: Fine-grained permissions for specific operations

### **Precise Calculations**
- **Real-time computation**: Calculate Y values for any X input
- **Decimal precision**: High-precision decimal arithmetic for accurate results
- **Mathematical validation**: Comprehensive validation of all parameters and operations
- **Edge case handling**: Proper handling of mathematical edge cases and limits

## Curve Mathematics

### **Exponential Function**
The curve implements the mathematical function:
```
Y = constant_value × (base_value ^ (multiple_variable_value × X))
```

### **Growth Curves**
For growth curves, the function returns the calculated result directly:
```
Y = C × (B ^ (M × X))
```

### **Decay Curves**
For decay curves, the function returns the inverse of the calculated result:
```
Y = 1 / (C × (B ^ (M × X)))
```

Where:
- **C**: Constant value (default: 1)
- **B**: Base value (required, must be > 0)
- **M**: Multiple variable value (default: 1)
- **X**: Input variable

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
            "curve_type": "growth",
            "base_value": 2,
            "multiple_variable_value": 3,
            "constant_value": 5
        }
    },
    "authorized_operator_addresses": [
        "andr1operator1...",
        "andr1operator2..."
    ]
}
```

**Parameters**:
- **curve_config**: Configuration for the exponential curve
  - **curve_type**: Type of curve (Growth or Decay)
  - **base_value**: Base of the exponential function (must be > 0)
  - **multiple_variable_value**: Coefficient for the variable (default: 1)
  - **constant_value**: Constant multiplier (default: 1)
- **authorized_operator_addresses**: Optional list of addresses authorized to modify curve config
  - If provided, only these addresses can update configuration
  - If empty or null, only contract owner can modify configuration

**Validation**:
- Base value must be greater than 0
- All values must fit within u64 range
- Authorized addresses must be valid Andromeda addresses

## ExecuteMsg

### UpdateCurveConfig
Updates the curve configuration parameters.

```rust
UpdateCurveConfig {
    curve_config: CurveConfig,
}
```

```json
{
    "update_curve_config": {
        "curve_config": {
            "exp_config": {
                "curve_type": "decay",
                "base_value": 3,
                "multiple_variable_value": 2,
                "constant_value": 10
            }
        }
    }
}
```

**Authorization**: Only authorized operators or contract owner can execute
**Validation**: New curve configuration must pass validation checks
**Effect**: Replaces current curve configuration with new parameters

### Reset
Removes the current curve configuration from storage.

```rust
Reset {}
```

```json
{
    "reset": {}
}
```

**Authorization**: Only authorized operators or contract owner can execute
**Effect**: Removes curve configuration from contract storage
**Use Cases**: Reset contract to initial state or clear invalid configurations

## QueryMsg

### GetCurveConfig
Returns the current curve configuration.

```rust
#[returns(GetCurveConfigResponse)]
GetCurveConfig {}

pub struct GetCurveConfigResponse {
    pub curve_config: CurveConfig,
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
            "curve_type": "growth",
            "base_value": 2,
            "multiple_variable_value": 3,
            "constant_value": 5
        }
    }
}
```

### GetPlotYFromX
Calculates the Y value for a given X input using the configured curve.

```rust
#[returns(GetPlotYFromXResponse)]
GetPlotYFromX { x_value: u64 }

pub struct GetPlotYFromXResponse {
    pub y_value: String,
}
```

```json
{
    "get_plot_y_from_x": {
        "x_value": 4
    }
}
```

**Response:**
```json
{
    "y_value": "20480.0"
}
```

**Calculation Example**:
For the above configuration with X = 4:
- Growth curve: Y = 5 × (2 ^ (3 × 4)) = 5 × (2 ^ 12) = 5 × 4096 = 20,480

## Usage Examples

### Simple Growth Curve
```json
{
    "curve_config": {
        "exp_config": {
            "curve_type": "growth",
            "base_value": 2,
            "multiple_variable_value": 1,
            "constant_value": 1
        }
    },
    "authorized_operator_addresses": null
}
```

### Token Bonding Curve
```json
{
    "curve_config": {
        "exp_config": {
            "curve_type": "growth",
            "base_value": 110,
            "multiple_variable_value": 1,
            "constant_value": 100
        }
    },
    "authorized_operator_addresses": [
        "andr1treasury_manager..."
    ]
}
```

### Decay Function for Time-Based Value
```json
{
    "curve_config": {
        "exp_config": {
            "curve_type": "decay",
            "base_value": 2,
            "multiple_variable_value": 1,
            "constant_value": 1000
        }
    },
    "authorized_operator_addresses": [
        "andr1protocol_operator..."
    ]
}
```

### Dynamic Pricing Algorithm
```json
{
    "curve_config": {
        "exp_config": {
            "curve_type": "growth",
            "base_value": 105,
            "multiple_variable_value": 2,
            "constant_value": 50
        }
    },
    "authorized_operator_addresses": [
        "andr1pricing_bot...",
        "andr1admin..."
    ]
}
```

## Calculation Examples

### Growth Curve Example
Configuration:
- Base value: 3
- Multiple variable: 2
- Constant value: 5
- Type: Growth

For X = 3:
```
Y = 5 × (3 ^ (2 × 3))
Y = 5 × (3 ^ 6)
Y = 5 × 729
Y = 3645
```

### Decay Curve Example
Configuration:
- Base value: 2
- Multiple variable: 1
- Constant value: 100
- Type: Decay

For X = 5:
```
Y = 1 / (100 × (2 ^ (1 × 5)))
Y = 1 / (100 × (2 ^ 5))
Y = 1 / (100 × 32)
Y = 1 / 3200
Y = 0.0003125
```

## Operational Examples

### Calculate Curve Value
```json
{
    "get_plot_y_from_x": {
        "x_value": 10
    }
}
```

### Update Curve Parameters
```json
{
    "update_curve_config": {
        "curve_config": {
            "exp_config": {
                "curve_type": "growth",
                "base_value": 5,
                "multiple_variable_value": 1,
                "constant_value": 2
            }
        }
    }
}
```

### Reset Configuration
```json
{
    "reset": {}
}
```

### Query Current Configuration
```json
{
    "get_curve_config": {}
}
```

## Integration Patterns

### With App Contract
Curve calculations can be integrated for pricing and economics:

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
                            "curve_type": "growth",
                            "base_value": 110,
                            "multiple_variable_value": 1,
                            "constant_value": 100
                        }
                    },
                    "authorized_operator_addresses": null
                }
            }
        },
        {
            "name": "decay_curve",
            "ado_type": "curve",
            "component_type": {
                "new": {
                    "curve_config": {
                        "exp_config": {
                            "curve_type": "decay",
                            "base_value": 2,
                            "multiple_variable_value": 1,
                            "constant_value": 1000
                        }
                    },
                    "authorized_operator_addresses": ["./pricing_manager"]
                }
            }
        }
    ]
}
```

### Token Bonding Curves
For implementing automated market makers:

1. **Deploy curve contract** with appropriate growth parameters
2. **Set authorized operators** for treasury management
3. **Query curve values** for token pricing calculations
4. **Update parameters** based on market conditions
5. **Integrate with AMM** for automated trading

### Dynamic Pricing Systems
For implementing dynamic pricing mechanisms:

1. **Configure growth curves** for increasing demand scenarios
2. **Set multiple operators** for different pricing components
3. **Calculate prices** based on current market variables
4. **Update curve parameters** based on market feedback
5. **Monitor and adjust** pricing strategies

### Incentive Mechanisms
For designing incentive and reward systems:

1. **Use decay curves** for diminishing returns over time
2. **Configure growth curves** for increasing rewards
3. **Calculate rewards** based on participation levels
4. **Adjust parameters** to optimize incentive alignment
5. **Track and analyze** incentive effectiveness

## Advanced Features

### **Mathematical Precision**
- **Decimal arithmetic**: High-precision calculations using Decimal types
- **Overflow protection**: Safe mathematical operations prevent overflow errors
- **Underflow protection**: Proper handling of very small values and division
- **Edge case handling**: Robust handling of mathematical edge cases

### **Flexible Parameters**
- **Optional values**: Default values for optional parameters
- **Dynamic configuration**: Update curve parameters without redeployment
- **Validation logic**: Comprehensive validation of all curve parameters
- **Type safety**: Strong typing prevents invalid curve configurations

### **Access Control Management**
- **Multi-operator support**: Multiple authorized operators for collaborative management
- **Action-based permissions**: Granular permissions for specific operations
- **Permission inheritance**: Contract owner maintains ultimate control
- **Operator management**: Dynamic operator authorization and revocation

### **Curve Versatility**
- **Growth and decay**: Support for both increasing and decreasing functions
- **Configurable steepness**: Adjust curve steepness through variable parameters
- **Scaling factors**: Apply constant multipliers for result scaling
- **Mathematical flexibility**: Support for various exponential curve shapes

## Security Features

### **Access Control**
- **Operator validation**: Verify operator permissions before configuration changes
- **Owner privileges**: Contract owner can always modify configuration
- **Permission granularity**: Separate permissions for different operations
- **Unauthorized prevention**: Prevent unauthorized access to sensitive operations

### **Mathematical Security**
- **Overflow prevention**: Prevent integer overflow in exponential calculations
- **Underflow protection**: Safe handling of very small decimal values
- **Input validation**: Comprehensive validation of all mathematical inputs
- **Precision maintenance**: Maintain mathematical precision throughout calculations

### **Configuration Protection**
- **Parameter validation**: Validate all curve parameters before storage
- **State consistency**: Maintain consistent state across configuration changes
- **Error handling**: Graceful handling of invalid configurations
- **Atomic updates**: All configuration changes are atomic operations

### **Calculation Safety**
- **Safe exponentiation**: Protected exponentiation operations
- **Division safety**: Safe division operations with zero-check protection
- **Type conversion**: Safe conversion between numerical types
- **Boundary checking**: Proper handling of numerical boundaries and limits

## Important Notes

- **Base value requirement**: Base value must be greater than 0 for valid exponential calculations
- **Default values**: Multiple variable and constant default to 1 if not specified
- **Precision limits**: Calculations are limited by decimal precision and u64 ranges
- **Exponent limits**: Exponent values are limited to u32 range for mathematical operations
- **Operator permissions**: Authorized operators can only modify configuration, not query results
- **Growth vs decay**: Decay curves return the inverse of the exponential calculation
- **Mathematical accuracy**: Results are returned as decimal strings for precision
- **Configuration persistence**: Curve configuration persists until explicitly modified or reset

## Common Workflow

### 1. **Deploy Curve Contract**
```json
{
    "curve_config": {
        "exp_config": {
            "curve_type": "growth",
            "base_value": 2,
            "multiple_variable_value": 1,
            "constant_value": 1
        }
    },
    "authorized_operator_addresses": null
}
```

### 2. **Calculate Initial Values**
```json
{
    "get_plot_y_from_x": {
        "x_value": 1
    }
}
```

### 3. **Query Current Configuration**
```json
{
    "get_curve_config": {}
}
```

### 4. **Update Curve Parameters**
```json
{
    "update_curve_config": {
        "curve_config": {
            "exp_config": {
                "curve_type": "growth",
                "base_value": 3,
                "multiple_variable_value": 2,
                "constant_value": 5
            }
        }
    }
}
```

### 5. **Calculate New Values**
```json
{
    "get_plot_y_from_x": {
        "x_value": 5
    }
}
```

### 6. **Reset if Needed**
```json
{
    "reset": {}
}
```

The Curve ADO provides essential mathematical infrastructure for advanced curve calculations in the Andromeda ecosystem, enabling sophisticated financial modeling, pricing algorithms, and mathematical functions with precise decimal arithmetic and comprehensive security controls.