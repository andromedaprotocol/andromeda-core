# Andromeda IBC Registry ADO

## Introduction

The Andromeda IBC Registry ADO is a critical operating system component that manages and validates Inter-Blockchain Communication (IBC) denomination information within the Andromeda ecosystem. This contract serves as a centralized registry for tracking IBC token denominations, their trace paths, and base denominations across different blockchain networks. The registry provides essential infrastructure for cross-chain asset verification, denomination validation, and IBC denom hash computation, making it fundamental for multi-chain operations, cross-chain asset transfers, and blockchain interoperability.

<b>Ado_type:</b> ibc-registry

## Why IBC Registry ADO

The IBC Registry ADO serves as essential infrastructure for applications requiring:

- **Cross-Chain Asset Tracking**: Track and validate IBC assets across multiple blockchain networks
- **Denomination Verification**: Verify IBC denomination authenticity and trace paths
- **Multi-Chain Operations**: Support applications operating across multiple Cosmos chains
- **Asset Interoperability**: Enable seamless asset transfers between different blockchains
- **IBC Compliance**: Ensure compliance with IBC protocol standards for denominations
- **Chain Registry Services**: Provide registry services for blockchain denomination metadata
- **Cross-Chain DeFi**: Support DeFi applications with multi-chain asset support
- **Asset Management**: Manage complex IBC asset portfolios across chains
- **Protocol Integration**: Integrate with IBC-enabled protocols and applications
- **Audit and Verification**: Provide audit trails for cross-chain asset movements

The ADO provides centralized denomination management with comprehensive validation and query capabilities for reliable cross-chain operations.

## Key Features

### **IBC Denomination Management**
- **Denom registration**: Store and manage IBC denomination information
- **Trace path tracking**: Track IBC trace paths for cross-chain assets
- **Base denomination mapping**: Map IBC denoms to their original base denominations
- **Hash validation**: Validate IBC denomination hashes against computed values
- **Duplicate prevention**: Prevent duplicate denomination registrations

### **SHA-256 Hash Computation**
- **Automatic hash calculation**: Compute IBC denom hashes from path and base denom
- **Hash verification**: Verify provided hashes match computed values
- **Standard compliance**: Follow IBC protocol standards for hash computation
- **Case handling**: Proper case handling for hash comparisons
- **Format validation**: Ensure proper IBC denomination format (ibc/<64-char-hash>)

### **Service Authorization**
- **Authorized registration**: Only authorized services can register denominations
- **Permission management**: Integration with Andromeda permission system
- **Service control**: Control which services can modify registry data
- **Access validation**: Verify permissions before allowing data modifications
- **Administrative oversight**: Owner control over service authorizations

### **Comprehensive Querying**
- **Individual queries**: Query specific denomination information
- **Bulk queries**: Retrieve multiple denomination records with pagination
- **Pagination support**: Handle large datasets with configurable limits
- **Start-after filtering**: Support for pagination through large result sets
- **Registry browsing**: Browse entire denomination registry with controls

## IBC Denomination Structure

### **DenomInfo**
```rust
pub struct DenomInfo {
    pub path: String,        // IBC trace path (port/channel/port/channel/...)
    pub base_denom: String,  // Original denomination on source chain
}
```

### **IBC Denom Hash Calculation**
The registry computes IBC denomination hashes using the following process:
1. **Concatenate** path and base_denom with "/" separator
2. **Hash** the concatenated string using SHA-256
3. **Format** as "ibc/<UPPERCASE_HEX_HASH>"

**Example**:
- Path: "transfer/channel-0/transfer/channel-1"
- Base denom: "uatom"
- Input: "transfer/channel-0/transfer/channel-1/uatom"
- Output: "ibc/<SHA256_HASH_IN_UPPERCASE_HEX>"

### **Trace Path Format**
IBC trace paths follow the format: `port_id/channel_id/port_id/channel_id/...`
- Must have even number of segments (port/channel pairs)
- Each segment must be non-empty
- Represents the path taken by an asset across chains

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
- **kernel_address**: Address of the Andromeda kernel for system integration
- **owner**: Optional contract owner address for administrative functions
- **service_address**: Address of the authorized service that can register denominations

**Authorization Setup**:
- Service address is automatically whitelisted for denomination registration
- Only the specified service can store denomination information
- Owner maintains administrative control over the contract

## ExecuteMsg

### StoreDenomInfo
Stores IBC denomination information (service-only).

```rust
StoreDenomInfo {
    ibc_denom_info: Vec<IBCDenomInfo>,
}

pub struct IBCDenomInfo {
    pub denom: String,      // IBC denomination (ibc/<64-char-hash>)
    pub denom_info: DenomInfo,
}
```

```json
{
    "store_denom_info": {
        "ibc_denom_info": [
            {
                "denom": "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
                "denom_info": {
                    "path": "transfer/channel-0",
                    "base_denom": "uatom"
                }
            },
            {
                "denom": "ibc/14F9BC3E44B8A9C1BE1FB08980FAB87034C9905EF17CF2F5008FC085218811CC",
                "denom_info": {
                    "path": "transfer/channel-1/transfer/channel-5",
                    "base_denom": "uosmo"
                }
            }
        ]
    }
}
```

**Authorization**: Only authorized service address can execute
**Validation Process**:
1. Verify sender is authorized service
2. Check denomination format (must start with "ibc/")
3. Validate hash length (exactly 64 characters after "ibc/")
4. Compute expected hash from path and base_denom
5. Verify provided hash matches computed hash
6. Check for duplicate denominations in submission
7. Store validated denomination information

**Requirements**:
- Must provide at least one denomination
- All denominations must be unique within the submission
- Each denomination must pass hash validation
- Trace paths must have valid format (even number of segments)

## QueryMsg

### DenomInfo
Returns information for a specific IBC denomination.

```rust
#[returns(DenomInfoResponse)]
DenomInfo {
    denom: String,
}

pub struct DenomInfoResponse {
    pub denom_info: DenomInfo,
}
```

```json
{
    "denom_info": {
        "denom": "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2"
    }
}
```

**Response:**
```json
{
    "denom_info": {
        "path": "transfer/channel-0",
        "base_denom": "uatom"
    }
}
```

### AllDenomInfo
Returns all registered denomination information with pagination.

```rust
#[returns(AllDenomInfoResponse)]
AllDenomInfo {
    limit: Option<u64>,
    start_after: Option<u64>,
}

pub struct AllDenomInfoResponse {
    pub denom_info: Vec<DenomInfo>,
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
            "path": "transfer/channel-0",
            "base_denom": "uatom"
        },
        {
            "path": "transfer/channel-1/transfer/channel-5",
            "base_denom": "uosmo"
        }
    ]
}
```

**Pagination Parameters**:
- **limit**: Maximum number of records to return (default: 100)
- **start_after**: Start returning results after this record number
- Results are returned in ascending order by denomination

## Usage Examples

### Single Chain IBC Asset
```json
{
    "store_denom_info": {
        "ibc_denom_info": [
            {
                "denom": "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
                "denom_info": {
                    "path": "transfer/channel-0",
                    "base_denom": "uatom"
                }
            }
        ]
    }
}
```
_Registers ATOM transferred from Cosmos Hub through channel-0_

### Multi-Hop IBC Asset
```json
{
    "store_denom_info": {
        "ibc_denom_info": [
            {
                "denom": "ibc/A8C2D23A1E6F95DA4E48BA349667E322BD7A6C996D8A4AAE8BA72E190F3D1D8E",
                "denom_info": {
                    "path": "transfer/channel-0/transfer/channel-3/transfer/channel-1",
                    "base_denom": "ujuno"
                }
            }
        ]
    }
}
```
_Registers JUNO that traveled through multiple chains via different channels_

### Multiple Asset Registration
```json
{
    "store_denom_info": {
        "ibc_denom_info": [
            {
                "denom": "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
                "denom_info": {
                    "path": "transfer/channel-0",
                    "base_denom": "uatom"
                }
            },
            {
                "denom": "ibc/14F9BC3E44B8A9C1BE1FB08980FAB87034C9905EF17CF2F5008FC085218811CC",
                "denom_info": {
                    "path": "transfer/channel-1",
                    "base_denom": "uosmo"
                }
            },
            {
                "denom": "ibc/C4CFF46FD6DE35CA4CF4CE031E643C8FDC9BA4B99AE598E9B0ED98FE3A2319F9",
                "denom_info": {
                    "path": "transfer/channel-2",
                    "base_denom": "ujuno"
                }
            }
        ]
    }
}
```

## Operational Examples

### Query Specific Denomination
```json
{
    "denom_info": {
        "denom": "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2"
    }
}
```

### Query All Denominations (First Page)
```json
{
    "all_denom_info": {
        "limit": 20,
        "start_after": null
    }
}
```

### Query All Denominations (Pagination)
```json
{
    "all_denom_info": {
        "limit": 20,
        "start_after": 20
    }
}
```

### Validate IBC Hash Calculation
For path "transfer/channel-0" and base_denom "uatom":
1. **Input**: "transfer/channel-0/uatom"
2. **SHA-256**: Compute hash of input string
3. **Format**: "ibc/<UPPERCASE_HEX_HASH>"
4. **Verify**: Provided denom matches computed hash

## Integration Patterns

### With Cross-Chain Applications
IBC registry provides denomination validation for cross-chain apps:

```json
{
    "components": [
        {
            "name": "ibc_registry",
            "ado_type": "ibc-registry",
            "component_type": {
                "new": {
                    "kernel_address": "andr1kernel...",
                    "owner": "andr1owner...",
                    "service_address": "andr1relayer_service..."
                }
            }
        }
    ]
}
```

### Multi-Chain Asset Management
For tracking assets across multiple chains:

1. **Deploy IBC registry** with authorized relayer service
2. **Register asset denominations** as they're transferred across chains
3. **Validate asset authenticity** through hash verification
4. **Query asset origins** to understand transfer paths
5. **Maintain asset registry** for multi-chain portfolio management

### Cross-Chain DeFi Integration
For DeFi applications with multi-chain support:

1. **Integrate with registry** for asset verification
2. **Validate IBC denominations** before accepting in DeFi operations
3. **Track asset provenance** through trace path analysis
4. **Support complex routing** with multi-hop IBC paths
5. **Ensure asset authenticity** through cryptographic verification

### Relayer Service Integration
For IBC relayer services:

1. **Authorize relayer service** for denomination registration
2. **Register new denominations** as assets are transferred
3. **Maintain denomination database** for transfer validation
4. **Provide verification services** to connected applications
5. **Support auditing** through comprehensive query capabilities

### Asset Verification Pipeline
For asset verification workflows:

1. **Receive IBC asset information** from transfer events
2. **Validate denomination format** and hash computation
3. **Register verified denominations** in central registry
4. **Provide lookup services** for asset verification
5. **Maintain audit trails** for compliance and security

## Advanced Features

### **Cryptographic Verification**
- **SHA-256 hashing**: Standard cryptographic hash function for denomination IDs
- **Hash validation**: Verify computed hashes match provided denominations
- **Case handling**: Proper uppercase formatting for hash strings
- **Format enforcement**: Strict format validation for IBC denominations
- **Integrity checking**: Ensure denomination data integrity through hashing

### **Trace Path Management**
- **Path parsing**: Parse complex multi-hop IBC trace paths
- **Hop extraction**: Extract individual port/channel pairs from paths
- **Path validation**: Validate trace path format and structure
- **Route reconstruction**: Reconstruct transfer routes from trace paths
- **Multi-chain tracking**: Track assets across multiple blockchain hops

### **Service Authorization**
- **Permission integration**: Integration with Andromeda permission system
- **Service whitelisting**: Authorize specific services for denomination registration
- **Access control**: Control who can modify registry data
- **Administrative oversight**: Owner control over service permissions
- **Security enforcement**: Strict permission validation for modifications

### **Registry Management**
- **Duplicate prevention**: Prevent duplicate denomination registrations
- **Bulk operations**: Support bulk denomination registration
- **Data validation**: Comprehensive validation of all denomination data
- **Storage optimization**: Efficient storage and retrieval of denomination data
- **Query optimization**: Optimized querying with pagination support

## Security Features

### **Authorization Control**
- **Service restrictions**: Only authorized services can register denominations
- **Permission verification**: Strict verification of service permissions
- **Administrative control**: Owner maintains ultimate control over authorizations
- **Access validation**: Comprehensive access control for all operations
- **Unauthorized prevention**: Prevent unauthorized denomination registration

### **Data Integrity**
- **Hash verification**: Cryptographic verification of denomination hashes
- **Format validation**: Strict validation of denomination and path formats
- **Duplicate detection**: Prevent duplicate or conflicting registrations
- **Input sanitization**: Safe handling of all input data
- **State consistency**: Maintain consistent registry state across operations

### **Cryptographic Security**
- **SHA-256 hashing**: Industry-standard cryptographic hash function
- **Hash computation**: Secure hash computation for denomination verification
- **Case sensitivity**: Proper case handling for security-critical operations
- **Format enforcement**: Strict format requirements for security
- **Verification standards**: Compliance with IBC protocol security standards

### **Registry Protection**
- **Access control**: Protect registry from unauthorized modifications
- **Data validation**: Validate all data before storage
- **State protection**: Maintain registry integrity across all operations
- **Error handling**: Safe error handling without state corruption
- **Transaction safety**: Ensure atomic operations for registry updates

## Important Notes

- **Service authorization**: Only authorized services can register denominations
- **Hash validation**: All IBC denominations must pass hash verification
- **Format requirements**: Denominations must follow "ibc/<64-char-hash>" format
- **Trace path format**: Paths must have even number of segments (port/channel pairs)
- **Case sensitivity**: Hash comparisons are case-insensitive, output is uppercase
- **Duplicate prevention**: Cannot register duplicate denominations
- **Pagination support**: Large result sets support pagination with configurable limits
- **Administrative control**: Contract owner maintains administrative oversight

## Common Workflow

### 1. **Deploy IBC Registry**
```json
{
    "kernel_address": "andr1kernel...",
    "owner": "andr1owner...",
    "service_address": "andr1relayer_service..."
}
```

### 2. **Register IBC Denominations**
```json
{
    "store_denom_info": {
        "ibc_denom_info": [
            {
                "denom": "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
                "denom_info": {
                    "path": "transfer/channel-0",
                    "base_denom": "uatom"
                }
            }
        ]
    }
}
```

### 3. **Query Denomination Info**
```json
{
    "denom_info": {
        "denom": "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2"
    }
}
```

### 4. **Browse All Denominations**
```json
{
    "all_denom_info": {
        "limit": 50,
        "start_after": null
    }
}
```

### 5. **Verify Asset Authenticity**
For received asset with denom "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2":
1. Query registry for denomination info
2. Verify path and base_denom make sense
3. Optionally recompute hash for additional verification
4. Use asset with confidence in authenticity

### 6. **Register Multi-Hop Asset**
```json
{
    "store_denom_info": {
        "ibc_denom_info": [
            {
                "denom": "ibc/A8C2D23A1E6F95DA4E48BA349667E322BD7A6C996D8A4AAE8BA72E190F3D1D8E",
                "denom_info": {
                    "path": "transfer/channel-0/transfer/channel-3/transfer/channel-1",
                    "base_denom": "ujuno"
                }
            }
        ]
    }
}
```

The IBC Registry ADO provides essential cross-chain infrastructure for the Andromeda ecosystem, enabling secure denomination tracking, validation, and management across multiple blockchain networks with cryptographic integrity and comprehensive query capabilities.