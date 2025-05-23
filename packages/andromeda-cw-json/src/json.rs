use std::collections::BTreeMap;

use cosmwasm_std::{from_json, to_json_string, Binary};
use serde::{Deserialize, Serialize};
use serde_cw_value::{to_value, Value};

use crate::error::JsonError;
use std::fmt;

// Wrapper against Serde Value which has different impl defined against it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JSON(Value);

impl TryFrom<&str> for JSON {
    type Error = JsonError;

    // Convert a JSON string to a JSON struct
    fn try_from(json_str: &str) -> Result<Self, Self::Error> {
        let parsed_json: Value = from_json(json_str)?;
        Ok(Self(parsed_json))
    }
}

impl TryFrom<Binary> for JSON {
    type Error = JsonError;

    // Convert a JSON binary to a JSON struct
    fn try_from(binary: Binary) -> Result<Self, Self::Error> {
        let parsed_json: Value = from_json(binary)?;
        Ok(Self(parsed_json))
    }
}

impl TryFrom<Value> for JSON {
    type Error = JsonError;

    // Convert a Serde Value to a JSON struct
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let v = Self(value);
        // Parse and store again to properly deserialize
        let parsed: String = v.try_into()?;
        Self::try_from(parsed.as_str())
    }
}

impl TryFrom<JSON> for String {
    type Error = JsonError;

    // Convert a JSON struct to a JSON string
    fn try_from(json: JSON) -> Result<Self, Self::Error> {
        Ok(to_json_string(&json.0)?)
    }
}

impl JSON {
    // Try to convert any type to a JSON struct
    #[inline(never)]
    pub fn from_any<T: serde::ser::Serialize>(value: T) -> Result<Self, JsonError> {
        let value = to_value(value)?;
        Self::try_from(value)
    }

    // Try to convert a JSON struct to any type
    #[inline(always)]
    pub fn to_any<T: serde::de::DeserializeOwned>(self) -> Result<T, JsonError> {
        Ok(self.0.deserialize_into()?)
    }

    // Try to get a nested value from a JSON struct using dot notation
    #[inline(always)]
    pub fn get<'a>(&'a self, key: &'a str) -> Result<Option<&'a Value>, JsonError> {
        if key.is_empty() {
            return Ok(Some(&self.0));
        }
        Self::get_nested(&self.0, key.split('.'))
    }

    /**
     * Private helper function to get a nested value from a JSON struct using dot notation.
     * Example, get nested values like data.nested.0.name
     */
    fn get_nested<'a, I>(json: &'a Value, mut keys: I) -> Result<Option<&'a Value>, JsonError>
    where
        'a: 'a,
        I: Iterator<Item = &'a str>,
    {
        let result = match json {
            Value::Map(map) => {
                // Get the next key
                if let Some(key) = keys.next() {
                    // If the key is present in the map, get the nested value, else return None
                    match map.get(&to_value(key)?) {
                        Some(next_json) => Self::get_nested(next_json, keys)?,
                        None => None,
                    }
                } else {
                    // If there are no more keys, return the current value
                    Some(json)
                }
            }
            Value::Seq(list) => {
                // Get the next key
                if let Some(index) = keys.next() {
                    // If the key is a valid index, get the nested value, else return None
                    if let Ok(idx) = index.parse::<usize>() {
                        // If the index is within the bounds of the list, get the nested value, else return None
                        match list.get(idx) {
                            Some(next_json) => Self::get_nested(next_json, keys)?,
                            None => None,
                        }
                    } else {
                        // If the key is not a valid index, return None
                        None
                    }
                } else {
                    // If there are no more keys, return the current value
                    Some(json)
                }
            }
            _ => match keys.next() {
                // If there are more keys, but the current value is not a complex struct, return None
                Some(_) => None,
                // If there are no more keys, return the current value
                None => Some(json),
            },
        };
        Ok(result)
    }

    /**
     * Update a nested value in the JSON struct using dot notation.
     * Example, update nested values like data.nested.0.name
     */
    #[inline(always)]
    pub fn update(
        &mut self,
        key: &str,
        value: Value,
        upsert: Option<bool>,
    ) -> Result<&Self, JsonError> {
        // If key is empty, then return root value
        if key.is_empty() {
            self.0 = value;
            return Ok(self);
        }
        let keys: Vec<&str> = key.split('.').collect();
        // Perform the update
        Self::update_nested(&mut self.0, &keys, value, upsert)?;
        Ok(self)
    }

    /**
     * Private helper function to update a nested value in the JSON struct using dot notation.
     * Example, update nested values like data.nested.0.name
     */
    fn update_nested(
        json: &mut Value,
        keys: &[&str],
        value: Value,
        upsert: Option<bool>,
    ) -> Result<(), JsonError> {
        if let Some(current_key) = keys.first() {
            match json {
                Value::Map(ref mut map) => {
                    // Early return if the key does not exist and upsert is false or empty
                    if !upsert.unwrap_or(false)
                        && !map.contains_key(&Value::String(current_key.to_string()))
                    {
                        return Err(JsonError::KeyNotFound(current_key.to_string()));
                    }

                    if keys.len() == 1 {
                        // Insert the value into the map for the current key
                        map.insert(Value::String(current_key.to_string()), value);
                    } else {
                        let next_json = map
                            .entry(Value::String(current_key.to_string()))
                            .or_insert(Value::Map(BTreeMap::new()));

                        Self::update_nested(next_json, &keys[1..], value, upsert)?;
                    };
                }
                Value::Seq(ref mut list) => {
                    if let Ok(index) = current_key.parse::<usize>() {
                        if keys.len() == 1 {
                            if index < list.len() {
                                list[index] = value;
                            } else {
                                return Err(JsonError::Custom(format!(
                                    "Array index out of bounds: {}",
                                    index
                                )));
                            }
                        } else if index < list.len() {
                            Self::update_nested(&mut list[index], &keys[1..], value, upsert)?;
                        } else {
                            return Err(JsonError::Custom(format!(
                                "Array index out of bounds: {}",
                                index
                            )));
                        };
                    } else {
                        return Err(JsonError::Custom(format!(
                            "Invalid array index: {}",
                            current_key
                        )));
                    }
                }
                _ => {
                    // Handle other cases here if needed
                    return Err(JsonError::Custom(format!(
                        "Invalid JSON structure at key: {}",
                        current_key
                    )));
                }
            }
        }
        Ok(())
    }

    /**
     * Delete a nested value in the JSON struct using dot notation.
     * Example, delete nested values like data.nested.0.name
     */
    pub fn delete(&mut self, key: &str) -> Result<(), JsonError> {
        let keys: Vec<&str> = key.split('.').collect();
        Self::delete_nested(&mut self.0, &keys)
    }

    /**
     * Private helper function to delete a nested value in the JSON struct using dot notation.
     * Example, delete nested values like data.nested.0.name
     */
    fn delete_nested(json: &mut Value, keys: &[&str]) -> Result<(), JsonError> {
        if keys.is_empty() {
            return Err(JsonError::Custom("No key provided".to_string()));
        }

        let current_key = keys[0];
        match json {
            Value::Map(ref mut map) => {
                // Early return if the key does not exist
                if !map.contains_key(&Value::String(current_key.to_string())) {
                    return Err(JsonError::KeyNotFound(current_key.to_string()));
                }

                if keys.len() == 1 {
                    // Remove the key from the map if its the last key
                    map.remove(&Value::String(current_key.to_string()));
                } else {
                    // Get the nested value
                    let next_json = map
                        .get_mut(&Value::String(current_key.to_string()))
                        .ok_or_else(|| JsonError::KeyNotFound(current_key.to_string()))?;
                    // Recursively delete the nested value
                    Self::delete_nested(next_json, &keys[1..])?;
                }
            }
            Value::Seq(ref mut list) => {
                if let Ok(index) = current_key.parse::<usize>() {
                    if keys.len() == 1 {
                        // Remove the key from the list if its the last key
                        if index < list.len() {
                            list.remove(index);
                        } else {
                            return Err(JsonError::Custom(format!(
                                "Array index out of bounds: {}",
                                index
                            )));
                        }
                    } else if index < list.len() {
                        // Recursively delete the nested value
                        Self::delete_nested(&mut list[index], &keys[1..])?;
                    } else {
                        // Return an error if the index is out of bounds
                        return Err(JsonError::Custom(format!(
                            "Array index out of bounds: {}",
                            index
                        )));
                    }
                } else {
                    // Return an error if the key is not a valid index
                    return Err(JsonError::Custom(format!(
                        "Invalid array index: {}",
                        current_key
                    )));
                }
            }
            _ => {
                // Return an error if the value is not a complex struct
                return Err(JsonError::Custom(format!(
                    "Invalid JSON structure at key: {}",
                    current_key
                )));
            }
        }
        Ok(())
    }
}

impl fmt::Display for JSON {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let json_string = to_json_string(&self.0).map_err(|_| fmt::Error)?;
        write!(f, "{}", json_string)
    }
}
