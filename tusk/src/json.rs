use chrono::{DateTime, Utc};
use std::{
    collections::HashMap,
    str::{Chars, FromStr},
};
use uuid::Uuid;

struct JsonDecoder;
impl JsonDecoder {
    fn derive_key(enumerator: &mut Chars) -> String {
        let mut current_key = String::new();
        while let Some(key_content) = enumerator.next() {
            if key_content != '"' {
                current_key.push(key_content)
            } else {
                // Skip the colon (and spaces)
                for t in enumerator.by_ref() {
                    if t == ':' {
                        break;
                    }
                }
                break;
            }
        }
        current_key
    }

    fn derive_value<T: Iterator<Item = char>>(enumerator: &mut T) -> String {
        let mut value_start = ' ';
        while value_start == ' ' || value_start == ',' {
            if let Some(v) = enumerator.next() {
                value_start = v;
            } else {
                return String::new();
            }
        }
        let exec = match value_start {
            '\"' => JsonTypeString::extract,
            '{' => JsonTypeObject::extract,
            '[' => JsonTypeArray::extract,
            _ => JsonTypePrimitive::extract,
        };
        exec(enumerator, value_start.to_string())
    }
}

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
    /// Creates an empty JSON object.
    /// This is useful for building a JSON
    /// object from scratch.
    pub fn empty() -> JsonObject {
        JsonObject {
            keys: HashMap::new(),
        }
    }

    /// Builds a JSONObject from a string
    /// containing keys and values.
    ///
    /// # Arguments
    ///
    /// * `json` — An owned string containing the JSON.
    pub fn from_string(json: &str) -> JsonObject {
        let mut keys: HashMap<String, String> = HashMap::new();
        let mut enumerator = json.chars();
        while let Some(c) = enumerator.next() {
            if c == '"' {
                let (k, v) = (
                    JsonDecoder::derive_key(&mut enumerator),
                    JsonDecoder::derive_value(&mut enumerator),
                );
                keys.insert(k, v);
            }
        }
        // dbg!(&keys);
        JsonObject { keys }
    }

    /// Return a key of the JSON object as a type which
    /// implements JsonRetrieve.
    ///
    /// # Arguments
    ///
    /// * `key` — The key to retrieve from.
    pub fn get<T: JsonRetrieve>(&self, key: &str) -> Result<T, JsonParseError> {
        T::parse(key.to_string(), self.keys.get(key))
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
}
impl Default for JsonObject {
    fn default() -> Self {
        JsonObject::empty()
    }
}

#[derive(Debug)]
pub struct JsonArray {
    values: Vec<String>,
}
impl JsonArray {
    /// Creates an empty JSON array.
    /// This is useful for building a JSON
    /// array from scratch.
    pub fn empty() -> JsonArray {
        JsonArray { values: Vec::new() }
    }

    /// Builds a JSONArray from a string
    /// containing children that implement
    /// `JsonRetreive`
    ///
    /// # Arguments
    ///
    /// * `json` — An owned string containing the JSON.
    pub fn from_string(json: &str) -> JsonArray {
        let mut values: Vec<String> = Vec::new();
        let mut enumerator = json.chars().peekable();

        while let Some(v) = enumerator.peek() {
            if v.is_whitespace() || *v == '[' {
                enumerator.next();
            } else {
                break;
            }
        }
        while enumerator.peek().is_some() {
            if *enumerator.peek().unwrap_or(&'_') == ']' {
                _ = enumerator.next();
                continue;
            }
            let v = JsonDecoder::derive_value(&mut enumerator);
            values.push(v);
        }
        JsonArray { values }
    }

    /// Gets the object at the index as a type
    /// that implements JsonRetrieve.
    ///
    /// # Arguments
    ///
    /// * `index` — The index to retrieve from.
    pub fn get<T: JsonRetrieve>(&self, index: usize) -> Result<T, JsonParseError> {
        T::parse(index.to_string(), self.values.get(index))
    }

    /// Converts all elements of this JSONArray
    /// to a type that implements JsonRetrieve.
    /// Progagates errors if any child keys are invalid.
    pub fn map<T: JsonRetrieve>(&self) -> Result<Vec<T>, JsonParseError> {
        if self.values.is_empty() {
            return Ok(Vec::new());
        }
        let mut build = Vec::new();
        for i in 0..self.values.len() {
            let value = self.values.get(i);
            build.push(T::parse(i.to_string(), value)?);
        }
        Ok(build)
    }

    /// Converts all elements of this JSONArray
    /// to a type that implements JsonRetrieve.
    /// Silently drops any invalid children.
    pub fn map_drop<T: JsonRetrieve>(&self) -> Vec<T> {
        if self.values.is_empty() {
            return Vec::new();
        }
        let mut build = Vec::new();
        for i in 0..self.values.len() {
            let value = &self.values[i];
            if let Ok(val) = T::parse(i.to_string(), Some(value)) {
                build.push(val);
            }
        }
        build
    }
}
impl Default for JsonArray {
    fn default() -> Self {
        JsonArray::empty()
    }
}

trait JsonType {
    fn extract<T: Iterator<Item = char>>(stream: &mut T, intl_value: String) -> String;
}

struct JsonTypePrimitive;
impl JsonType for JsonTypePrimitive {
    fn extract<T: Iterator<Item = char>>(stream: &mut T, intl_value: String) -> String {
        let mut buf = intl_value;
        for n in stream.by_ref() {
            if n.is_whitespace() || n == ',' || n == '}' || n == ']' {
                break;
            }
            buf.push(n);
        }
        buf
    }
}

struct JsonTypeString;
impl JsonType for JsonTypeString {
    fn extract<T: Iterator<Item = char>>(stream: &mut T, intl_value: String) -> String {
        let mut buf = intl_value;
        let mut prev = '_';
        let mut prev_prev = '_';
        for n in stream.by_ref() {
            buf.push(n);
            if n == '"' && (prev != '\\' || prev_prev == '\\') {
                break;
            }
            prev_prev = prev;
            prev = n;
        }
        buf
    }
}

struct JsonTypeObject;
impl JsonType for JsonTypeObject {
    fn extract<T: Iterator<Item = char>>(stream: &mut T, intl_value: String) -> String {
        let mut buf = intl_value;
        let mut sep_stack = 1;

        let mut prev = '_';
        let mut prev_prev = '_';
        let mut is_in_string = false;

        for n in stream.by_ref() {
            if n == '"' && (prev != '\\' || prev_prev == '\\') {
                is_in_string = !is_in_string;
            }
            if !is_in_string && n.is_whitespace() {
                continue;
            }
            buf.push(n);
            if n == '{' {
                sep_stack += 1
            } else if n == '}' {
                sep_stack -= 1
            }
            if sep_stack == 0 {
                break;
            }
            prev_prev = prev;
            prev = n;
        }
        buf
    }
}

struct JsonTypeArray;
impl JsonType for JsonTypeArray {
    fn extract<T: Iterator<Item = char>>(stream: &mut T, intl_value: String) -> String {
        let mut buf = intl_value;
        let mut sep_stack = 1;

        let mut prev = '_';
        let mut prev_prev = '_';
        let mut is_in_string = false;

        for n in stream.by_ref() {
            if n == '"' && (prev != '\\' || prev_prev == '\\') {
                is_in_string = !is_in_string;
            }
            if !is_in_string && n.is_whitespace() {
                continue;
            }
            buf.push(n);
            if n == '[' {
                sep_stack += 1
            } else if n == ']' {
                sep_stack -= 1
            }
            if sep_stack == 0 {
                break;
            }
            prev_prev = prev;
            prev = n;
        }
        buf
    }
}

#[derive(Debug)]
pub enum JsonParseError {
    NotFound(String),
    InvalidType(String, &'static str),
}

/// ToJson is a trait that allows any conforming
/// structs to convert to a JSON format.
///
/// A default implemenation is most easily
/// obtained by deriving this trait.
pub trait ToJson {
    /// ToJson creates a JSON string from
    /// anything which implements it
    fn to_json(&self) -> String;
}

/// FromJs is a trait that allows any conforming
/// structs to be converted from a JSON format.
///
/// A default implemenation is most easily
/// obtained by deriving this trait.
pub trait FromJson {
    fn from_json(json: &JsonObject) -> Result<Self, JsonParseError>
    where
        Self: Sized;
}

impl ToJson for String {
    fn to_json(&self) -> String {
        let mut o = String::new();
        o += "\"";
        o += &self
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\t', "\\t");
        o += "\"";
        o
    }
}
impl ToJson for str {
    fn to_json(&self) -> String {
        let mut o = String::new();
        o += "\"";
        o += &self
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\t', "\\t");
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
        if output != "{" {
            output.pop();
        }
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
    fn parse(key: String, value: Option<&String>) -> Result<Self, JsonParseError>
    where
        Self: Sized;
}

impl JsonRetrieve for String {
    fn parse(key: String, value: Option<&String>) -> Result<Self, JsonParseError> {
        let val = value.ok_or(JsonParseError::NotFound(key.clone()))?;
        if val.len() < 2 {
            return Err(JsonParseError::InvalidType(key, "String"));
        }
        Ok(val[1..val.len() - 1].replace("\\\"", "\"").to_string())
    }
}
impl JsonRetrieve for i32 {
    fn parse(key: String, value: Option<&String>) -> Result<Self, JsonParseError> {
        if let Some(v) = value {
            Ok(v.parse()
                .map_err(|_| JsonParseError::InvalidType(key, "i32"))?)
        } else {
            Err(JsonParseError::NotFound(key))
        }
    }
}
impl JsonRetrieve for i64 {
    fn parse(key: String, value: Option<&String>) -> Result<Self, JsonParseError> {
        if let Some(v) = value {
            Ok(v.parse()
                .map_err(|_| JsonParseError::InvalidType(key, "i64"))?)
        } else {
            Err(JsonParseError::NotFound(key))
        }
    }
}
impl JsonRetrieve for f32 {
    fn parse(key: String, value: Option<&String>) -> Result<Self, JsonParseError> {
        if let Some(v) = value {
            Ok(v.parse()
                .map_err(|_| JsonParseError::InvalidType(key, "f32"))?)
        } else {
            Err(JsonParseError::NotFound(key))
        }
    }
}
impl JsonRetrieve for f64 {
    fn parse(key: String, value: Option<&String>) -> Result<Self, JsonParseError> {
        if let Some(v) = value {
            Ok(v.parse()
                .map_err(|_| JsonParseError::InvalidType(key, "f64"))?)
        } else {
            Err(JsonParseError::NotFound(key))
        }
    }
}
impl JsonRetrieve for bool {
    fn parse(key: String, value: Option<&String>) -> Result<Self, JsonParseError> {
        if let Some(v) = value {
            match v.as_ref() {
                "true" => Ok(true),
                "false" => Ok(false),
                _ => Err(JsonParseError::InvalidType(key, "bool")),
            }
        } else {
            Err(JsonParseError::NotFound(key))
        }
    }
}
impl<T: JsonRetrieve> JsonRetrieve for Vec<T> {
    fn parse(key: String, value: Option<&String>) -> Result<Self, JsonParseError> {
        JsonArray::from_string(value.ok_or(JsonParseError::NotFound(key))?).map()
    }
}
impl<T: JsonRetrieve> JsonRetrieve for Option<T> {
    fn parse(key: String, value: Option<&String>) -> Result<Self, JsonParseError> {
        if let Some(v) = value {
            if v != "null" {
                return Ok(Some(T::parse(key, value)?));
            }
        }
        Ok(None)
    }
}
impl JsonRetrieve for JsonObject {
    fn parse(key: String, value: Option<&String>) -> Result<Self, JsonParseError> {
        Ok(JsonObject::from_string(
            value.ok_or(JsonParseError::NotFound(key))?,
        ))
    }
}
impl JsonRetrieve for JsonArray {
    fn parse(key: String, value: Option<&String>) -> Result<Self, JsonParseError> {
        Ok(JsonArray::from_string(
            value.ok_or(JsonParseError::NotFound(key))?,
        ))
    }
}
impl<T: FromJson> JsonRetrieve for T {
    fn parse(key: String, value: Option<&String>) -> Result<Self, JsonParseError> {
        Self::from_json(&JsonObject::from_string(
            value.ok_or(JsonParseError::NotFound(key))?,
        ))
    }
}

impl JsonRetrieve for Uuid {
    fn parse(key: String, value: Option<&String>) -> Result<Self, JsonParseError> {
        let val = value.ok_or_else(|| JsonParseError::NotFound(key.clone()))?;
        let string_val = val[1..val.len() - 1].replace("\\\"", "\"").to_string();
        Uuid::from_str(&string_val).map_err(|_| JsonParseError::InvalidType(key, "UUID"))
    }
}
impl ToJson for Uuid {
    fn to_json(&self) -> String {
        self.to_string()
    }
}

impl JsonRetrieve for DateTime<Utc> {
    fn parse(key: String, value: Option<&String>) -> Result<Self, JsonParseError> {
        if let Some(v) = value {
            Ok(DateTime::parse_from_rfc3339(&v.replace('\"', ""))
                .map_err(|_| JsonParseError::InvalidType(key, "RFC3339 Date"))?
                .with_timezone(&Utc))
        } else {
            Err(JsonParseError::NotFound(key))
        }
    }
}

impl ToJson for DateTime<Utc> {
    fn to_json(&self) -> String {
        format!("\"{}\"", self.to_rfc3339())
    }
}
