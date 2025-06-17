# Andromeda String Storage ADO

## Introduction

The Andromeda String Storage ADO is a versatile text data storage contract that provides secure string storage with configurable access control and validation. It enables applications to store, retrieve, and manage string data with fine-grained permission controls, making it ideal for text storage, metadata management, configuration storage, user-generated content, and any application requiring persistent string data with ownership tracking.

<b>Ado_type:</b> string-storage

## Why String Storage ADO

The String Storage ADO serves as a fundamental building block for applications requiring:

- **Text Data Storage**: Store user-generated content, messages, or descriptions
- **Metadata Management**: Store NFT metadata, asset descriptions, or annotations
- **Configuration Storage**: Store application settings, preferences, or parameters as strings
- **Content Management**: Manage blog posts, comments, or documentation
- **Name Services**: Store domain names, usernames, or aliases
- **URL Storage**: Store links, references, or external resource addresses
- **Documentation**: Store help text, instructions, or user guides
- **Labeling Systems**: Store tags, categories, or classification labels
- **JSON Storage**: Store structured data as JSON strings
- **Template Storage**: Store message templates, email templates, or UI templates

The ADO supports three access control modes: **Private** (owner only), **Public** (anyone can set), and **Restricted** (configurable permissions), with automatic validation to prevent empty strings.

## InstantiateMsg

```rust
pub struct InstantiateMsg {
    pub restriction: StringStorageRestriction,
}

pub enum StringStorageRestriction {
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

- **restriction**: Controls who can modify the stored string
  - `"Private"`: Only contract owner can set the string value
  - `"Public"`: Anyone can set the string value
  - `"Restricted"`: Uses Andromeda's advanced permission system for access control

## ExecuteMsg

### SetValue
Sets the string value stored in the contract.

_**Note:** Permission requirements depend on the restriction setting configured during instantiation. Empty strings are not allowed._

```rust
SetValue {
    value: StringStorage,
}

pub enum StringStorage {
    String(String),
}
```

```json
{
    "set_value": {
        "value": {
            "string": "Hello, Andromeda! This is my stored text."
        }
    }
}
```

### DeleteValue
Removes the stored string value from the contract.

_**Note:** Permission requirements depend on the restriction setting._

```rust
DeleteValue {}
```

```json
{
    "delete_value": {}
}
```

### UpdateRestriction
Updates the access restriction for string storage operations.

_**Note:** Only contract owner can execute this operation._

```rust
UpdateRestriction {
    restriction: StringStorageRestriction,
}
```

```json
{
    "update_restriction": {
        "restriction": "Public"
    }
}
```

## QueryMsg

### GetValue
Returns the current string value stored in the contract.

```rust
pub enum QueryMsg {
    #[returns(GetValueResponse)]
    GetValue {},
}
```

```json
{
    "get_value": {}
}
```

**Response:**
```json
{
    "value": "Hello, Andromeda! This is my stored text."
}
```

### GetDataOwner
Returns the address that currently owns the string data (who set the value).

```rust
pub enum QueryMsg {
    #[returns(GetDataOwnerResponse)]
    GetDataOwner {},
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
    "owner": "andr1useraddress..."
}
```

## String Storage Types

### Plain Text
```json
{
    "set_value": {
        "value": {
            "string": "This is a simple text message."
        }
    }
}
```

### JSON Data
```json
{
    "set_value": {
        "value": {
            "string": "{\"name\": \"John Doe\", \"age\": 30, \"city\": \"New York\"}"
        }
    }
}
```

### Metadata
```json
{
    "set_value": {
        "value": {
            "string": "NFT Description: A unique digital artwork representing the essence of blockchain technology."
        }
    }
}
```

### Configuration
```json
{
    "set_value": {
        "value": {
            "string": "theme=dark;language=en;notifications=true"
        }
    }
}
```

## Access Control Modes

### Private Mode
- **Who can set value**: Only contract owner
- **Use case**: Personal notes, private configuration, owner-controlled content
- **Example**: User profile descriptions, private settings

```json
{
    "restriction": "Private"
}
```

### Public Mode
- **Who can set value**: Anyone
- **Use case**: Community content, collaborative editing, public messages
- **Example**: Guest books, public announcements, shared descriptions

```json
{
    "restriction": "Public"
}
```

### Restricted Mode
- **Who can set value**: Uses Andromeda's permission system
- **Use case**: Role-based content management, controlled access, team collaboration
- **Example**: Editorial systems, managed content, authorized updates

```json
{
    "restriction": "Restricted"
}
```

## Usage Examples

### User Profile Description
```json
{
    "set_value": {
        "value": {
            "string": "Blockchain enthusiast and DeFi developer. Building the future of decentralized finance."
        }
    }
}
```

### NFT Metadata
```json
{
    "set_value": {
        "value": {
            "string": "{\"name\": \"Cosmic Dragon #1337\", \"description\": \"A majestic dragon soaring through the cosmos\", \"attributes\": [{\"trait_type\": \"Rarity\", \"value\": \"Legendary\"}]}"
        }
    }
}
```

### Application Configuration
```json
{
    "set_value": {
        "value": {
            "string": "max_users=1000;timeout=30;debug_mode=false;api_endpoint=https://api.example.com"
        }
    }
}
```

### Welcome Message
```json
{
    "set_value": {
        "value": {
            "string": "Welcome to our decentralized application! This platform enables secure, trustless interactions."
        }
    }
}
```

### URL Storage
```json
{
    "set_value": {
        "value": {
            "string": "https://docs.andromedaprotocol.io/andromeda/andromeda-digital-objects/string-storage"
        }
    }
}
```

## Integration Patterns

### With App Contract
The String Storage ADO can be integrated into App contracts for text management:

```json
{
    "components": [
        {
            "name": "user_bio",
            "ado_type": "string-storage",
            "component_type": {
                "new": {
                    "restriction": "Private"
                }
            }
        },
        {
            "name": "public_announcement",
            "ado_type": "string-storage",
            "component_type": {
                "new": {
                    "restriction": "Public"
                }
            }
        }
    ]
}
```

### Content Management Systems
For managing text content:

1. **Deploy String Storage ADOs** for different content types
2. **Set appropriate restrictions** based on editorial workflow
3. **Store content** with proper access controls
4. **Track content ownership** for accountability

### Metadata Storage
For NFT and asset metadata:

1. **Store metadata** as JSON strings
2. **Link to assets** through references
3. **Update metadata** with proper permissions
4. **Query metadata** for display and analysis

### Configuration Management
For application settings:

1. **Store settings** as structured strings
2. **Parse configuration** in application logic
3. **Update settings** with appropriate access control
4. **Version control** through ownership tracking

### User-Generated Content
For social and community features:

1. **Allow users** to store personal content
2. **Moderate content** through restricted permissions
3. **Display content** from multiple storage contracts
4. **Track contributors** through data ownership

## String Validation

### Validation Rules
- **Non-empty**: Strings cannot be empty or whitespace-only
- **Length limits**: Practical limits imposed by blockchain storage
- **Content validation**: No content filtering (applications can implement their own)
- **Encoding**: Supports UTF-8 encoding for international characters

### Error Handling
- **EmptyString**: Thrown when attempting to store empty strings
- **Unauthorized**: Access denied based on restriction settings
- **InvalidFormat**: Malformed string storage structure

## State Management

### String Lifecycle
1. **Unset**: Initial state, no string stored
2. **Set**: String value stored
3. **Updated**: String value modified
4. **Deleted**: String removed, returns to unset state

### Data Ownership
- **Data Owner**: Address that most recently set the string
- **Contract Owner**: Address that owns the contract (different from data owner)
- **Permissions**: Based on restriction mode and contract ownership

## Performance Considerations

### Storage Efficiency
- **Efficient encoding**: Uses standard string storage
- **Size considerations**: Large strings increase storage costs
- **Gas costs**: Longer strings require more gas for storage
- **Retrieval speed**: Constant-time access regardless of string length

### Best Practices
- **Reasonable length**: Keep strings reasonably sized to minimize costs
- **Structured data**: Use JSON for complex data structures
- **Compression**: Consider compression for large text data
- **Batch operations**: Update multiple strings in single transactions when possible

## Important Notes

- **UTF-8 Support**: Full Unicode character support
- **No Content Filtering**: Applications responsible for content validation
- **Immutable Until Updated**: String remains constant until explicitly changed
- **Single String Storage**: One string per contract instance
- **Gas Efficiency**: Optimized for storage and retrieval operations
- **Cross-Chain Compatible**: Works across all Cosmos chains

## Example Workflows

### Personal Note Storage
```bash
# Deploy with private restriction
# User stores a personal note
{"set_value": {"value": {"string": "Remember to check the validator rewards tomorrow."}}}

# Query the note
{"get_value": {}}
# Response: {"value": "Remember to check the validator rewards tomorrow."}

# Update the note
{"set_value": {"value": {"string": "Validator rewards checked. Next: review staking options."}}}
```

### Public Message Board
```bash
# Deploy with public restriction
# User A posts a message
{"set_value": {"value": {"string": "Welcome everyone to our new community platform!"}}}

# User B updates the message (overwrites A's message)
{"set_value": {"value": {"string": "Thank you for the warm welcome! Excited to be here."}}}

# Check who posted the current message
{"get_data_owner": {}}
# Response: {"owner": "andr1...user_b_address"}
```

### Configuration Management
```bash
# Store application configuration
{"set_value": {"value": {"string": "{\"theme\": \"dark\", \"language\": \"en\", \"notifications\": true}"}}}

# Query configuration
{"get_value": {}}
# Application parses JSON and applies settings
```

The String Storage ADO provides a simple, secure way to store and manage text data in blockchain applications, with flexible access controls and validation that can adapt to various use cases from personal notes to complex content management systems.