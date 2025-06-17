# Andromeda Form ADO

## Introduction

The Andromeda Form ADO is a comprehensive form management contract that provides structured data collection with schema validation, time-based controls, and submission management. It enables applications to create forms with configurable submission rules, edit capabilities, and administrative controls, making it ideal for surveys, applications, registrations, feedback collection, and any scenario requiring structured data gathering with validation.

<b>Ado_type:</b> form

## Why Form ADO

The Form ADO serves as a powerful data collection engine for applications requiring:

- **Survey Systems**: Collect structured responses with validation and time controls
- **Application Forms**: Manage job applications, grant submissions, or registration forms
- **Feedback Collection**: Gather user feedback with structured questions and responses
- **Registration Systems**: Handle event registrations, membership applications, or sign-ups
- **Voting and Polls**: Implement structured voting with schema validation
- **Data Collection**: Scientific data gathering with standardized formats
- **KYC/Compliance**: Collect compliance information with validation requirements
- **Contest Submissions**: Manage contest entries with submission controls
- **Questionnaires**: Create research questionnaires with configurable parameters
- **Approval Workflows**: Multi-step approval processes with structured data

The ADO integrates with Schema ADO for data validation and supports configurable time windows, multiple submissions, edit capabilities, and administrative controls.

## InstantiateMsg

```rust
pub struct InstantiateMsg {
    pub schema_ado_address: AndrAddr,
    pub authorized_addresses_for_submission: Option<Vec<AndrAddr>>,
    pub form_config: FormConfig,
    pub custom_key_for_notifications: Option<String>,
}

pub struct FormConfig {
    pub start_time: Option<Expiry>,
    pub end_time: Option<Expiry>,
    pub allow_multiple_submissions: bool,
    pub allow_edit_submission: bool,
}
```

```json
{
    "schema_ado_address": "andr1schemacontract...",
    "authorized_addresses_for_submission": [
        "andr1user1...",
        "andr1user2..."
    ],
    "form_config": {
        "start_time": {
            "at_time": "1640995200000000000"
        },
        "end_time": {
            "at_time": "1641081600000000000"
        },
        "allow_multiple_submissions": false,
        "allow_edit_submission": true
    },
    "custom_key_for_notifications": "survey_form_2024"
}
```

- **schema_ado_address**: Address of the Schema ADO that validates form submissions
- **authorized_addresses_for_submission**: Optional list of addresses allowed to submit (if not provided, anyone can submit)
- **form_config**: Configuration for form behavior
  - **start_time**: Optional form opening time
  - **end_time**: Optional form closing time  
  - **allow_multiple_submissions**: Whether users can submit multiple times
  - **allow_edit_submission**: Whether users can edit their submissions
- **custom_key_for_notifications**: Optional custom identifier for notifications

## ExecuteMsg

### SubmitForm
Submits form data that will be validated against the configured schema.

_**Note:** Subject to authorization and time restrictions configured during instantiation._

```rust
SubmitForm { 
    data: String 
}
```

```json
{
    "submit_form": {
        "data": "{\"name\": \"John Doe\", \"email\": \"john@example.com\", \"age\": 30, \"feedback\": \"Great platform!\"}"
    }
}
```

### DeleteSubmission
Removes a specific submission by ID and wallet address.

_**Note:** Only contract owner can execute this operation._

```rust
DeleteSubmission {
    submission_id: u64,
    wallet_address: AndrAddr,
}
```

```json
{
    "delete_submission": {
        "submission_id": 1,
        "wallet_address": "andr1useraddress..."
    }
}
```

### EditSubmission
Edits an existing submission if editing is enabled in form configuration.

_**Note:** Only the original submitter can edit their submission, and editing must be enabled in form config._

```rust
EditSubmission {
    submission_id: u64,
    wallet_address: AndrAddr,
    data: String,
}
```

```json
{
    "edit_submission": {
        "submission_id": 1,
        "wallet_address": "andr1useraddress...",
        "data": "{\"name\": \"John Doe\", \"email\": \"john.doe@example.com\", \"age\": 31, \"feedback\": \"Excellent platform with great features!\"}"
    }
}
```

### OpenForm
Manually opens the form for submissions.

_**Note:** Only contract owner can execute this operation._

```rust
OpenForm {}
```

```json
{
    "open_form": {}
}
```

### CloseForm
Manually closes the form to prevent new submissions.

_**Note:** Only contract owner can execute this operation._

```rust
CloseForm {}
```

```json
{
    "close_form": {}
}
```

## QueryMsg

### GetSchema
Returns the schema configuration used for validating form submissions.

```rust
pub enum QueryMsg {
    #[returns(GetSchemaResponse)]
    GetSchema {},
}
```

```json
{
    "get_schema": {}
}
```

**Response:**
```json
{
    "schema": "{\"type\": \"object\", \"properties\": {\"name\": {\"type\": \"string\"}, \"email\": {\"type\": \"string\"}, \"age\": {\"type\": \"number\"}}}"
}
```

### GetAllSubmissions
Returns all form submissions with pagination support.

```rust
pub enum QueryMsg {
    #[returns(GetAllSubmissionsResponse)]
    GetAllSubmissions {},
}
```

```json
{
    "get_all_submissions": {}
}
```

**Response:**
```json
{
    "all_submissions": [
        {
            "submission_id": 1,
            "wallet_address": "andr1user1...",
            "data": "{\"name\": \"John Doe\", \"email\": \"john@example.com\", \"age\": 30}"
        },
        {
            "submission_id": 2,
            "wallet_address": "andr1user2...",
            "data": "{\"name\": \"Jane Smith\", \"email\": \"jane@example.com\", \"age\": 28}"
        }
    ]
}
```

### GetSubmission
Returns a specific submission by ID and wallet address.

```rust
pub enum QueryMsg {
    #[returns(GetSubmissionResponse)]
    GetSubmission {
        submission_id: u64,
        wallet_address: AndrAddr,
    },
}
```

```json
{
    "get_submission": {
        "submission_id": 1,
        "wallet_address": "andr1useraddress..."
    }
}
```

**Response:**
```json
{
    "submission": {
        "submission_id": 1,
        "wallet_address": "andr1useraddress...",
        "data": "{\"name\": \"John Doe\", \"email\": \"john@example.com\", \"age\": 30}"
    }
}
```

### GetSubmissionIds
Returns all submission IDs for a specific wallet address.

```rust
pub enum QueryMsg {
    #[returns(GetSubmissionIdsResponse)]
    GetSubmissionIds { wallet_address: AndrAddr },
}
```

```json
{
    "get_submission_ids": {
        "wallet_address": "andr1useraddress..."
    }
}
```

**Response:**
```json
{
    "submission_ids": [1, 3, 7]
}
```

### GetFormStatus
Returns the current status of the form (opened or closed).

```rust
pub enum QueryMsg {
    #[returns(GetFormStatusResponse)]
    GetFormStatus {},
}
```

```json
{
    "get_form_status": {}
}
```

**Response:**
```json
"Opened"
```
or
```json
"Closed"
```

## Form Configuration Examples

### Open Survey (Public)
```json
{
    "form_config": {
        "start_time": null,
        "end_time": null,
        "allow_multiple_submissions": true,
        "allow_edit_submission": true
    }
}
```

### Limited Time Application
```json
{
    "form_config": {
        "start_time": {
            "at_time": "1640995200000000000"
        },
        "end_time": {
            "at_time": "1641081600000000000"
        },
        "allow_multiple_submissions": false,
        "allow_edit_submission": false
    }
}
```

### Registration with Edit Capability
```json
{
    "form_config": {
        "start_time": {
            "at_time": "1640995200000000000"
        },
        "end_time": null,
        "allow_multiple_submissions": false,
        "allow_edit_submission": true
    }
}
```

### Restricted Access Form
```json
{
    "authorized_addresses_for_submission": [
        "andr1member1...",
        "andr1member2...",
        "andr1member3..."
    ],
    "form_config": {
        "allow_multiple_submissions": true,
        "allow_edit_submission": false
    }
}
```

## Schema Integration

### Schema Validation
The Form ADO integrates with Schema ADO for data validation:

1. **Form data** is validated against the schema before storage
2. **Invalid submissions** are rejected with validation errors
3. **Schema updates** can be managed through the Schema ADO
4. **Consistent data structure** is enforced across all submissions

### Example Schema (JSON Schema format)
```json
{
    "type": "object",
    "required": ["name", "email"],
    "properties": {
        "name": {
            "type": "string",
            "minLength": 1,
            "maxLength": 100
        },
        "email": {
            "type": "string",
            "format": "email"
        },
        "age": {
            "type": "number",
            "minimum": 18,
            "maximum": 120
        },
        "feedback": {
            "type": "string",
            "maxLength": 1000
        }
    }
}
```

## Usage Examples

### Survey Form
```json
{
    "submit_form": {
        "data": "{\"satisfaction\": 8, \"recommendation\": true, \"comments\": \"Great user experience and intuitive interface.\"}"
    }
}
```

### Job Application
```json
{
    "submit_form": {
        "data": "{\"name\": \"Alice Johnson\", \"position\": \"Software Engineer\", \"experience\": 5, \"skills\": [\"Rust\", \"JavaScript\", \"Blockchain\"], \"resume_url\": \"https://example.com/resume.pdf\"}"
    }
}
```

### Event Registration
```json
{
    "submit_form": {
        "data": "{\"attendee_name\": \"Bob Wilson\", \"email\": \"bob@example.com\", \"dietary_restrictions\": \"Vegetarian\", \"t_shirt_size\": \"L\"}"
    }
}
```

### Feedback Collection
```json
{
    "submit_form": {
        "data": "{\"rating\": 9, \"category\": \"UI/UX\", \"description\": \"The new dashboard design is much more intuitive.\", \"priority\": \"medium\"}"
    }
}
```

## Integration Patterns

### With App Contract
The Form ADO can be integrated into App contracts for data collection:

```json
{
    "components": [
        {
            "name": "user_survey",
            "ado_type": "form",
            "component_type": {
                "new": {
                    "schema_ado_address": "andr1schema...",
                    "form_config": {
                        "allow_multiple_submissions": false,
                        "allow_edit_submission": true
                    }
                }
            }
        }
    ]
}
```

### Multi-Stage Applications
For complex application processes:

1. **Deploy multiple Form ADOs** for different stages
2. **Configure different schemas** for each stage
3. **Control submission timing** with start/end times
4. **Track progress** through submission IDs

### Data Collection Pipelines
For research and analytics:

1. **Design data schema** for consistency
2. **Configure collection parameters** (timing, access, editing)
3. **Collect responses** with validation
4. **Export data** for analysis

### Approval Workflows
For business processes:

1. **Collect initial applications** through forms
2. **Allow editing** during review period
3. **Control access** to authorized reviewers
4. **Track submission history** for audit

## Access Control

### Submission Authorization
- **Open Forms**: Anyone can submit (no authorized_addresses_for_submission)
- **Restricted Forms**: Only authorized addresses can submit
- **Owner Controls**: Owner can always manage form state and submissions

### Time-Based Controls
- **Start Time**: Form opens at specified time
- **End Time**: Form closes at specified time
- **Manual Override**: Owner can manually open/close regardless of time settings

### Edit Permissions
- **Original Submitter**: Can edit their own submissions if editing is enabled
- **Owner Control**: Owner can delete any submission
- **Edit Window**: Editing subject to form time restrictions

## Important Notes

- **Schema Dependency**: Requires a Schema ADO for data validation
- **Data Persistence**: All submissions are stored permanently until deleted
- **Gas Considerations**: Large form data increases storage costs
- **Submission Tracking**: Each submission gets a unique ID per user
- **Time Validation**: Form status checked against blockchain time
- **Edit History**: No built-in version history for edited submissions

## Performance Considerations

### Optimization Strategies
- **Efficient schemas**: Design schemas to minimize validation overhead
- **Reasonable data size**: Keep form data within practical limits
- **Batch operations**: Process multiple submissions efficiently
- **Query pagination**: Large forms may need pagination for submission queries

### Scalability
- **Submission limits**: No hard limits but practical constraints from storage costs
- **Schema complexity**: More complex schemas require more validation gas
- **Edit frequency**: Frequent edits increase transaction costs
- **Query performance**: Large submission datasets may affect query speed

The Form ADO provides a comprehensive solution for structured data collection in blockchain applications, offering the flexibility and control needed for complex form workflows while maintaining data integrity through schema validation and proper access controls.