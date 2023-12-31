use super::RouteError;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// A JSON structure that is formatted
/// like the following:
///
/// {
///     "key": "value"
/// }
#[derive(Debug)]
pub struct JsonObject {
    keys: HashMap<String, String>,
}

impl JsonObject {
    /// Builds a JSONObject from a string
    /// containing keys and values.
    ///
    /// # Arguments
    ///
    /// * `json` — An owned string containing the JSON.
    pub fn from_string(json: String) -> JsonObject {
        let mut keys: HashMap<String, String> = HashMap::new();

        let mut current_key = String::new();
        let mut current_value = String::new();

        let mut enumerator = json.chars();

        while let Some(c) = enumerator.next() {
            if c == '"' {
                // Get key content.
                'key: while let Some(key_content) = enumerator.next() {
                    if key_content != '"' {
                        current_key.push(key_content)
                    } else {
                        // Skip the colon (and spaces)
                        for t in enumerator.by_ref() {
                            if t == ':' { break }
                        }
                        break 'key;
                    }
                }

                // Get value of derived key
                let mut value_start = ' ';
                while value_start == ' ' {
                    value_start = enumerator.next().unwrap();
                }

                if let Some(t) = JsonType::type_for_delimiter(value_start) {
                    // Read value
                    if t == JsonType::Primitive {
                        // We need to add the first index to the value.
                        // Because the other types have delimeters (", {, [)
                        // but primitives do not.
                        current_value.push(value_start);
                        let mut in_quote = value_start == '"';
                        for inner_value in enumerator.by_ref() {
                            if current_value.chars().last().unwrap_or('_') != '\\'
                                && inner_value == '"'
                            {
                                in_quote = !in_quote;
                            }
                            if (inner_value == ',' || inner_value == '}' || inner_value == ']')
                                && !in_quote
                            {
                                break;
                            } else {
                                current_value.push(inner_value);
                            }
                        }
                    } else if t == JsonType::Object {
                        let mut delimiter_stack_count = 1;
                        current_value.push('{');
                        for inner_value in enumerator.by_ref() {
                            current_value.push(inner_value);
                            if inner_value == '{' {
                                delimiter_stack_count += 1;
                            }
                            if inner_value == '}' {
                                delimiter_stack_count -= 1;
                                if delimiter_stack_count == 0 {
                                    // Remove the trailing }
                                    // current_value.pop();
                                    break;
                                }
                            }
                        }
                    } else if t == JsonType::Array {
                        let mut delimiter_stack_count = 1;
                        current_value.push('[');
                        for inner_value in enumerator.by_ref() {
                            current_value.push(inner_value);
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
                    keys.insert(current_key, current_value);
                }
                current_key = String::new();
                current_value = String::new();
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
        T::parse(self.keys.get(key))
    }

    /// Return a key of the JSON object as a type which
    /// implements JsonRetrieve.
    ///
    /// # Arguments
    ///
    /// * `key` — The key to retrieve from.
    pub fn set<T: ToJson>(&mut self, key: &str, data: T) {
        self.keys.insert(key.to_string(), data.to_json());
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
    values: Vec<String>,
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
        let mut values: Vec<String> = Vec::new();
        let json = json[1..json.chars().count()].to_string();

        let mut enumerator = json.chars().peekable();
        let mut current_value = String::new();

        while enumerator.peek().is_some() {
            let mut value_start = ' ';
            // Trim any extra whitespace
            for value_spacing in enumerator.by_ref() {
                if value_spacing != ' ' {
                    value_start = value_spacing;
                    break;
                }
            }
            if let Some(current_type) = JsonType::type_for_delimiter(value_start) {
                // Read value
                if current_type == JsonType::Primitive {
                    // We need to add the first index to the value.
                    // Because the other types have delimeters (", {, [)
                    // but primitives do not.
                    current_value.push(value_start);
                    for inner_value in enumerator.by_ref() {
                        if inner_value != ',' {
                            current_value.push(inner_value)
                        } else {
                            break;
                        }
                    }
                } else if current_type == JsonType::Object {
                    let mut delimiter_stack_count = 1;
                    current_value.push('{');
                    for inner_value in enumerator.by_ref() {
                        current_value.push(inner_value);
                        if inner_value == '{' {
                            delimiter_stack_count += 1;
                        }
                        if inner_value == '}' {
                            delimiter_stack_count -= 1;
                            if delimiter_stack_count == 0 {
                                // Remove the trailing }
                                break;
                            }
                        }
                    }
                } else if current_type == JsonType::Array {
                    let mut delimiter_stack_count = 1;
                    current_value.push('[');
                    for inner_value in enumerator.by_ref() {
                        current_value.push(inner_value);
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
                if current_type != JsonType::Primitive {
                    for value_skipper in enumerator.by_ref() {
                        if value_skipper == ',' {
                            break;
                        }
                    }
                }
            }
            values.push(current_value);
            current_value = String::new();
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
        T::parse(self.values.get(index))
    }

    /// Converts all elements of this JSONArray
    /// to a type that implements JsonRetrieve.
    /// Drops any types that are not parsed properly.
    pub fn map<T: JsonRetrieve>(&self) -> Vec<T> {
        self.values
            .iter()
            .filter_map(|x| T::parse(Some(x)))
            .collect()
    }
}

#[derive(Debug, PartialEq)]
enum JsonType {
    Primitive,
    Object,
    Array,
}

impl JsonType {
    pub fn type_for_delimiter(dlm: char) -> Option<JsonType> {
        if dlm == '[' {
            Some(JsonType::Array)
        } else if dlm == '{' {
            Some(JsonType::Object)
        } else {
            Some(JsonType::Primitive)
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

pub trait FromJson {
    fn from_json(json: &JsonObject) -> Option<Self>
    where
        Self: Sized;
    fn from_json_validated(json: &JsonObject) -> Result<Self, RouteError>
    where
        Self: Sized;
}

impl ToJson for String {
    fn to_json(&self) -> String {
        let mut o = String::new();
        o += "\"";
        o += &self.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\t', "\\t");
        o += "\"";
        o
    }
}
impl ToJson for str {
    fn to_json(&self) -> String {
        let mut o = String::new();
        o += "\"";
        o += &self.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\t', "\\t");
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
            output.pop();
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
impl<K: ToJson, V: ToJson> ToJson for HashMap<K, V> {
    fn to_json(&self) -> String {
        let mut output = String::new();
        output += "{";
        for (k, v) in self {
            output += "\"";
            output += &k.to_json();
            output += "\":";
            output += &v.to_json();
            output += ",";
        }
        output.pop();
        output += "}";
        output
    }
}
impl ToJson for DateTime<Utc> {
    fn to_json(&self) -> String {
        format!("\"{}\"", self.to_rfc3339())
    }
}
impl ToJson for JsonObject {
    fn to_json(&self) -> String {
        let mut output = "{".to_string();
        for (k, v) in &self.keys {
            output += "\"";
            output += k;
            output += "\":";
            output += v;
            output += ",";
        }
        output.pop();
        output += "}";
        output
    }
}
impl ToJson for JsonArray {
    fn to_json(&self) -> String {
        let mut output = "[".to_string();
        for v in &self.values {
            output += v;
            output += ",";
        }
        output.pop();
        output += "]";
        output
    }
}

pub trait JsonRetrieve {
    fn parse(value: Option<&String>) -> Option<Self>
    where
        Self: Sized;
}

impl JsonRetrieve for String {
    fn parse(value: Option<&String>) -> Option<Self> {
        let mut v = value?.clone();
        v.remove(0);
        v.pop();
        v = v.replace("\\\"", "\"");
        Some(v)
    }
}
impl JsonRetrieve for i32 {
    fn parse(value: Option<&String>) -> Option<Self> {
        value?.parse().ok()
    }
}
impl JsonRetrieve for i64 {
    fn parse(value: Option<&String>) -> Option<Self> {
        value?.parse().ok()
    }
}
impl JsonRetrieve for f32 {
    fn parse(value: Option<&String>) -> Option<Self> {
        value?.parse().ok()
    }
}
impl JsonRetrieve for f64 {
    fn parse(value: Option<&String>) -> Option<Self> {
        value?.parse().ok()
    }
}
impl JsonRetrieve for bool {
    fn parse(value: Option<&String>) -> Option<Self> {
        match value?.as_ref() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        }
    }
}
impl<T: JsonRetrieve> JsonRetrieve for Vec<T> {
    fn parse(value: Option<&String>) -> Option<Self> {
        Some(JsonArray::from_string(value?.clone()).map())
    }
}
impl<T: JsonRetrieve> JsonRetrieve for Option<T> {
    fn parse(value: Option<&String>) -> Option<Self> {
        if let Some(v) = value {
            if v != "null" {
                return Some(T::parse(value));
            }
        }
        Some(None)
    }
}
impl JsonRetrieve for JsonObject {
    fn parse(value: Option<&String>) -> Option<Self> {
        Some(JsonObject::from_string(value?.clone()))
    }
}
impl JsonRetrieve for JsonArray {
    fn parse(value: Option<&String>) -> Option<Self> {
        Some(JsonArray::from_string(value?.clone()))
    }
}
impl JsonRetrieve for DateTime<Utc> {
    fn parse(value: Option<&String>) -> Option<Self> {
        Some(
            DateTime::parse_from_rfc3339(&value?.replace('\"', ""))
                .ok()?
                .with_timezone(&Utc),
        )
    }
}
impl<T: FromJson> JsonRetrieve for T {
    fn parse(value: Option<&String>) -> Option<Self> {
        Self::from_json(&JsonObject::from_string(value?.clone()))
    }
}
