#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::{to_json_binary, Addr};
    use serde_cw_value::{to_value, Value};

    use crate::{error::JsonError, JSON};

    // Test cases for JSON parsing and manipulation
    mod json_parsing {
        use super::*;

        #[test]
        fn test_vector_of_json() {
            // Parse JSON string with a vector of JSON objects
            let json_str = r#"{
                "data": [
                    {"value": 10},
                    {"value": 20},
                    {"value": 30}
                ]
            }"#;
            let parsed_json = JSON::try_from(json_str).unwrap();

            // Get a vector of JSON objects
            let data = parsed_json.get("data").unwrap();
            assert_eq!(
                data,
                Some(&Value::Seq(vec![
                    Value::Map(BTreeMap::from([(
                        Value::String("value".to_string()),
                        Value::U64(10)
                    )])),
                    Value::Map(BTreeMap::from([(
                        Value::String("value".to_string()),
                        Value::U64(20)
                    )])),
                    Value::Map(BTreeMap::from([(
                        Value::String("value".to_string()),
                        Value::U64(30)
                    )])),
                ]))
            );
        }

        #[test]
        fn test_complex_nested_json_with_arrays() {
            // Parse JSON string with complex nested structure and arrays
            let json_str = r#"
            {
                "user": {
                    "id": 1,
                    "name": "Alice",
                    "contacts": {
                        "email": "alice@example.com",
                        "phone": "123-456-7890"
                    },
                    "preferences": {
                        "notifications": {
                            "email": true,
                            "sms": false
                        }
                    },
                    "friends": [
                        {
                            "id": 2,
                            "name": "Bob"
                        },
                        {
                            "id": 3,
                            "name": "Charlie"
                        }
                    ]
                }
            }"#;

            let parsed_json = JSON::try_from(json_str).unwrap();

            // Get values at multiple levels
            let user_id = parsed_json.get("user.id").unwrap();
            assert_eq!(user_id, Some(&Value::U64(1)));

            let user_name = parsed_json.get("user.name").unwrap();
            assert_eq!(user_name, Some(&Value::String("Alice".to_string())));

            let user_email = parsed_json.get("user.contacts.email").unwrap();
            assert_eq!(
                user_email,
                Some(&Value::String("alice@example.com".to_string()))
            );

            let user_sms_notifications = parsed_json
                .get("user.preferences.notifications.sms")
                .unwrap();
            assert_eq!(user_sms_notifications, Some(&Value::Bool(false)));

            // Get values from the array of friends
            let first_friend_name = parsed_json.get("user.friends.0.name").unwrap();
            assert_eq!(first_friend_name, Some(&Value::String("Bob".to_string())));

            let second_friend_id = parsed_json.get("user.friends.1.id").unwrap();
            assert_eq!(second_friend_id, Some(&Value::U64(3)));
        }

        #[test]
        fn test_empty_json() {
            // Parse an empty JSON object
            let json_str = r#"{}"#;
            let parsed_json = JSON::try_from(json_str).unwrap();

            // Attempt to get a non-existent key
            let result = parsed_json.get("non_existent_key").unwrap();
            assert_eq!(result, None);
        }

        #[test]
        fn test_invalid_json() {
            // Parse an invalid JSON string
            let json_str = r#"{"data": [1, 2, 3, ]"#; // Missing closing brace
            let result = JSON::try_from(json_str);
            assert!(result.is_err());
        }
    }

    // Test cases for JSON updates
    mod json_updates {
        use super::*;

        #[test]
        fn test_vector_update() {
            // Parse JSON string with a vector
            let json_str = r#"{
                "data": [10, 20, 30]
            }"#;
            let mut parsed_json = JSON::try_from(json_str).unwrap();

            // Update a vector value
            let new_data = Value::Seq(vec![Value::U64(40), Value::U64(50)]);
            parsed_json.update("data", new_data, None).unwrap();

            // Get the updated vector value
            let updated_data = parsed_json.get("data").unwrap();
            assert_eq!(
                updated_data,
                Some(&Value::Seq(vec![Value::U64(40), Value::U64(50)]))
            );
        }

        #[test]
        fn test_nested_array_update() {
            // Parse JSON string with an array of nested objects
            let json_str = r#"
            {
                "data": [
                    {
                        "numbers": [10, 20, 30]
                    },
                    {
                        "numbers": [40, 50, 60]
                    }
                ]
            }"#;

            let mut parsed_json = JSON::try_from(json_str).unwrap();

            // Get the array of nested objects
            let array_value = parsed_json.get("data").unwrap();
            assert_eq!(
                array_value,
                Some(&Value::Seq(vec![
                    Value::Map({
                        let mut map = BTreeMap::new();
                        map.insert(
                            Value::String("numbers".to_string()),
                            Value::Seq(vec![Value::U64(10), Value::U64(20), Value::U64(30)]),
                        );
                        map
                    }),
                    Value::Map({
                        let mut map = BTreeMap::new();
                        map.insert(
                            Value::String("numbers".to_string()),
                            Value::Seq(vec![Value::U64(40), Value::U64(50), Value::U64(60)]),
                        );
                        map
                    })
                ]))
            );

            // Update the array value in the nested object at index 1
            let new_value = Value::Seq(vec![Value::U64(70), Value::U64(80)]);
            parsed_json
                .update("data.1.numbers", new_value.clone(), None)
                .unwrap();

            // Get the updated array value in the nested object at index 1
            let updated_array_value = parsed_json.get("data.1.numbers").unwrap();
            assert_eq!(updated_array_value, Some(&new_value));

            parsed_json
                .update("data.1.numbers.1", Value::U64(10), None)
                .unwrap();

            // Get the updated array value in the nested object at index 1
            let updated_array_value = parsed_json.get("data.1.numbers.1").unwrap();
            assert_eq!(updated_array_value, Some(&Value::U64(10)));
        }

        #[test]
        fn test_binary_update() {
            #[cw_serde]
            struct Dummy {
                pub address: Addr,
                pub owner: Addr,
            }
            let mut data = Dummy {
                owner: Addr::unchecked("owner"),
                address: Addr::unchecked("address"),
            };
            let encoded = to_json_binary(&data).unwrap();

            // Start parsing
            let mut parsed_json = JSON::try_from(encoded).unwrap();

            // Update a vector value
            parsed_json
                .update(
                    "owner",
                    to_value(Addr::unchecked("new_owner")).unwrap(),
                    None,
                )
                .unwrap();

            // Get the updated vector value
            let updated_data: Addr = parsed_json
                .get("owner")
                .unwrap()
                .unwrap()
                .clone()
                .deserialize_into()
                .unwrap();
            assert_eq!(updated_data, Addr::unchecked("new_owner"));

            data.owner = Addr::unchecked("new_owner");
            let encoded = to_json_binary(&data).unwrap();
            let json_encoded = to_json_binary(&parsed_json).unwrap();
            assert_eq!(encoded, json_encoded);

            // Also check deserialized comparison for the values
            assert_eq!(data, parsed_json.to_any().unwrap());
        }

        #[test]
        fn test_update_nested_array() {
            let json_str = r#"{
                "data": {
                    "numbers": [1, 2, 3]
                }
            }"#;

            let mut parsed_json = JSON::try_from(json_str).unwrap();

            // Update the second element in the numbers array
            parsed_json
                .update("data.numbers.1", Value::U64(20), None)
                .unwrap();

            // Get the updated array value in the nested object at index 1
            let updated_array_value = parsed_json.get("data.numbers.1").unwrap();
            assert_eq!(updated_array_value, Some(&Value::U64(20)));
        }

        #[test]
        fn test_delete_array_element() {
            let json_str = r#"{
                "data": {
                    "numbers": [1, 2, 3, 4, 5]
                }
            }"#;

            let mut parsed_json = JSON::try_from(json_str).unwrap();

            // Delete the third element in the numbers array (index 2)
            parsed_json.delete("data.numbers.2").unwrap();

            // Get the updated array value in the nested object
            let updated_array_value = parsed_json.get("data.numbers").unwrap();
            assert_eq!(
                updated_array_value,
                Some(&Value::Seq(vec![
                    Value::U64(1),
                    Value::U64(2),
                    Value::U64(4),
                    Value::U64(5),
                ]))
            );
        }

        #[test]
        fn test_delete_key() {
            let json_str = r#"{
                "data": {
                    "key_to_delete": "value",
                    "numbers": [1, 2, 3]
                }
            }"#;

            let mut parsed_json = JSON::try_from(json_str).unwrap();

            // Delete the key "key_to_delete"
            parsed_json.delete("data.key_to_delete").unwrap();

            // Attempt to get the deleted key
            let deleted_key_value = parsed_json.get("data.key_to_delete").unwrap();
            assert_eq!(deleted_key_value, None);

            assert_eq!(parsed_json.to_string(), r#"{"data":{"numbers":[1,2,3]}}"#);
        }

        #[test]
        fn test_invalid_delete() {
            let json_str = r#"{
                "data": {
                    "numbers": [1, 2, 3]
                }
            }"#;

            let mut parsed_json = JSON::try_from(json_str).unwrap();

            // Attempt to delete a non-existent key
            let result = parsed_json.delete("data.invalid_key");
            assert!(result.is_err(), "Delete should fail");
            assert_eq!(
                result.err().unwrap(),
                JsonError::KeyNotFound("invalid_key".to_string())
            );
        }

        #[test]
        fn test_invalid_update() {
            let json_str = r#"{
                "data": {
                    "numbers": [1, 2, 3]
                }
            }"#;

            let mut parsed_json = JSON::try_from(json_str).unwrap();

            // Attempt to update an invalid key
            let result = parsed_json.update("data.invalid_key", Value::U64(10), None);
            assert!(result.is_err(), "Update should fail");
            assert_eq!(
                result.err().unwrap(),
                JsonError::KeyNotFound("invalid_key".to_string())
            );
        }

        #[test]
        fn test_get_non_existent_key() {
            let json_str = r#"{
                "data": {
                    "numbers": [1, 2, 3]
                }
            }"#;

            let parsed_json = JSON::try_from(json_str).unwrap();

            // Attempt to get a non-existent key
            let result = parsed_json.get("data.non_existent_key").unwrap();
            assert_eq!(result, None);
        }

        #[test]
        fn test_upsert() {
            let json_str = r#"{
                "data": {
                    "numbers": [1, 2, 3]
                }
            }"#;

            let mut parsed_json = JSON::try_from(json_str).unwrap();

            // Upsert a new key
            parsed_json
                .update("data.new_key", Value::U64(10), Some(true))
                .unwrap();
            let new_key_value = parsed_json.get("data.new_key").unwrap();
            assert_eq!(new_key_value, Some(&Value::U64(10)));

            // Upsert an existing key
            parsed_json
                .update("data.numbers.1", Value::U64(20), Some(true))
                .unwrap();
            let updated_array_value = parsed_json.get("data.numbers.1").unwrap();
            assert_eq!(updated_array_value, Some(&Value::U64(20)));

            let final_json_string: String = parsed_json.try_into().unwrap();
            assert_eq!(
                final_json_string,
                r#"{"data":{"new_key":10,"numbers":[1,20,3]}}"#
            );
        }

        #[test]
        fn test_complex_json_update_upsert_delete() {
            // Parse JSON string with a complex structure
            let json_str = r#"
            {
                "user": {
                    "id": 1,
                    "name": "Alice",
                    "contacts": {
                        "email": "alice@example.com",
                        "phone": "123-456-7890"
                    },
                    "preferences": {
                        "notifications": {
                            "email": true,
                            "sms": false
                        }
                    }
                }
            }"#;

            let mut parsed_json = JSON::try_from(json_str).unwrap();

            // Update existing key
            parsed_json
                .update("user.name", Value::String("Alicia".to_string()), None)
                .unwrap();
            let updated_name = parsed_json.get("user.name").unwrap();
            assert_eq!(updated_name, Some(&Value::String("Alicia".to_string())));

            // Upsert a new key
            parsed_json
                .update("user.age", Value::U64(30), Some(true))
                .unwrap();
            let new_age_value = parsed_json.get("user.age").unwrap();
            assert_eq!(new_age_value, Some(&Value::U64(30)));

            // Attempt to delete an existing key
            parsed_json.delete("user.contacts.phone").unwrap();
            let deleted_phone_value = parsed_json.get("user.contacts.phone").unwrap();
            assert_eq!(deleted_phone_value, None);

            // Attempt to delete a non-existent key (should return an error)
            let delete_result = parsed_json.delete("user.non_existent_key");
            assert!(delete_result.is_err());

            // Final JSON string after updates
            let final_json_string: String = parsed_json.try_into().unwrap();
            assert_eq!(
                final_json_string,
                r#"{"user":{"age":30,"contacts":{"email":"alice@example.com"},"id":1,"name":"Alicia","preferences":{"notifications":{"email":true,"sms":false}}}}"#
            );
        }

        #[test]
        fn test_null_values() {
            let json_str = r#"{
                "user": {
                    "id": 1,
                    "name": null,
                    "contacts": {
                        "email": null,
                        "phone": "123-456-7890"
                    },
                    "preferences": {
                        "notifications": {
                            "email": true,
                            "sms": false
                        }
                    }
                }
            }"#;

            let mut parsed_json = JSON::try_from(json_str).unwrap();

            // Check that the name is null
            let name_value = parsed_json.get("user.name").unwrap();
            assert_eq!(name_value, Some(&Value::Unit));

            // Update the name to a non-null value
            parsed_json
                .update("user.name", Value::String("Alice".to_string()), None)
                .unwrap();
            let updated_name = parsed_json.get("user.name").unwrap();
            assert_eq!(updated_name, Some(&Value::String("Alice".to_string())));

            // Check that the email is null
            let email_value = parsed_json.get("user.contacts.email").unwrap();
            assert_eq!(email_value, Some(&Value::Unit));

            // Update the email to a non-null value
            parsed_json
                .update(
                    "user.contacts.email",
                    Value::String("alice@example.com".to_string()),
                    None,
                )
                .unwrap();
            let updated_email = parsed_json.get("user.contacts.email").unwrap();
            assert_eq!(
                updated_email,
                Some(&Value::String("alice@example.com".to_string()))
            );
        }

        #[test]
        fn test_update_non_existent_key() {
            let json_str = r#"{
                "data": {
                    "numbers": [1, 2, 3]
                }
            }"#;

            let mut parsed_json = JSON::try_from(json_str).unwrap();

            // Attempt to update a non-existent key
            let result = parsed_json.update("data.non_existent_key", Value::U64(10), None);
            assert!(result.is_err(), "Update should fail");
            assert_eq!(
                result.err().unwrap(),
                JsonError::KeyNotFound("non_existent_key".to_string())
            );
        }
    }

    #[test]
    fn test_delete_non_existent_key() {
        let json_str = r#"{
                "data": {
                    "numbers": [1, 2, 3]
                }
            }"#;

        let mut parsed_json = JSON::try_from(json_str).unwrap();

        // Attempt to delete a non-existent key
        let result = parsed_json.delete("data.non_existent_key");
        assert!(result.is_err(), "Delete should fail");
        assert_eq!(
            result.err().unwrap(),
            JsonError::KeyNotFound("non_existent_key".to_string())
        );
    }

    #[test]
    fn test_update_nested_key_with_invalid_path() {
        let json_str = r#"{
                "data": {
                    "numbers": [1, 2, 3]
                }
            }"#;

        let mut parsed_json = JSON::try_from(json_str).unwrap();

        // Attempt to update a nested key with an invalid path
        let result = parsed_json.update("data.numbers.3", Value::U64(4), None);
        assert!(
            result.is_err(),
            "Update should fail for invalid nested path"
        );
        assert_eq!(
            result.err().unwrap(),
            JsonError::Custom("Array index out of bounds: 3".to_string())
        );
    }

    #[test]
    fn test_plain_string() {
        let json_str = r#""hello""#;
        let parsed_json = JSON::try_from(json_str).unwrap();

        // Verify the parsed value matches the original string
        assert_eq!(
            parsed_json.get("").unwrap(),
            Some(&Value::String("hello".to_string()))
        );

        // Verify string conversion
        let json_string: String = parsed_json.try_into().unwrap();
        assert_eq!(json_string, r#""hello""#);
    }
}
