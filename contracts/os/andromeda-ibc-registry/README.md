# Andromeda IBC Registry ADO

## Introduction

The Andromeda IBC Registry ADO is a specialized registry contract that manages Inter-Blockchain Communication (IBC) denomination information. This ADO stores and validates IBC token denoms, providing a central lookup service for cross-chain asset management with comprehensive validation of IBC denomination formats, trace paths, and checksums for secure multi-chain token operations.

<b>Ado_type:</b> ibc-registry

## Why IBC Registry ADO

The IBC Registry ADO serves as a critical infrastructure component for cross-chain applications requiring:

- **IBC Token Management**: Centralized registry for all IBC token denominations
- **Cross-Chain Asset Tracking**: Track tokens across multiple blockchain networks
- **Denomination Validation**: Ensure IBC denom format compliance and integrity
- **Token Metadata Storage**: Store trace paths and base denomination information
- **Asset Discovery**: Enable applications to discover available cross-chain assets
- **Integration Standardization**: Provide consistent IBC token information across apps
- **Security Validation**: Validate IBC denomination checksums and formats
- **Multi-Chain Operations**: Support complex multi-hop IBC transfers
- **Protocol Compliance**: Ensure adherence to IBC specification standards
- **Ecosystem Interoperability**: Enable seamless cross-chain token operations

The ADO provides comprehensive IBC denomination management with SHA-256 hash validation, trace path parsing, and secure denomination storage for reliable cross-chain asset operations.

## Key Features

### **IBC Denomination Management**
- **Denomination storage**: Store complete IBC denomination information
- **Format validation**: Ensure proper IBC denomination format compliance
- **Checksum verification**: Validate SHA-256 hash integrity for security
- **Trace path management**: Store and validate IBC trace paths

### **Cross-Chain Asset Registry**
- **Asset registration**: Register IBC tokens from multiple chains
- **Metadata storage**: Store base denomination and path information
- **Bulk operations**: Support batch registration of multiple tokens
- **Duplicate prevention**: Prevent registration of duplicate denominations

### **Validation Engine**
- **Format checking**: Validate "ibc/" prefix and 64-character hash format
- **Hash verification**: Verify SHA-256 checksums against trace paths
- **Path validation**: Validate IBC trace path format and structure
- **Hop parsing**: Convert between trace paths and hop representations

### **Access Control**
- **Service restrictions**: Limit denomination storage to authorized services
- **Permission management**: Control who can register new denominations
- **Public queries**: Allow public access to denomination lookups
- **Registry integrity**: Prevent unauthorized modifications

## IBC Denomination Format

### **Standard IBC Denom Structure**
```
ibc/{SHA256_HASH}
```

Where:
- **ibc/**: Required prefix for all IBC denominations
- **SHA256_HASH**: 64-character uppercase hexadecimal SHA-256 hash

### **Hash Generation**
The SHA-256 hash is generated from the concatenation of the trace path and base denomination:
```
SHA256(trace_path + "/" + base_denom)
```

**Example:**
- Trace Path: `transfer/channel-12/transfer/channel-255`
- Base Denom: `inj`
- Input: `transfer/channel-12/transfer/channel-255/inj`
- Result: `ibc/EAB02686416E4B155CFEE9C247171E1C4196B218C6A254F765B0958B3AF59D09`

## InstantiateMsg

```rust
pub struct InstantiateMsg {
    pub kernel_address: Addr,
    pub owner: Option<String>,
    pub service_address: AndrAddr,
}
```

```json
{
    "kernel_address": "andr1kernel_address...",
    "owner": "andr1owner_address...",
    "service_address": "andr1service_address..."
}
```

**Parameters**:
- **kernel_address**: Address of the Andromeda Kernel contract
- **owner**: Optional owner address for contract administration
- **service_address**: Authorized address allowed to store denomination info
  - Only this address can execute StoreDenomInfo operations
  - Typically set to an IBC service or relayer address

## ExecuteMsg

### StoreDenomInfo
Stores IBC denomination information (restricted to service address).

_**Note:** Only the authorized service address can store denomination information._

```rust
StoreDenomInfo {
    ibc_denom_info: Vec<IBCDenomInfo>,
}

pub struct IBCDenomInfo {
    pub denom: String,
    pub denom_info: DenomInfo,
}

pub struct DenomInfo {
    pub path: String,
    pub base_denom: String,
}
```

```json
{
    "store_denom_info": {
        "ibc_denom_info": [
            {
                "denom": "ibc/EAB02686416E4B155CFEE9C247171E1C4196B218C6A254F765B0958B3AF59D09",
                "denom_info": {
                    "path": "transfer/channel-12/transfer/channel-255",
                    "base_denom": "inj"
                }
            },
            {
                "denom": "ibc/C4CFF46FD6DE35CA4CF4CE031E643C8FDC9BA4B99AE598E9B0ED98FE3A2319F9",
                "denom_info": {
                    "path": "transfer/channel-0",
                    "base_denom": "uatom"
                }
            }
        ]
    }
}
```

**Parameters**:
- **ibc_denom_info**: Array of IBC denomination information objects
  - **denom**: Complete IBC denomination string (must start with "ibc/")
  - **denom_info**: Trace path and base denomination information
    - **path**: IBC trace path showing transfer route
    - **base_denom**: Original denomination on the source chain

**Validation Rules**:
- Denomination must start with "ibc/"
- Must have exactly 64 characters after "ibc/"
- SHA-256 hash must match path/base_denom combination
- No duplicate denominations allowed in batch
- Only authorized service address can execute

## QueryMsg

### DenomInfo
Returns denomination information for a specific IBC denom.

```rust
pub enum QueryMsg {
    #[returns(DenomInfoResponse)]
    DenomInfo { denom: String },
}
```

```json
{
    "denom_info": {
        "denom": "ibc/EAB02686416E4B155CFEE9C247171E1C4196B218C6A254F765B0958B3AF59D09"
    }
}
```

**Response:**
```json
{
    "denom_info": {
        "path": "transfer/channel-12/transfer/channel-255",
        "base_denom": "inj"
    }
}
```

### AllDenomInfo
Returns paginated list of all stored denomination information.

```rust
pub enum QueryMsg {
    #[returns(AllDenomInfoResponse)]
    AllDenomInfo {
        limit: Option<u64>,
        start_after: Option<u64>,
    },
}
```

```json
{
    "all_denom_info": {
        "limit": 50,
        "start_after": 10
    }
}
```

**Response:**
```json
{
    "denom_info": [
        {
            "path": "transfer/channel-12/transfer/channel-255",
            "base_denom": "inj"
        },
        {
            "path": "transfer/channel-0",
            "base_denom": "uatom"
        },
        {
            "path": "transfer/channel-5/transfer/channel-1",
            "base_denom": "uosmo"
        }
    ]
}
```

**Parameters**:
- **limit**: Maximum number of results to return (default: 100)
- **start_after**: Starting point for pagination (optional)

## IBC Trace Path Format

### **Path Structure**
IBC trace paths follow the format:
```
port/channel/port/channel/...
```

**Example Paths:**
- Single hop: `transfer/channel-0`
- Multi-hop: `transfer/channel-12/transfer/channel-255`
- Complex route: `transfer/channel-0/transfer/channel-5/transfer/channel-10`

### **Path Components**
- **Port**: IBC port identifier (typically "transfer")
- **Channel**: IBC channel identifier (e.g., "channel-0", "channel-255")
- **Hops**: Each port/channel pair represents one IBC hop

### **Path Validation**
- Must have even number of segments (port/channel pairs)
- Port and channel IDs cannot be empty
- Path segments separated by forward slashes

## Token Examples

### Cosmos Hub ATOM
```json
{
    "denom": "ibc/C4CFF46FD6DE35CA4CF4CE031E643C8FDC9BA4B99AE598E9B0ED98FE3A2319F9",
    "denom_info": {
        "path": "transfer/channel-0",
        "base_denom": "uatom"
    }
}
```

### Osmosis OSMO (Multi-hop)
```json
{
    "denom": "ibc/14F9BC3E44B8A9C1BE1FB08EA71A4ACE4DD93FB5E8C8F4EEF9DCE3F8AAC9C38C",
    "denom_info": {
        "path": "transfer/channel-0/transfer/channel-141",
        "base_denom": "uosmo"
    }
}
```

### Injective INJ
```json
{
    "denom": "ibc/EAB02686416E4B155CFEE9C247171E1C4196B218C6A254F765B0958B3AF59D09",
    "denom_info": {
        "path": "transfer/channel-12/transfer/channel-255",
        "base_denom": "inj"
    }
}
```

### Terra Luna (Complex Route)
```json
{
    "denom": "ibc/785AFEC6B3741100D15129DA2D6A647CECB2F83E71F40F2C013AA16A8CE1B9AC5",
    "denom_info": {
        "path": "transfer/channel-1/transfer/channel-7/transfer/channel-0",
        "base_denom": "uluna"
    }
}
```

## Usage Examples

### Cross-Chain DEX Integration
```json
{
    "ibc_denom_info": [
        {
            "denom": "ibc/C4CFF46FD6DE35CA4CF4CE031E643C8FDC9BA4B99AE598E9B0ED98FE3A2319F9",
            "denom_info": {
                "path": "transfer/channel-0",
                "base_denom": "uatom"
            }
        },
        {
            "denom": "ibc/14F9BC3E44B8A9C1BE1FB08EA71A4ACE4DD93FB5E8C8F4EEF9DCE3F8AAC9C38C",
            "denom_info": {
                "path": "transfer/channel-0/transfer/channel-141",
                "base_denom": "uosmo"
            }
        }
    ]
}
```

### Portfolio Tracking
```json
{
    "ibc_denom_info": [
        {
            "denom": "ibc/EAB02686416E4B155CFEE9C247171E1C4196B218C6A254F765B0958B3AF59D09",
            "denom_info": {
                "path": "transfer/channel-12/transfer/channel-255",
                "base_denom": "inj"
            }
        },
        {
            "denom": "ibc/785AFEC6B3741100D15129DA2D6A647CECB2F83E71F40F2C013AA16A8CE1B9AC5",
            "denom_info": {
                "path": "transfer/channel-1/transfer/channel-7",
                "base_denom": "uluna"
            }
        }
    ]
}
```

### Asset Discovery Service
```json
{
    "ibc_denom_info": [
        {
            "denom": "ibc/9712DBB13B9631EDFA9BF61B55F1B2D290B2ADB67E3A4EB3A875F3B6081B3B84",
            "denom_info": {
                "path": "transfer/channel-8",
                "base_denom": "ujuno"
            }
        },
        {
            "denom": "ibc/4ABBEF4C8926DDDB320AE5188CFD63267ABBCEFC0583E4AE05D6E5AA2401DDAB",
            "denom_info": {
                "path": "transfer/channel-3/transfer/channel-1",
                "base_denom": "ustars"
            }
        }
    ]
}
```

### Stablecoin Registry
```json
{
    "ibc_denom_info": [
        {
            "denom": "ibc/8E27BA2D5493AF5636760E354E46004562C46AB7EC0CC4C1CA14E9E20E2545B5",
            "denom_info": {
                "path": "transfer/channel-1",
                "base_denom": "uusdc"
            }
        },
        {
            "denom": "ibc/F91EA2C0A23697A1048E08C2F787E3A58AC6F706A1CD2257A504925158CFC0F3",
            "denom_info": {
                "path": "transfer/channel-2",
                "base_denom": "uusdt"
            }
        }
    ]
}
```

## Integration Patterns

### With App Contract
The IBC Registry can be integrated into App contracts for asset management:

```json
{
    "components": [
        {
            "name": "ibc_token_registry",
            "ado_type": "ibc-registry",
            "component_type": {
                "new": {
                    "kernel_address": "andr1kernel...",
                    "service_address": "andr1ibc_service..."
                }
            }
        }
    ]
}
```

### Cross-Chain Asset Management
For multi-chain asset operations:

1. **Register IBC tokens** through authorized service
2. **Query denomination info** for asset identification
3. **Validate token formats** before operations
4. **Track asset origins** through trace paths

### DEX Integration
For decentralized exchange operations:

1. **Maintain token registry** of supported IBC assets
2. **Validate trading pairs** using denomination lookups
3. **Display asset metadata** from registry information
4. **Ensure asset authenticity** through hash validation

### Portfolio Management
For multi-chain portfolio tracking:

1. **Register user holdings** across multiple chains
2. **Aggregate asset values** using denomination lookups
3. **Track asset movements** through IBC transfers
4. **Maintain asset metadata** for display purposes

## Security Features

### **Service Authorization**
- **Restricted registration**: Only authorized service can store denominations
- **Permission management**: Service address set during instantiation
- **Access control**: Prevent unauthorized denomination storage
- **Registry integrity**: Maintain data consistency through controlled access

### **Denomination Validation**
- **Format verification**: Ensure proper IBC denomination format
- **Checksum validation**: Verify SHA-256 hash integrity
- **Duplicate prevention**: Prevent registration of duplicate denominations
- **Input sanitization**: Validate all input parameters

### **Hash Integrity**
- **SHA-256 verification**: Validate hash against trace path and base denom
- **Case normalization**: Handle case-insensitive comparisons
- **Deterministic hashing**: Ensure consistent hash generation
- **Tamper detection**: Detect any modification attempts

### **Data Consistency**
- **Atomic operations**: Ensure batch operations are atomic
- **State validation**: Maintain consistent internal state
- **Error handling**: Graceful handling of invalid inputs
- **Storage integrity**: Prevent data corruption

## Common Workflows

### 1. **Register IBC Tokens**
```json
{
    "store_denom_info": {
        "ibc_denom_info": [
            {
                "denom": "ibc/C4CFF46FD6DE35CA4CF4CE031E643C8FDC9BA4B99AE598E9B0ED98FE3A2319F9",
                "denom_info": {
                    "path": "transfer/channel-0",
                    "base_denom": "uatom"
                }
            }
        ]
    }
}
```

### 2. **Query Token Information**
```json
{
    "denom_info": {
        "denom": "ibc/C4CFF46FD6DE35CA4CF4CE031E643C8FDC9BA4B99AE598E9B0ED98FE3A2319F9"
    }
}
```

### 3. **Browse All Tokens**
```json
{
    "all_denom_info": {
        "limit": 100
    }
}
```

### 4. **Validate Token Format**
Before registration, verify that:
- Denom starts with "ibc/"
- Has exactly 64 characters after "ibc/"
- SHA-256 hash matches path/base_denom

## Important Notes

- **Service authorization**: Only designated service address can store denomination info
- **Format validation**: All IBC denoms must follow proper format and validation rules
- **Hash verification**: SHA-256 checksums are validated against trace paths
- **Duplicate prevention**: Cannot register the same denomination twice
- **Case sensitivity**: Denomination comparisons are case-insensitive
- **Batch operations**: Multiple denominations can be registered in one transaction
- **Public queries**: Anyone can query stored denomination information
- **Immutable storage**: Stored denomination info cannot be modified, only added

## Hash Calculation Example

**Input Data:**
- Path: `transfer/channel-12/transfer/channel-255`
- Base Denom: `inj`
- Combined: `transfer/channel-12/transfer/channel-255/inj`

**Hash Generation:**
```rust
let input = format!("{}/{}", path, base_denom);
let hash = Sha256::digest(input.as_bytes());
let hash_str = format!("{:X}", hash);
let ibc_denom = format!("ibc/{}", hash_str.to_uppercase());
```

**Result:**
`ibc/EAB02686416E4B155CFEE9C247171E1C4196B218C6A254F765B0958B3AF59D09`

The IBC Registry ADO provides essential infrastructure for cross-chain asset management, enabling secure and reliable tracking of IBC tokens across the Cosmos ecosystem with comprehensive validation and lookup capabilities.