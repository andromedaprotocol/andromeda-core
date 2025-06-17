# Andromeda Form ADO

## Introduction

The Andromeda Form ADO is a sophisticated data collection and management contract that provides secure, schema-validated form submission functionality with advanced access controls and time-based availability. This contract enables the creation of structured forms with JSON schema validation, configurable submission rules, and comprehensive form lifecycle management. The form system supports both public and restricted access patterns, making it ideal for surveys, applications, data collection campaigns, KYC processes, and any scenario requiring structured data submission with validation.

<b>Ado_type:</b> form

## Why Form ADO

The Form ADO serves as essential data collection infrastructure for applications requiring:

- **Survey and Research**: Collect structured survey responses with validation
- **Application Processes**: Handle job applications, grant applications, or membership forms
- **KYC/AML Compliance**: Collect and validate customer verification data
- **Data Collection Campaigns**: Gather structured data from users or participants
- **Registration Systems**: Manage event registrations, course enrollments, or service signups
- **Feedback Collection**: Structured feedback forms with data validation
- **Contest and Submission Management**: Handle contest entries with proper validation
- **Government and Administrative Forms**: Digital forms for government services
- **Medical Data Collection**: Structured medical forms with validation
- **Research Data Gathering**: Academic or scientific data collection with schema enforcement

The ADO provides schema-based validation, time-controlled availability, and flexible access controls for reliable form-based data collection.

## Key Features

### **Schema-Based Validation**
- **JSON schema integration**: Validates all submissions against predefined schemas
- **Schema ADO connection**: Links to external schema ADO for validation rules
- **Data integrity**: Ensures all submitted data meets specified requirements
- **Type safety**: Strong typing and validation for submitted data
- **Error reporting**: Clear validation error messages for invalid submissions

### **Flexible Form Configuration**
- **Time-based availability**: Optional start and end times for form availability
- **Multiple submission control**: Configure whether users can submit multiple times
- **Edit submission support**: Allow or disallow editing of existing submissions
- **Access control**: Restrict submissions to authorized addresses only
- **Dynamic form management**: Open and close forms administratively

### **Advanced Submission Management**
- **Unique submission IDs**: Auto-generated unique identifiers for each submission
- **User-based organization**: Track submissions by wallet address
- **Edit capabilities**: Allow users to modify their submissions when enabled
- **Deletion support**: Administrative deletion of specific submissions
- **Comprehensive querying**: Query submissions by ID, user, or retrieve all submissions

### **Access Control and Security**
- **Authorization lists**: Restrict submissions to specific addresses
- **Permission-based access**: Integration with Andromeda permission system
- **Owner controls**: Administrative functions for form management
- **Time-based security**: Automatic form availability based on configured times
- **User ownership**: Users can only edit their own submissions

## Form Lifecycle Management

### **Form States**
- **Unopened**: Form exists but submissions not yet accepted
- **Open**: Form actively accepting submissions
- **Closed**: Form no longer accepting submissions
- **Reopened**: Previously closed form reopened for submissions

### **Time-based Control**
The form automatically manages availability based on configured times:
1. **Before start time**: Form is closed, submissions rejected
2. **Between start and end time**: Form is open, submissions accepted
3. **After end time**: Form is closed, submissions rejected
4. **Administrative override**: Owner can manually open/close regardless of time

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
    "schema_ado_address": "andr1schema_contract...",
    "authorized_addresses_for_submission": [
        "andr1user1...",
        "andr1user2...",
        "andr1user3..."
    ],
    "form_config": {
        "start_time": {
            "at_time": "1672617600000000000"
        },
        "end_time": {
            "at_time": "1675209600000000000"
        },
        "allow_multiple_submissions": false,
        "allow_edit_submission": true
    },
    "custom_key_for_notifications": "survey_2024_q1"
}
```

**Parameters**:
- **schema_ado_address**: Address of the schema ADO that validates submissions
- **authorized_addresses_for_submission**: Optional list of addresses authorized to submit
  - If empty/null: Anyone can submit (public form)
  - If provided: Only listed addresses can submit (restricted form)
- **form_config**: Configuration settings for form behavior
  - **start_time**: Optional time when form opens for submissions
  - **end_time**: Optional time when form closes for submissions
  - **allow_multiple_submissions**: Whether users can submit multiple times
  - **allow_edit_submission**: Whether users can edit their existing submissions
- **custom_key_for_notifications**: Optional key for external notification systems

**Validation**:
- Schema ADO address must be valid and accessible
- End time must be after start time if both are provided
- Custom notification key is used for integration with external services

## ExecuteMsg

### SubmitForm
Submits form data for validation and storage.

```rust
SubmitForm {
    data: String,
}
```

```json
{
    "submit_form": {
        "data": "{\"name\": \"John Doe\", \"email\": \"john@example.com\", \"age\": 30}"
    }
}
```

**Usage**: Submit JSON data that will be validated against the configured schema.

**Process**:
1. Check if user is authorized to submit (if restrictions exist)
2. Validate form is currently open
3. Validate data against schema ADO
4. Check multiple submission rules
5. Store submission with unique ID

**Requirements**:
- Form must be open (within time window if configured)
- User must be authorized (if authorization list exists)
- Data must be valid JSON that passes schema validation
- If multiple submissions disabled, user cannot already have a submission

### EditSubmission
Edits an existing submission.

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
        "wallet_address": "andr1user...",
        "data": "{\"name\": \"John Smith\", \"email\": \"john.smith@example.com\", \"age\": 31}"
    }
}
```

**Requirements**:
- Form must allow edit submissions (configured during instantiation)
- Form must be currently open
- Submission must exist
- Only the original submitter can edit their submission
- New data must pass schema validation

### DeleteSubmission
Deletes a specific submission (admin-only).

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
        "wallet_address": "andr1user..."
    }
}
```

**Authorization**: Only contract owner/admin can delete submissions
**Effect**: Permanently removes the specified submission from storage

### OpenForm
Manually opens the form for submissions (admin-only).

```rust
OpenForm {}
```

```json
{
    "open_form": {}
}
```

**Authorization**: Only contract owner can execute
**Effects**:
- Sets start time to current time if not already open
- Clears end time if form was previously closed
- Allows submissions to be accepted

### CloseForm
Manually closes the form to submissions (admin-only).

```rust
CloseForm {}
```

```json
{
    "close_form": {}
}
```

**Authorization**: Only contract owner can execute
**Effect**: Sets end time to current time, preventing new submissions

## QueryMsg

### GetSchema
Returns the JSON schema used for validation.

```rust
#[returns(GetSchemaResponse)]
GetSchema {}

pub struct GetSchemaResponse {
    pub schema: String,
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
    "schema": "{\"type\": \"object\", \"properties\": {\"name\": {\"type\": \"string\"}, \"email\": {\"type\": \"string\", \"format\": \"email\"}}}"
}
```

### GetAllSubmissions
Returns all submissions to the form.

```rust
#[returns(GetAllSubmissionsResponse)]
GetAllSubmissions {}

pub struct GetAllSubmissionsResponse {
    pub all_submissions: Vec<SubmissionInfo>,
}

pub struct SubmissionInfo {
    pub submission_id: u64,
    pub wallet_address: Addr,
    pub data: String,
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
            "data": "{\"name\": \"John Doe\", \"email\": \"john@example.com\"}"
        },
        {
            "submission_id": 2,
            "wallet_address": "andr1user2...",
            "data": "{\"name\": \"Jane Smith\", \"email\": \"jane@example.com\"}"
        }
    ]
}
```

### GetSubmission
Returns a specific submission by ID and wallet address.

```rust
#[returns(GetSubmissionResponse)]
GetSubmission {
    submission_id: u64,
    wallet_address: AndrAddr,
}

pub struct GetSubmissionResponse {
    pub submission: Option<SubmissionInfo>,
}
```

```json
{
    "get_submission": {
        "submission_id": 1,
        "wallet_address": "andr1user..."
    }
}
```

**Response:**
```json
{
    "submission": {
        "submission_id": 1,
        "wallet_address": "andr1user...",
        "data": "{\"name\": \"John Doe\", \"email\": \"john@example.com\"}"
    }
}
```

### GetSubmissionIds
Returns all submission IDs for a specific wallet address.

```rust
#[returns(GetSubmissionIdsResponse)]
GetSubmissionIds {
    wallet_address: AndrAddr,
}

pub struct GetSubmissionIdsResponse {
    pub submission_ids: Vec<u64>,
}
```

```json
{
    "get_submission_ids": {
        "wallet_address": "andr1user..."
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
Returns the current status of the form (open or closed).

```rust
#[returns(GetFormStatusResponse)]
GetFormStatus {}

pub enum GetFormStatusResponse {
    Opened,
    Closed,
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

## Usage Examples

### Public Survey Form
```json
{
    "schema_ado_address": "andr1survey_schema...",
    "authorized_addresses_for_submission": null,
    "form_config": {
        "start_time": {
            "at_time": "1672617600000000000"
        },
        "end_time": {
            "at_time": "1675209600000000000"
        },
        "allow_multiple_submissions": false,
        "allow_edit_submission": true
    },
    "custom_key_for_notifications": "customer_satisfaction_2024"
}
```

### Restricted Application Form
```json
{
    "schema_ado_address": "andr1application_schema...",
    "authorized_addresses_for_submission": [
        "andr1applicant1...",
        "andr1applicant2...",
        "andr1applicant3..."
    ],
    "form_config": {
        "start_time": null,
        "end_time": {
            "at_time": "1675209600000000000"
        },
        "allow_multiple_submissions": false,
        "allow_edit_submission": false
    },
    "custom_key_for_notifications": "job_application_2024"
}
```

### Ongoing Feedback Collection
```json
{
    "schema_ado_address": "andr1feedback_schema...",
    "authorized_addresses_for_submission": null,
    "form_config": {
        "start_time": null,
        "end_time": null,
        "allow_multiple_submissions": true,
        "allow_edit_submission": false
    },
    "custom_key_for_notifications": null
}
```

## Operational Examples

### Submit Form Data
```json
{
    "submit_form": {
        "data": "{\"name\": \"Alice Johnson\", \"email\": \"alice@company.com\", \"department\": \"Engineering\", \"feedback\": \"Great platform!\"}"
    }
}
```

### Edit Existing Submission
```json
{
    "edit_submission": {
        "submission_id": 5,
        "wallet_address": "andr1alice...",
        "data": "{\"name\": \"Alice Johnson\", \"email\": \"alice@company.com\", \"department\": \"Engineering\", \"feedback\": \"Excellent platform with room for improvement!\"}"
    }
}
```

### Manually Close Form
```json
{
    "close_form": {}
}
```

### Query User's Submissions
```json
{
    "get_submission_ids": {
        "wallet_address": "andr1alice..."
    }
}
```

### Check Form Status
```json
{
    "get_form_status": {}
}
```

### Get Specific Submission
```json
{
    "get_submission": {
        "submission_id": 5,
        "wallet_address": "andr1alice..."
    }
}
```

## Integration Patterns

### With App Contract
Form can be integrated for data collection workflows:

```json
{
    "components": [
        {
            "name": "user_survey",
            "ado_type": "form",
            "component_type": {
                "new": {
                    "schema_ado_address": "andr1survey_schema...",
                    "authorized_addresses_for_submission": null,
                    "form_config": {
                        "start_time": null,
                        "end_time": {
                            "at_time": "1675209600000000000"
                        },
                        "allow_multiple_submissions": false,
                        "allow_edit_submission": true
                    },
                    "custom_key_for_notifications": "user_feedback_q1"
                }
            }
        },
        {
            "name": "survey_schema",
            "ado_type": "schema",
            "component_type": {
                "new": {
                    "schema": "{\"type\": \"object\", \"properties\": {\"rating\": {\"type\": \"number\", \"minimum\": 1, \"maximum\": 5}}}"
                }
            }
        }
    ]
}
```

### Survey and Research Systems
For structured data collection:

1. **Deploy schema ADO** with validation rules for survey questions
2. **Deploy form ADO** linked to schema with time restrictions
3. **Configure access controls** for target respondents
4. **Collect submissions** with automatic validation
5. **Analyze results** through comprehensive querying

### Application and Registration Processes
For application management:

1. **Create application schema** with required fields and validation
2. **Set up restricted form** with authorized applicant addresses
3. **Enable edit functionality** for application updates
4. **Manage application deadlines** through time-based controls
5. **Review applications** through administrative queries

### KYC and Compliance Data Collection
For regulatory compliance:

1. **Define compliance schemas** with required verification fields
2. **Create restricted forms** for verified users only
3. **Disable multiple submissions** to prevent duplicate entries
4. **Enable editing** for information updates
5. **Maintain audit trails** through immutable submission records

### Research Data Gathering
For academic or scientific data collection:

1. **Design research schemas** with specific data requirements
2. **Configure time-bounded studies** with start and end dates
3. **Manage participant access** through authorization lists
4. **Collect structured data** with validation guarantees
5. **Export research data** through comprehensive querying

## Advanced Features

### **Schema Integration**
- **External validation**: Links to separate schema ADO for validation rules
- **Schema evolution**: Update validation rules through schema ADO updates
- **Type safety**: Strong typing enforcement through JSON schema validation
- **Custom validation**: Support for complex validation rules and patterns

### **Time-based Management**
- **Flexible scheduling**: Optional start and end times for form availability
- **Administrative override**: Manual open/close regardless of scheduled times
- **Automatic management**: Forms automatically open and close based on configuration
- **Real-time status**: Query current form status for availability checking

### **Submission Controls**
- **Multiple submission logic**: Configurable multiple submission behavior
- **Edit capabilities**: Optional editing of existing submissions
- **User ownership**: Users can only edit their own submissions
- **Administrative deletion**: Owner can delete any submission

### **Access Control Integration**
- **Authorization lists**: Restrict submissions to specific addresses
- **Permission system**: Integration with Andromeda permission framework
- **Public/private modes**: Support for both open and restricted forms
- **Dynamic authorization**: Update authorized addresses through permission system

## Security Features

### **Data Validation**
- **Schema enforcement**: All submissions validated against predefined schemas
- **Type checking**: Strong typing prevents invalid data storage
- **Validation reporting**: Clear error messages for validation failures
- **Data integrity**: Guarantee that stored data meets requirements

### **Access Control**
- **User authorization**: Verify permission before accepting submissions
- **Owner privileges**: Administrative functions restricted to contract owner
- **Address validation**: Comprehensive validation of all addresses
- **Unauthorized prevention**: Prevent unauthorized access to form functions

### **Submission Security**
- **User ownership**: Users can only modify their own submissions
- **Edit restrictions**: Edit functionality can be disabled for security
- **Administrative controls**: Owner can manage all submissions
- **Audit trail**: Complete record of all submissions and modifications

### **Time-based Security**
- **Automatic enforcement**: Time restrictions automatically enforced
- **Manual override**: Administrative control over form availability
- **Precise timing**: Microsecond precision for time-based controls
- **Status transparency**: Clear status reporting for availability

## Important Notes

- **Schema dependency**: Form requires a linked schema ADO for validation
- **Time zone handling**: All times are in UTC/blockchain time
- **Data format**: All submission data must be valid JSON strings
- **Edit restrictions**: Editing requires explicit configuration during instantiation
- **Authorization lists**: Empty list means public access, populated list means restricted
- **Submission uniqueness**: Submission IDs are unique and auto-generated
- **Administrative power**: Owner can override most restrictions and manage all data
- **Gas considerations**: Large forms with many submissions may require pagination

## Common Workflow

### 1. **Deploy Schema ADO**
```json
{
    "schema": "{\"type\": \"object\", \"properties\": {\"name\": {\"type\": \"string\"}, \"email\": {\"type\": \"string\", \"format\": \"email\"}}}"
}
```

### 2. **Deploy Form ADO**
```json
{
    "schema_ado_address": "andr1schema...",
    "authorized_addresses_for_submission": null,
    "form_config": {
        "start_time": null,
        "end_time": {
            "at_time": "1675209600000000000"
        },
        "allow_multiple_submissions": false,
        "allow_edit_submission": true
    },
    "custom_key_for_notifications": "survey_2024"
}
```

### 3. **Submit Form Data**
```json
{
    "submit_form": {
        "data": "{\"name\": \"John Doe\", \"email\": \"john@example.com\"}"
    }
}
```

### 4. **Query Submissions**
```json
{
    "get_all_submissions": {}
}
```

### 5. **Edit Submission**
```json
{
    "edit_submission": {
        "submission_id": 1,
        "wallet_address": "andr1user...",
        "data": "{\"name\": \"John Smith\", \"email\": \"john.smith@example.com\"}"
    }
}
```

### 6. **Check Form Status**
```json
{
    "get_form_status": {}
}
```

### 7. **Close Form**
```json
{
    "close_form": {}
}
```

The Form ADO provides comprehensive form management infrastructure for the Andromeda ecosystem, enabling secure, validated, and time-controlled data collection with flexible access controls and administrative capabilities.