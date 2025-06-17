# Andromeda Address List ADO

## Introduction

The Andromeda Address List ADO is a specialized permission management module that provides centralized access control for the Andromeda ecosystem. This contract enables ADO owners to create reusable permission lists that can be referenced by multiple ADOs, streamlining access control management across complex applications. Rather than setting permissions individually on each ADO, users can create shared address lists that define who can perform specific actions across their entire ecosystem.

<b>Ado_type:</b> address-list

## Why Address List ADO

The Address List ADO serves as a critical infrastructure component for applications requiring:

- **Centralized Access Control**: Manage permissions for multiple ADOs from a single location
- **Role-Based Security**: Create reusable permission groups for different user roles
- **Scalable Permission Management**: Efficiently manage large numbers of users and permissions
- **Consistent Security Policies**: Apply uniform access control across entire ecosystems
- **Administrative Efficiency**: Reduce overhead of managing individual ADO permissions
- **Dynamic Permission Updates**: Modify access rights across multiple contracts simultaneously
- **Compliance Management**: Maintain audit trails and standardized access controls
- **Organizational Security**: Implement enterprise-grade permission hierarchies
- **Integration Flexibility**: Reference permission lists from any compatible ADO
- **Security Governance**: Enable governance-based permission management

The ADO supports time-based permissions, whitelisting, and blacklisting models for comprehensive access control.

## Key Features

### **Permission Management**
- **Centralized storage**: Single source of truth for permission lists
- **Actor-based permissions**: Assign specific permissions to individual addresses
- **Bulk operations**: Add or remove multiple actors in single transactions
- **Permission validation**: Automatic validation of permission types and parameters
- **Address resolution**: Support for both direct addresses and AndrAddr references

### **Permission Types**
- **Blacklisted permissions**: Explicitly deny access to specific addresses
- **Whitelisted permissions**: Grant access only to approved addresses
- **Time-based controls**: Configure permissions with expiration times and usage limits
- **Inclusive/Exclusive modes**: Support both allow-list and deny-list models
- **Permission inheritance**: Leverage Andromeda's hierarchical permission system

### **Query Interface**
- **Inclusion checking**: Verify if addresses are included in permission lists
- **Permission lookup**: Retrieve specific permission details for addresses
- **Efficient validation**: Fast permission checking for real-time access control
- **Integration support**: Easy integration with other ADOs for permission validation

### **Administrative Controls**
- **Owner-only management**: Restrict permission modifications to contract owner
- **Batch processing**: Handle multiple permission changes efficiently
- **Error handling**: Comprehensive validation and error reporting
- **State consistency**: Maintain consistent permission state across operations

## Permission System

### **Local Permission Types**
The Address List ADO supports several types of local permissions from the Andromeda framework:

1. **Blacklisted**: Explicitly denies access to specified addresses
2. **Whitelisted**: Grants access with optional time and usage constraints
3. **Limited**: Not supported in Address List (too complex for centralized management)

### **Time-Based Controls**
- **Expiration times**: Set permission expiration dates
- **Usage limits**: Configure maximum number of uses
- **Validation timing**: Automatic validation of time-based constraints
- **Dynamic updates**: Modify time constraints without recreating permissions

### **Integration Model**
Address lists are referenced by other ADOs through their contract address, enabling:
- **Shared permissions**: Multiple ADOs can reference the same address list
- **Consistent policies**: Uniform access control across applications
- **Efficient updates**: Single update affects all referencing ADOs
- **Modular security**: Separates permission logic from business logic

## InstantiateMsg

```rust
pub struct InstantiateMsg {
    pub actor_permission: Option<ActorPermission>,
}

pub struct ActorPermission {
    pub actors: Vec<AndrAddr>,
    pub permission: LocalPermission,
}
```

```json
{
    "actor_permission": {
        "actors": [
            "andr1admin_address...",
            "andr1manager_address...",
            "andr1operator_address..."
        ],
        "permission": {
            "whitelisted": {
                "expiration": {
                    "at_time": "1672704000000"
                },
                "uses": 100
            }
        }
    }
}
```

**Parameters**:
- **actor_permission**: Optional initial permission configuration
  - **actors**: List of addresses to assign permissions
  - **permission**: Permission type and constraints to apply

**Permission Types**:
```json
// Blacklist permission (deny access)
{
    "permission": {
        "blacklisted": {
            "expiration": null
        }
    }
}

// Whitelist permission (allow access)
{
    "permission": {
        "whitelisted": {
            "expiration": {
                "at_time": "1672704000000"
            },
            "uses": 50
        }
    }
}

// Simple whitelist (no constraints)
{
    "permission": {
        "whitelisted": {
            "expiration": null,
            "uses": null
        }
    }
}
```

**Validation**:
- Limited permissions are not supported (too complex for address lists)
- Actor list cannot be empty if provided
- Time-based permissions are validated for proper timing
- Addresses are validated and resolved through AndrAddr system

## ExecuteMsg

### PermissionActors
Adds or updates permissions for multiple actors.

```rust
PermissionActors {
    actors: Vec<AndrAddr>,
    permission: LocalPermission,
}
```

```json
{
    "permission_actors": {
        "actors": [
            "andr1new_admin...",
            "andr1new_manager...",
            "andr1new_operator..."
        ],
        "permission": {
            "whitelisted": {
                "expiration": {
                    "at_time": "1675296000000"
                },
                "uses": 200
            }
        }
    }
}
```

**Functionality**:
- Adds new actors to the permission list
- Updates existing actor permissions with new values
- Supports bulk operations for efficiency
- Validates all permissions before applying changes

**Parameters**:
- **actors**: List of addresses to add or update
- **permission**: Permission configuration to apply

**Requirements**:
- Only contract owner can execute this message
- Actor list cannot be empty
- Limited permissions are not supported
- Time-based permissions must be valid

### RemovePermissions
Removes permissions for specified actors.

```rust
RemovePermissions { actors: Vec<AndrAddr> }
```

```json
{
    "remove_permissions": {
        "actors": [
            "andr1former_admin...",
            "andr1revoked_user..."
        ]
    }
}
```

**Functionality**:
- Completely removes actors from the permission list
- Supports bulk removal operations
- Validates that actors exist before removal
- Provides clear error messages for missing actors

**Parameters**:
- **actors**: List of addresses to remove from permissions

**Requirements**:
- Only contract owner can execute this message
- Actor list cannot be empty
- All specified actors must exist in the permission list

## QueryMsg

### IncludesActor
Checks if an actor is included in the permission list.

```rust
#[returns(IncludesActorResponse)]
IncludesActor { actor: Addr }
```

```json
{
    "includes_actor": {
        "actor": "andr1user_to_check..."
    }
}
```

**Response:**
```json
{
    "included": true
}
```

**Usage**: This query is primarily used by other ADOs to validate permissions before executing operations. It provides a simple boolean response indicating whether the address is present in the permission list.

### ActorPermission
Retrieves the specific permission details for an actor.

```rust
#[returns(ActorPermissionResponse)]
ActorPermission { actor: Addr }
```

```json
{
    "actor_permission": {
        "actor": "andr1user_to_query..."
    }
}
```

**Response:**
```json
{
    "permission": {
        "whitelisted": {
            "expiration": {
                "at_time": "1672704000000"
            },
            "uses": 45
        }
    }
}
```

**Usage**: This query returns detailed permission information, including expiration times and remaining uses. It's useful for administrative interfaces and detailed permission auditing.

**Error Handling**: Returns `ActorNotFound` error if the actor is not in the permission list.

## Usage Examples

### Administrative Team Setup
```json
{
    "actor_permission": {
        "actors": [
            "andr1ceo_address...",
            "andr1cto_address...",
            "andr1security_officer..."
        ],
        "permission": {
            "whitelisted": {
                "expiration": null,
                "uses": null
            }
        }
    }
}
```
_Create permanent whitelist for executive team with unlimited access._

### Time-Limited Access
```json
{
    "actor_permission": {
        "actors": [
            "andr1contractor1...",
            "andr1contractor2..."
        ],
        "permission": {
            "whitelisted": {
                "expiration": {
                    "at_time": "1677628800000"
                },
                "uses": 50
            }
        }
    }
}
```
_Grant temporary access to contractors with usage limits._

### Security Blacklist
```json
{
    "actor_permission": {
        "actors": [
            "andr1blocked_user1...",
            "andr1suspicious_address..."
        ],
        "permission": {
            "blacklisted": {
                "expiration": null
            }
        }
    }
}
```
_Permanently blacklist problematic addresses._

### Beta Tester Group
```json
{
    "actor_permission": {
        "actors": [
            "andr1beta_tester1...",
            "andr1beta_tester2...",
            "andr1beta_tester3..."
        ],
        "permission": {
            "whitelisted": {
                "expiration": {
                    "at_time": "1674240000000"
                },
                "uses": 100
            }
        }
    }
}
```
_Create beta testing group with limited access period._

## Operational Examples

### Add New Team Members
```json
{
    "permission_actors": {
        "actors": [
            "andr1new_developer...",
            "andr1new_designer..."
        ],
        "permission": {
            "whitelisted": {
                "expiration": {
                    "at_time": "1680307200000"
                },
                "uses": 500
            }
        }
    }
}
```

### Remove Former Employees
```json
{
    "remove_permissions": {
        "actors": [
            "andr1former_employee1...",
            "andr1former_employee2..."
        ]
    }
}
```

### Update Contractor Permissions
```json
{
    "permission_actors": {
        "actors": [
            "andr1contractor_address..."
        ],
        "permission": {
            "whitelisted": {
                "expiration": {
                    "at_time": "1685491200000"
                },
                "uses": 200
            }
        }
    }
}
```

### Check User Access
```json
{
    "includes_actor": {
        "actor": "andr1potential_user..."
    }
}
```

### Audit User Permissions
```json
{
    "actor_permission": {
        "actor": "andr1team_member..."
    }
}
```

## Integration Patterns

### With App Contract
Address lists can be integrated into App contracts for centralized permission management:

```json
{
    "components": [
        {
            "name": "admin_permissions",
            "ado_type": "address-list",
            "component_type": {
                "new": {
                    "actor_permission": {
                        "actors": [
                            "andr1admin1...",
                            "andr1admin2..."
                        ],
                        "permission": {
                            "whitelisted": {
                                "expiration": null,
                                "uses": null
                            }
                        }
                    }
                }
            }
        },
        {
            "name": "main_contract",
            "ado_type": "marketplace",
            "component_type": {
                "new": {
                    "authorized_addresses": ["./admin_permissions"]
                }
            }
        }
    ]
}
```

### Enterprise Permission Management
For large organizations with complex access requirements:

1. **Create role-based address lists** for different organizational levels
2. **Reference lists from multiple ADOs** to maintain consistency
3. **Implement permission hierarchies** through multiple address lists
4. **Use time-based permissions** for temporary access and contractors

### Multi-ADO Applications
For applications spanning multiple ADOs:

1. **Deploy shared address lists** for common permission sets
2. **Reference lists across ADOs** for consistent access control
3. **Implement modular permissions** for different application components
4. **Maintain centralized permission management** while preserving ADO autonomy

### DeFi Protocol Security
For DeFi applications requiring strict access controls:

1. **Create admin address lists** for protocol governance
2. **Implement emergency address lists** for crisis management
3. **Use blacklists for security** to block malicious addresses
4. **Configure time-limited permissions** for operational security

## Security Features

### **Access Control**
- **Owner-only modifications**: Only contract owner can modify permissions
- **Comprehensive validation**: All inputs are thoroughly validated
- **Address verification**: Addresses are verified through AndrAddr system
- **Atomic operations**: Permission changes are applied atomically

### **Permission Validation**
- **Type checking**: Only supported permission types are accepted
- **Time validation**: Expiration times and usage limits are validated
- **Consistency enforcement**: Permission state is kept consistent
- **Error handling**: Clear error messages for invalid operations

### **Integration Security**
- **Immutable references**: Address list contracts cannot be changed once referenced
- **Query-only access**: Other contracts can only query, not modify permissions
- **State isolation**: Address list state is isolated from referencing contracts
- **Audit capabilities**: Complete audit trail of permission changes

### **Operational Security**
- **Bulk operation safety**: Multiple operations are handled safely
- **Empty list protection**: Prevents creation of empty permission sets
- **Existence validation**: Ensures actors exist before removal
- **Permission consistency**: Maintains consistent permission semantics

## Advanced Features

### **Time-Based Permissions**
- **Expiration management**: Automatic handling of time-based permission expiry
- **Usage tracking**: Built-in tracking of permission usage counts
- **Flexible timing**: Support for various time-based restriction models
- **Validation integration**: Time constraints are validated during permission checks

### **Batch Operations**
- **Efficient bulk processing**: Handle multiple permission changes efficiently
- **Atomic batch updates**: All operations in a batch succeed or fail together
- **Error reporting**: Clear reporting of batch operation results
- **Performance optimization**: Optimized for handling large permission sets

### **Query Optimization**
- **Fast inclusion checking**: Optimized queries for permission validation
- **Detailed permission lookup**: Comprehensive permission detail retrieval
- **Integration-friendly**: Designed for easy integration with other ADOs
- **Caching support**: Query results are suitable for caching by integrating contracts

### **Address Resolution**
- **AndrAddr support**: Full support for Andromeda address resolution
- **Direct address handling**: Also supports direct Cosmos addresses
- **Validation pipeline**: Comprehensive address validation before storage
- **Error handling**: Clear error messages for invalid addresses

## Important Notes

- **Limited permissions not supported**: Address lists don't support Limited permission type due to complexity
- **Owner-only modifications**: Only contract owner can modify permission lists
- **Immutable references**: Once referenced by other ADOs, address lists should be considered stable
- **Query-based integration**: Other contracts interact with address lists through queries only
- **Atomic operations**: All permission changes are applied atomically
- **Address validation**: All addresses are validated and resolved before storage
- **Permission inheritance**: Leverages Andromeda's broader permission system
- **State consistency**: Permission state is maintained consistently across all operations

## Common Workflow

### 1. **Deploy Address List**
```json
{
    "actor_permission": {
        "actors": [
            "andr1initial_admin...",
            "andr1initial_manager..."
        ],
        "permission": {
            "whitelisted": {
                "expiration": null,
                "uses": null
            }
        }
    }
}
```

### 2. **Add Team Members**
```json
{
    "permission_actors": {
        "actors": [
            "andr1new_team_member1...",
            "andr1new_team_member2..."
        ],
        "permission": {
            "whitelisted": {
                "expiration": {
                    "at_time": "1680307200000"
                },
                "uses": 1000
            }
        }
    }
}
```

### 3. **Reference from Other ADOs**
```json
{
    "instantiate": {
        "ado_type": "marketplace",
        "msg": {
            "authorized_addresses": ["andr1address_list_contract..."]
        }
    }
}
```

### 4. **Validate Permissions (by other ADOs)**
```json
{
    "includes_actor": {
        "actor": "andr1user_attempting_action..."
    }
}
```

### 5. **Audit User Permissions**
```json
{
    "actor_permission": {
        "actor": "andr1user_to_audit..."
    }
}
```

### 6. **Remove Former Members**
```json
{
    "remove_permissions": {
        "actors": [
            "andr1former_employee...",
            "andr1expired_contractor..."
        ]
    }
}
```

### 7. **Update Permissions**
```json
{
    "permission_actors": {
        "actors": [
            "andr1promoted_user..."
        ],
        "permission": {
            "whitelisted": {
                "expiration": null,
                "uses": null
            }
        }
    }
}
```

The Address List ADO provides essential infrastructure for scalable permission management in the Andromeda ecosystem, enabling centralized access control that integrates seamlessly with all other ADOs for enterprise-grade security and administrative efficiency.