use std::collections::HashMap;

use super::RouteError;

/// A JSON structure that is formatted
/// like the following:
///
/// {
///     "key": "value"
/// }
#[derive(Debug)]
pub struct JsonObject {
    keys: HashMap<String, JsonChild>,
}

impl JsonObject {
    /// Builds a JSONObject from a string
    /// containing keys and values.
    ///
    /// # Arguments
    ///
    /// * `json` — An owned string containing the JSON.
    pub fn from_string(json: String) -> JsonObject {
        let json = json[0..json.chars().count() - 1].to_string();
        let mut keys: HashMap<String, JsonChild> = HashMap::new();

        let mut current_key = String::new();
        let mut current_subkey = JsonChild::new();

        let mut enumerator = json.chars();

        while let Some(c) = enumerator.next() {
            if c == '"' {
                // Get key content.
                while let Some(key_content) = enumerator.next() {
                    if key_content != '"' {
                        current_key.push(key_content)
                    } else {
                        // Skip the colon
                        enumerator.next();
                        break;
                    }
                }

                // Get value of derived key
                let mut value_start = ' ';
                for value_spacing in enumerator.by_ref() {
                    if value_spacing != ' ' {
                        value_start = value_spacing;
                        break;
                    }
                }
                current_subkey.content_type = JsonType::type_for_delimiter(value_start);

                // Read value
                if current_subkey.content_type == JsonType::String {
                    let mut last_value = '0';
                    for inner_value in enumerator.by_ref() {
                        if inner_value != '"' {
                            current_subkey.contents.push(inner_value)
                        } else if last_value == '\\' && (inner_value == '"' || inner_value == '\\')
                        {
                            current_subkey.contents.pop();
                            current_subkey.contents.push(inner_value)
                        } else {
                            break;
                        }
                        last_value = inner_value;
                    }
                } else if current_subkey.content_type.is_primitive() {
                    // We need to add the first index to the value.
                    // Because the other types have delimeters (", {, [)
                    // but primitives do not.
                    current_subkey.contents.push(value_start);
                    for inner_value in enumerator.by_ref() {
                        if inner_value != ',' {
                            current_subkey.contents.push(inner_value)
                        } else {
                            break;
                        }
                    }
                } else if current_subkey.content_type == JsonType::Object {
                    let mut delimiter_stack_count = 1;
                    current_subkey.contents.push('{');
                    for inner_value in enumerator.by_ref() {
                        current_subkey.contents.push(inner_value);
                        if inner_value == '{' {
                            delimiter_stack_count += 1;
                        }
                        if inner_value == '}' {
                            delimiter_stack_count -= 1;
                            if delimiter_stack_count == 0 {
                                // Remove the trailing }
                                current_subkey.contents.pop();
                                break;
                            }
                        }
                    }
                } else if current_subkey.content_type == JsonType::Array {
                    let mut delimiter_stack_count = 1;
                    current_subkey.contents.push('[');
                    for inner_value in enumerator.by_ref() {
                        current_subkey.contents.push(inner_value);
                        if inner_value == '[' {
                            delimiter_stack_count += 1;
                        }
                        if inner_value == ']' {
                            delimiter_stack_count -= 1;
                            if delimiter_stack_count == 0 {
                                break;
                            }
                        }
                    }
                }

                keys.insert(current_key, current_subkey);

                current_key = String::new();
                current_subkey = JsonChild::new();
            }
        }
        JsonObject { keys }
    }

    /// Return a key of the JSON object as a type which
    /// implements JsonRetrieve.
    ///
    /// # Arguments
    ///
    /// * `key` — The key to retrieve from.
    pub fn get<T: JsonRetrieve>(&self, key: &str) -> Option<T> {
        let child = self.keys.get(key)?;
        T::parse(child)
    }

    /// A convienience function that calls the `get` method
    /// and validates that it exists and was parsed correctly
    /// or returns a `RouteError::bad_request` (400 error) with
    /// the message provided.
    ///
    /// # Arguments
    ///
    /// * `key` — The key to retrieve from.
    /// * `err` — The error message to return if key is None.
    pub fn validate_get<T: JsonRetrieve>(&self, key: &str, err: &str) -> Result<T, RouteError> {
        self.get(key).ok_or(RouteError::bad_request(err))
    }
}
#[derive(Debug)]
pub struct JsonArray {
    values: Vec<JsonChild>,
}
impl JsonArray {
    /// Builds a JSONArray from a string
    /// containing children that implement
    /// `JsonRetreive`
    ///
    /// # Arguments
    ///
    /// * `json` — An owned string containing the JSON.
    pub fn from_string(json: String) -> JsonArray {
        let mut values: Vec<JsonChild> = Vec::new();
        let json = json[1..json.chars().count() - 1].to_string();

        let mut enumerator = json.chars().peekable();
        let mut current_value = JsonChild::new();

        while enumerator.peek().is_some() {
            let mut value_start = ' ';
            // Trim any extra whitespace
            for value_spacing in enumerator.by_ref() {
                if value_spacing != ' ' {
                    value_start = value_spacing;
                    break;
                }
            }
            current_value.content_type = JsonType::type_for_delimiter(value_start);
            // Read value
            if current_value.content_type == JsonType::String {
                let mut last_value = '0';
                for inner_value in enumerator.by_ref() {
                    if inner_value != '"' {
                        current_value.contents.push(inner_value)
                    } else if last_value == '\\' && (inner_value == '"' || inner_value == '\\') {
                        current_value.contents.pop();
                        current_value.contents.push(inner_value)
                    } else {
                        break;
                    }
                    last_value = inner_value;
                }
            } else if current_value.content_type.is_primitive() {
                // We need to add the first index to the value.
                // Because the other types have delimeters (", {, [)
                // but primitives do not.
                current_value.contents.push(value_start);
                for inner_value in enumerator.by_ref() {
                    if inner_value != ',' {
                        current_value.contents.push(inner_value)
                    } else {
                        break;
                    }
                }
            } else if current_value.content_type == JsonType::Object {
                let mut delimiter_stack_count = 1;
                current_value.contents.push('{');
                for inner_value in enumerator.by_ref() {
                    current_value.contents.push(inner_value);
                    if inner_value == '{' {
                        delimiter_stack_count += 1;
                    }
                    if inner_value == '}' {
                        delimiter_stack_count -= 1;
                        if delimiter_stack_count == 0 {
                            // Remove the trailing }
                            current_value.contents.pop();
                            break;
                        }
                    }
                }
            } else if current_value.content_type == JsonType::Array {
                let mut delimiter_stack_count = 1;
                current_value.contents.push('[');
                for inner_value in enumerator.by_ref() {
                    current_value.contents.push(inner_value);
                    if inner_value == '[' {
                        delimiter_stack_count += 1;
                    }
                    if inner_value == ']' {
                        delimiter_stack_count -= 1;
                        if delimiter_stack_count == 0 {
                            break;
                        }
                    }
                }
            }
            // Because the primitive types do not have a ending delimiter
            // and read straight to the comma, we do not search until a comma
            // if our type is primitive.
            if !current_value.content_type.is_primitive() {
                for value_skipper in enumerator.by_ref() {
                    if value_skipper == ',' {
                        break;
                    }
                }
            }
            values.push(current_value);
            current_value = JsonChild::new();
        }

        JsonArray { values }
    }

    /// Gets the object at the index as a type
    /// that implements JsonRetrieve.
    ///
    /// # Arguments
    ///
    /// * `index` — The index to retrieve from.
    pub fn get<T: JsonRetrieve>(&self, index: usize) -> Option<T> {
        return T::parse(self.values.get(index)?);
    }

    /// Converts all elements of this JSONArray
    /// to a type that implements JsonRetrieve.
    /// Drops any types that are not parsed properly.
    pub fn map<T: JsonRetrieve>(&self) -> Vec<T> {
        self.values.iter().map(|x| T::parse(x)).filter_map(|x| x).collect()
    }
}

/// A type used to internally represet JSON.
/// Only used publicly to implement `JsonRetrieve`.
#[derive(Debug)]
pub struct JsonChild {
    content_type: JsonType,
    contents: String,
}
impl JsonChild {
    fn new() -> JsonChild {
        JsonChild {
            content_type: JsonType::String,
            contents: String::new(),
        }
    }
}

#[derive(Debug, PartialEq)]
enum JsonType {
    String,
    Number,
    Boolean,
    Object,
    Array,
}

impl JsonType {
    pub fn is_primitive(&self) -> bool {
        *self == JsonType::Number || *self == JsonType::Boolean
    }
    pub fn type_for_delimiter(dlm: char) -> JsonType {
        if dlm.is_ascii_digit() {
            JsonType::Number
        } else if dlm == '"' {
            JsonType::String
        } else if dlm == 'f' || dlm == 't' {
            JsonType::Boolean
        } else if dlm == '[' {
            JsonType::Array
        } else if dlm == '{' {
            JsonType::Object
        } else {
            panic!("Unexpected value {}", dlm);
        }
    }
}

/// ToJson is a trait that allows any conforming
/// structs to convert to a JSON format.
pub trait ToJson {
    /// ToJson creates a JSON string from
    /// anything which implements it
    fn to_json(&self) -> String;
}

impl ToJson for String {
    fn to_json(&self) -> String {
        let mut o = String::new();
        o += "\"";
        o += self;
        o += "\"";
        o
    }
}
impl ToJson for str {
    fn to_json(&self) -> String {
        let mut o = String::new();
        o += "\"";
        o += self;
        o += "\"";
        o
    }
}
impl ToJson for i32 {
    fn to_json(&self) -> String {
        self.to_string()
    }
}
impl ToJson for i64 {
    fn to_json(&self) -> String {
        self.to_string()
    }
}
impl ToJson for u32 {
    fn to_json(&self) -> String {
        self.to_string()
    }
}
impl ToJson for u64 {
    fn to_json(&self) -> String {
        self.to_string()
    }
}
impl ToJson for f32 {
    fn to_json(&self) -> String {
        self.to_string()
    }
}
impl ToJson for f64 {
    fn to_json(&self) -> String {
        self.to_string()
    }
}
impl ToJson for bool {
    fn to_json(&self) -> String {
        if *self {
            "true".to_string()
        } else {
            "false".to_string()
        }
    }
}
impl<T: ToJson> ToJson for Vec<T> {
    fn to_json(&self) -> String {
        let mut output = String::new();
        output += "[";
        for i in self.iter() {
            output += &i.to_json();
            output += ",";
        }
        if !self.is_empty() {
            output = output[0..output.chars().count() - 1].to_string();
        }
        output += "]";
        output
    }
}
impl<T: ToJson> ToJson for Option<T> {
    fn to_json(&self) -> String {
        match self {
            Some(x) => x.to_json(),
            None => "null".to_string(),
        }
    }
}
impl<K: AsRef<str>, V: ToJson> ToJson for HashMap<K, V> {
    fn to_json(&self) -> String {
        let mut output = String::new();
        output += "{";
        for (k, v) in self {
            output += "\"";
            output += k.as_ref();
            output += "\":";
            output += &v.to_json();
            output += ",";
        }
        output = output[0..output.len() - 1].to_string();
        output += "}";
        output
    }
}

pub trait JsonRetrieve {
    fn parse(value: &JsonChild) -> Option<Self>
    where
        Self: Sized;
}

impl JsonRetrieve for String {
    fn parse(value: &JsonChild) -> Option<Self> {
        return Some(value.contents.clone());
    }
}
impl JsonRetrieve for i32 {
    fn parse(value: &JsonChild) -> Option<Self> {
        return value.contents.parse().ok();
    }
}
impl JsonRetrieve for i64 {
    fn parse(value: &JsonChild) -> Option<Self> {
        return value.contents.parse().ok();
    }
}
impl JsonRetrieve for f32 {
    fn parse(value: &JsonChild) -> Option<Self> {
        return value.contents.parse().ok();
    }
}
impl JsonRetrieve for f64 {
    fn parse(value: &JsonChild) -> Option<Self> {
        return value.contents.parse().ok();
    }
}
impl JsonRetrieve for JsonObject {
    fn parse(value: &JsonChild) -> Option<Self> {
        if value.content_type == JsonType::Object {
            return Some(JsonObject::from_string(value.contents.clone()));
        }
        None
    }
}
impl JsonRetrieve for JsonArray {
    fn parse(value: &JsonChild) -> Option<Self> {
        if value.content_type == JsonType::Array {
            return Some(JsonArray::from_string(value.contents.clone()));
        }
        None
    }
}
