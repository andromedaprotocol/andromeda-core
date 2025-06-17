# Andromeda Matrix ADO

## Introduction

The Andromeda Matrix ADO is a mathematical utility contract that provides matrix storage, validation, and arithmetic operations. This contract enables applications to store and manipulate matrices with support for addition, subtraction, and multiplication operations. The Matrix ADO includes comprehensive validation to ensure mathematical consistency, overflow protection, and key-based matrix storage with user ownership tracking. It supports standard matrix operations with automatic validation and error handling.

<b>Ado_type:</b> matrix

## Why Matrix ADO

The Matrix ADO serves as essential mathematical infrastructure for applications requiring:

- **Linear Algebra Operations**: Perform matrix addition, subtraction, and multiplication
- **Mathematical Modeling**: Support mathematical models requiring matrix calculations
- **Data Analysis**: Process structured numerical data in matrix format
- **Game Development**: Handle transformation matrices for graphics and physics
- **Scientific Computing**: Support scientific calculations and simulations
- **Machine Learning**: Provide matrix operations for ML algorithms and models
- **Statistical Analysis**: Perform statistical calculations on matrix data
- **Engineering Applications**: Support engineering calculations and modeling
- **Financial Modeling**: Handle financial calculations requiring matrix operations
- **Computer Graphics**: Process transformation and projection matrices

The ADO provides robust matrix operations with validation, error handling, and overflow protection.

## Key Features

### **Matrix Arithmetic Operations**
- **Matrix addition**: Add matrices of compatible dimensions
- **Matrix subtraction**: Subtract matrices of compatible dimensions  
- **Matrix multiplication**: Multiply matrices with proper dimension validation
- **Dimension validation**: Automatic validation of matrix dimensions for operations
- **Overflow protection**: Safe arithmetic operations with overflow detection

### **Matrix Storage System**
- **Key-based storage**: Store matrices with optional custom keys
- **User ownership**: Track matrix ownership by user address
- **Matrix validation**: Ensure matrices have consistent row lengths
- **Authorized operations**: Configurable authorized operators for matrix operations
- **Multiple matrices**: Store and manage multiple matrices per user

### **Data Integrity and Validation**
- **Matrix structure validation**: Ensure all rows have equal column count
- **Non-empty validation**: Prevent storage of empty matrices
- **Dimension compatibility**: Validate matrix dimensions for arithmetic operations
- **Mathematical consistency**: Ensure operations follow mathematical rules
- **Error handling**: Comprehensive error reporting for invalid operations

## Matrix Operations

### **Addition and Subtraction**
Matrices must have identical dimensions:
```
A + B = C where A[i,j] + B[i,j] = C[i,j]
A - B = C where A[i,j] - B[i,j] = C[i,j]
```

### **Matrix Multiplication**
Number of columns in first matrix must equal number of rows in second:
```
A(m×n) × B(n×p) = C(m×p)
C[i,j] = Σ(k=0 to n-1) A[i,k] × B[k,j]
```

### **Matrix Validation Rules**
- **Non-empty**: Matrices cannot be empty
- **Rectangular**: All rows must have the same number of columns
- **Compatible dimensions**: Operations require compatible matrix dimensions
- **Overflow safety**: All arithmetic operations check for overflow

## InstantiateMsg

```rust
pub struct InstantiateMsg {
    pub authorized_operator_addresses: Option<Vec<AndrAddr>>,
}
```

```json
{
    "authorized_operator_addresses": [
        "andr1operator1address...",
        "andr1operator2address..."
    ]
}
```

**Parameters**:
- **authorized_operator_addresses**: Optional list of addresses authorized to perform matrix operations
- **Default behavior**: If not specified, standard Andromeda access controls apply
- **Multiple operators**: Support for multiple authorized operators

## ExecuteMsg

### StoreMatrix
Stores a matrix with an optional key (nonpayable).

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
            [1, 2, 3],
            [4, 5, 6],
            [7, 8, 9]
        ]
    }
}
```

**Parameters**:
- **key**: Optional identifier for the matrix (auto-generated if not provided)
- **data**: Matrix data as 2D array of integers
- **Validation**: Matrix structure validated before storage

### DeleteMatrix
Deletes a stored matrix by key (nonpayable).

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

**Parameters**:
- **key**: Optional matrix key (uses default key if not provided)
- **Authorization**: Only matrix owner or authorized operators can delete

## QueryMsg

### GetMatrix
Retrieves a stored matrix by key.

```rust
#[returns(GetMatrixResponse)]
GetMatrix {
    key: Option<String>,
}

pub struct GetMatrixResponse {
    pub key: String,
    pub data: Matrix,
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
        [1, 2, 3],
        [4, 5, 6],
        [7, 8, 9]
    ]
}
```

### AllKeys
Returns all matrix keys stored in the contract.

```rust
#[returns(Vec<String>)]
AllKeys {}
```

```json
{
    "all_keys": {}
}
```

**Response:**
```json
[
    "matrix_1",
    "transformation_matrix",
    "rotation_matrix"
]
```

### OwnerKeys
Returns all matrix keys owned by a specific address.

```rust
#[returns(Vec<String>)]
OwnerKeys {
    owner: AndrAddr,
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
    "user_transformation"
]
```

## Usage Examples

### Identity Matrix Storage
```json
{
    "store_matrix": {
        "key": "identity_3x3",
        "data": [
            [1, 0, 0],
            [0, 1, 0],
            [0, 0, 1]
        ]
    }
}
```

### 2x2 Matrix for Transformations
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

### Matrix Arithmetic (Off-chain)
```rust
// Example matrix operations (implemented in contract logic)
let matrix_a = Matrix(vec![vec![1, 2], vec![3, 4]]);
let matrix_b = Matrix(vec![vec![5, 6], vec![7, 8]]);

// Addition: [[6, 8], [10, 12]]
let sum = matrix_a.add(&matrix_b)?;

// Multiplication: [[19, 22], [43, 50]]  
let product = matrix_a.mul(&matrix_b)?;
```

### Query User Matrices
```json
{
    "owner_keys": {
        "owner": "andr1useraddress..."
    }
}
```

## Integration Patterns

### With App Contract
Matrix operations for mathematical applications:

```json
{
    "components": [
        {
            "name": "matrix_calculator",
            "ado_type": "matrix", 
            "component_type": {
                "new": {
                    "authorized_operator_addresses": [
                        "andr1calculatorservice..."
                    ]
                }
            }
        }
    ]
}
```

### Linear Algebra Applications
For mathematical modeling and calculations:

1. **Store coefficient matrices** for linear equation systems
2. **Perform matrix operations** for solving mathematical problems
3. **Handle transformation matrices** for geometric operations
4. **Process data matrices** for statistical analysis
5. **Manage user-specific matrices** with ownership tracking

### Game Development
For graphics and physics calculations:

1. **Store transformation matrices** for object positioning
2. **Handle rotation matrices** for object orientation
3. **Process projection matrices** for rendering
4. **Manage physics matrices** for collision detection
5. **Store animation matrices** for movement calculations

### Scientific Computing
For research and simulation applications:

1. **Store experimental data** in matrix format
2. **Perform statistical calculations** on matrix data
3. **Handle simulation matrices** for modeling
4. **Process correlation matrices** for data analysis
5. **Manage computation results** with matrix storage

## Advanced Features

### **Matrix Validation System**
- **Structure validation**: Ensure rectangular matrix format
- **Dimension checking**: Validate compatibility for operations
- **Non-empty validation**: Prevent storage of empty matrices
- **Mathematical consistency**: Ensure operations follow mathematical rules
- **Error reporting**: Detailed error messages for validation failures

### **Arithmetic Operations with Safety**
- **Overflow detection**: Protect against integer overflow in operations
- **Dimension validation**: Automatic validation for matrix operations
- **Mathematical accuracy**: Precise calculation results
- **Error handling**: Comprehensive error reporting for invalid operations
- **Operation chaining**: Support for complex mathematical workflows

### **Storage and Ownership Management**
- **Key-based access**: Flexible matrix identification system
- **User ownership**: Track matrix ownership by address
- **Authorized operations**: Configurable operator permissions
- **Multiple matrices**: Support for storing multiple matrices per user
- **Key enumeration**: Query all keys or user-specific keys

### **Access Control and Security**
- **Authorized operators**: Configurable list of authorized addresses
- **Owner permissions**: Matrix owners can perform all operations
- **Operation restrictions**: Control who can modify matrices
- **Data integrity**: Ensure only valid matrices are stored
- **Permission validation**: Check permissions before operations

## Important Notes

- **Matrix format**: Matrices stored as 2D arrays of 64-bit signed integers
- **Dimension requirements**: All rows must have identical column count
- **Overflow protection**: All arithmetic operations check for overflow
- **Key management**: Keys are optional (auto-generated if not provided)
- **Owner tracking**: Matrices associated with storing user address
- **Operation validation**: Mathematical rules enforced for all operations
- **Non-empty requirement**: Matrices cannot be empty
- **Authorized operators**: Can be configured during instantiation

## Common Workflow

### 1. **Deploy Matrix ADO**
```json
{
    "authorized_operator_addresses": [
        "andr1calculatorservice..."
    ]
}
```

### 2. **Store Matrices**
```json
{
    "store_matrix": {
        "key": "matrix_a",
        "data": [
            [1, 2, 3],
            [4, 5, 6]
        ]
    }
}
```

### 3. **Retrieve Matrix**
```json
{
    "get_matrix": {
        "key": "matrix_a"
    }
}
```

### 4. **Query User Matrices**
```json
{
    "owner_keys": {
        "owner": "andr1useraddress..."
    }
}
```

### 5. **Perform Operations (Application Logic)**
```rust
// Application would retrieve matrices and perform operations
let matrix_a = get_matrix("matrix_a")?;
let matrix_b = get_matrix("matrix_b")?;
let result = matrix_a.add(&matrix_b)?;
```

The Matrix ADO provides essential mathematical infrastructure for the Andromeda ecosystem, enabling matrix storage, validation, and arithmetic operations with comprehensive safety features and access control.