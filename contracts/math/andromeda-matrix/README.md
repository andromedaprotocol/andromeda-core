# Andromeda Matrix ADO

## Introduction

The Andromeda Matrix ADO is a powerful mathematical utility contract that provides comprehensive matrix storage and computation capabilities. It supports matrix operations including addition, subtraction, multiplication, and storage management with multi-user support and key-based organization. The ADO performs automatic validation to ensure mathematical correctness and prevents overflow errors.

<b>Ado_type:</b> matrix

## Why Matrix ADO

The Matrix ADO serves as a fundamental mathematical tool for applications requiring:

- **Linear Algebra**: Perform matrix calculations for scientific and engineering applications
- **Machine Learning**: Store and manipulate weight matrices, transformation matrices
- **Computer Graphics**: Handle transformation matrices for 2D/3D graphics operations
- **Economic Modeling**: Store and calculate economic data matrices and relationships
- **Game Development**: Manage game state matrices, board representations, scoring systems
- **Scientific Computing**: Store experimental data in matrix format for analysis
- **Cryptography**: Handle cryptographic matrices and mathematical operations
- **Data Analysis**: Store and manipulate structured numerical data
- **Signal Processing**: Apply matrix operations for signal transformation and filtering
- **Optimization**: Store constraint matrices and optimization parameters

The ADO supports matrices of signed 64-bit integers with comprehensive validation and mathematical operation support.

## Mathematical Operations

### Supported Operations
- **Addition**: A + B (matrices must have same dimensions)
- **Subtraction**: A - B (matrices must have same dimensions)  
- **Multiplication**: A × B (columns of A must equal rows of B)
- **Storage**: Store/retrieve matrices with string keys
- **Validation**: Automatic validation of matrix structure and operations

### Matrix Requirements
- **Non-empty**: Matrices cannot be empty
- **Rectangular**: All rows must have the same number of columns
- **Integer Elements**: All elements must be signed 64-bit integers
- **Overflow Protection**: Operations check for overflow and prevent errors

## InstantiateMsg

```rust
pub struct InstantiateMsg {
    pub authorized_operator_addresses: Option<Vec<AndrAddr>>,
}
```

```json
{
    "authorized_operator_addresses": [
        "andr1user1address...",
        "andr1user2address..."
    ]
}
```

- **authorized_operator_addresses**: Optional list of addresses that can store and delete matrices
- If not provided, only the contract owner can perform matrix operations

## ExecuteMsg

### StoreMatrix
Stores a matrix with an optional string key for identification.

_**Note:** Only contract owner or authorized operators can execute this operation._

```rust
StoreMatrix {
    key: Option<String>,
    data: Matrix,
}

pub struct Matrix(pub Vec<Vec<i64>>);
```

```json
{
    "store_matrix": {
        "key": "transformation_matrix",
        "data": [
            [1, 0, 0],
            [0, 1, 0],
            [0, 0, 1]
        ]
    }
}
```

If no key is provided, a default key will be generated based on the sender's address.

### DeleteMatrix
Removes a stored matrix by its key.

_**Note:** Only contract owner or authorized operators can execute this operation._

```rust
DeleteMatrix {
    key: Option<String>,
}
```

```json
{
    "delete_matrix": {
        "key": "transformation_matrix"
    }
}
```

If no key is provided, deletes the default matrix for the sender.

## QueryMsg

### GetMatrix
Retrieves a stored matrix by its key.

```rust
pub enum QueryMsg {
    #[returns(GetMatrixResponse)]
    GetMatrix { key: Option<String> },
}
```

```json
{
    "get_matrix": {
        "key": "transformation_matrix"
    }
}
```

**Response:**
```json
{
    "key": "transformation_matrix",
    "data": [
        [1, 0, 0],
        [0, 1, 0],
        [0, 0, 1]
    ]
}
```

### AllKeys
Returns all matrix keys stored in the contract.

```rust
pub enum QueryMsg {
    #[returns(Vec<String>)]
    AllKeys {},
}
```

```json
{
    "all_keys": {}
}
```

**Response:**
```json
[
    "transformation_matrix",
    "data_matrix",
    "weight_matrix"
]
```

### OwnerKeys
Returns all matrix keys owned by a specific address.

```rust
pub enum QueryMsg {
    #[returns(Vec<String>)]
    OwnerKeys { owner: AndrAddr },
}
```

```json
{
    "owner_keys": {
        "owner": "andr1useraddress..."
    }
}
```

**Response:**
```json
[
    "user_matrix_1",
    "user_matrix_2"
]
```

## Matrix Operations (Off-Chain Calculation)

While the ADO stores matrices, applications can retrieve matrices and perform calculations using the built-in mathematical functions:

### Addition
```rust
let matrix_a = Matrix(vec![vec![1, 2], vec![3, 4]]);
let matrix_b = Matrix(vec![vec![5, 6], vec![7, 8]]);
let result = matrix_a.add(&matrix_b)?;
// Result: [[6, 8], [10, 12]]
```

### Subtraction
```rust
let matrix_a = Matrix(vec![vec![5, 6], vec![7, 8]]);
let matrix_b = Matrix(vec![vec![1, 2], vec![3, 4]]);
let result = matrix_a.sub(&matrix_b)?;
// Result: [[4, 4], [4, 4]]
```

### Multiplication
```rust
let matrix_a = Matrix(vec![vec![1, 2], vec![3, 4]]);
let matrix_b = Matrix(vec![vec![5, 6], vec![7, 8]]);
let result = matrix_a.mul(&matrix_b)?;
// Result: [[19, 22], [43, 50]]
```

## Usage Examples

### 2D Transformation Matrix
```json
{
    "store_matrix": {
        "key": "rotation_90",
        "data": [
            [0, -1],
            [1, 0]
        ]
    }
}
```

### 3D Identity Matrix
```json
{
    "store_matrix": {
        "key": "identity_3d",
        "data": [
            [1, 0, 0],
            [0, 1, 0],
            [0, 0, 1]
        ]
    }
}
```

### Game Score Matrix
```json
{
    "store_matrix": {
        "key": "player_scores",
        "data": [
            [100, 250, 180],
            [220, 190, 300],
            [150, 280, 210]
        ]
    }
}
```

### Neural Network Weights
```json
{
    "store_matrix": {
        "key": "layer1_weights",
        "data": [
            [15, -23, 8, 12],
            [7, 19, -5, 31],
            [-12, 4, 26, -9]
        ]
    }
}
```

## Integration Patterns

### With App Contract
The Matrix ADO can be integrated into App contracts for mathematical computations:

```json
{
    "components": [
        {
            "name": "math_engine",
            "ado_type": "matrix",
            "component_type": {
                "new": {
                    "authorized_operator_addresses": [
                        "andr1scientist1...",
                        "andr1engineer1..."
                    ]
                }
            }
        }
    ]
}
```

### Scientific Applications
For research and data analysis:

1. **Store experimental data** in matrix format
2. **Retrieve matrices** for computational analysis
3. **Perform calculations** using built-in mathematical functions
4. **Share data** with authorized researchers

### Gaming Applications
For game mechanics and scoring:

1. **Store game boards** as matrices
2. **Track player statistics** in matrix format
3. **Calculate game state** using matrix operations
4. **Manage leaderboards** with matrix data

### Machine Learning
For AI and ML applications:

1. **Store model weights** in matrices
2. **Track training data** in structured format
3. **Perform matrix calculations** for model operations
4. **Version control** for model parameters

## Matrix Validation Rules

### Structure Requirements
- **Non-empty**: Matrix must contain at least one element
- **Rectangular**: All rows must have identical column count
- **Integer values**: All elements must be valid i64 integers
- **Finite size**: Limited by blockchain storage constraints

### Operation Requirements
- **Addition/Subtraction**: Matrices must have identical dimensions
- **Multiplication**: Matrix A columns must equal Matrix B rows
- **Overflow protection**: All operations check for integer overflow

## Error Handling

### Common Validation Errors
- `"Matrix must not be empty"`
- `"All rows in the matrix must have the same number of columns"`
- `"Can not add or sub this matrix"` (dimension mismatch)
- `"Can not multiply this matrix"` (invalid dimensions for multiplication)

### Overflow Protection
All mathematical operations include overflow checking to prevent:
- Integer overflow during addition/multiplication
- Underflow during subtraction
- Invalid results from large number operations

## Storage and Access Control

### Key Management
- **Default keys**: Generated from sender address if not specified
- **Custom keys**: User-defined string identifiers
- **Namespaced**: Keys are unique across the entire contract
- **Owner tracking**: Each matrix tracks its owner for access control

### Authorization Levels
- **Contract Owner**: Full access to all operations and matrices
- **Authorized Operators**: Can store/delete matrices (set during instantiation)
- **Read Access**: Anyone can query stored matrices
- **Write Access**: Restricted to owner and authorized operators

## Performance Considerations

### Storage Efficiency
- **Integer storage**: Uses efficient i64 representation
- **Key indexing**: Fast key-based matrix retrieval
- **Batch operations**: Store multiple matrices in single transactions
- **Memory limits**: Large matrices may hit blockchain storage limits

### Computational Complexity
- **Addition/Subtraction**: O(m×n) for m×n matrices
- **Multiplication**: O(m×n×p) for m×n and n×p matrices
- **Validation**: O(m×n) for structure checking
- **Storage/Retrieval**: O(1) key-based access

## Important Notes

- **Immutable Operations**: Stored matrices don't change unless explicitly updated
- **Precision**: Uses signed 64-bit integers, no floating-point support
- **Concurrency**: Multiple users can store matrices simultaneously
- **Versioning**: No built-in versioning - use different keys for versions
- **Size Limits**: Practical limits imposed by blockchain storage costs

## Example Matrix Calculations

### 2×2 Matrix Addition
```
[1, 2]   [5, 6]   [6, 8]
[3, 4] + [7, 8] = [10, 12]
```

### 2×2 Matrix Multiplication
```
[1, 2]   [5, 6]   [19, 22]
[3, 4] × [7, 8] = [43, 50]
```

### 3×3 Identity Matrix
```
[1, 0, 0]
[0, 1, 0]  (Multiplication with any 3×3 matrix returns the original matrix)
[0, 0, 1]
```

The Matrix ADO provides a robust foundation for mathematical applications requiring matrix storage and computation, with comprehensive validation, error handling, and multi-user support that makes it suitable for scientific, gaming, and engineering applications.