use std::collections::HashMap;

use super::RouteError;

#[derive(Debug)]
pub struct JsonObject {
    keys: HashMap<String, JsonChild>,
}

impl JsonObject {
    pub fn from_string(json: String) -> JsonObject {
        let json = json[0..json.chars().count() - 1].to_string();
        let mut keys: HashMap<String, JsonChild> = HashMap::new();

        let mut current_key = String::new();
        let mut current_subkey = JsonChild::new();

        let mut enumerator = json.chars().into_iter();

        loop {
            if let Some(c) = enumerator.next() {
                if c == '"' {
                    // Get key content.
                    loop {
                        if let Some(key_content) = enumerator.next() {
                            if key_content != '"' {
                                current_key.push(key_content)
                            } else {
                                // Skip the colon
                                enumerator.next();
                                break;
                            }
                        } else {
                            break;
                        }
                    }

                    // Get value of derived key
                    let mut value_start = ' ';
                    loop {
                        if let Some(value_spacing) = enumerator.next() {
                            if value_spacing != ' ' {
                                value_start = value_spacing;
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                    current_subkey.content_type = JsonType::type_for_delimiter(value_start);

                    // Read value
                    if current_subkey.content_type == JsonType::String {
                        let mut last_value = '0';
                        loop {
                            if let Some(inner_value) = enumerator.next() {
                                if inner_value != '"' {
                                    current_subkey.contents.push(inner_value)
                                } else if last_value == '\\'
                                    && (inner_value == '"' || inner_value == '\\')
                                {
                                    current_subkey.contents.pop();
                                    current_subkey.contents.push(inner_value)
                                } else {
                                    break;
                                }
                                last_value = inner_value;
                            } else {
                                break;
                            }
                        }
                    } else if current_subkey.content_type.is_primitive() {
                        // We need to add the first index to the value.
                        // Because the other types have delimeters (", {, [)
                        // but primitives do not.
                        current_subkey.contents.push(value_start);
                        loop {
                            if let Some(inner_value) = enumerator.next() {
                                if inner_value != ',' {
                                    current_subkey.contents.push(inner_value)
                                } else {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                    } else if current_subkey.content_type == JsonType::Object {
                        let mut delimiter_stack_count = 1;
                        current_subkey.contents.push('{');
                        loop {
                            if let Some(inner_value) = enumerator.next() {
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
                            } else {
                                break;
                            }
                        }
                    } else if current_subkey.content_type == JsonType::Array {
                        let mut delimiter_stack_count = 1;
                        current_subkey.contents.push('[');
                        loop {
                            if let Some(inner_value) = enumerator.next() {
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
                            } else {
                                break;
                            }
                        }
                    }

                    keys.insert(current_key, current_subkey);

                    current_key = String::new();
                    current_subkey = JsonChild::new();
                }
            } else {
                break;
            }
        }
        return JsonObject { keys };
    }

    pub fn string(&self, key: &str) -> Option<String> {
        let child = self.keys.get(key)?;
        Some(child.contents.clone())
    }

    pub fn validate_string(&self, key: &str, err: &str) -> Result<String, RouteError> {
        let child = self.keys.get(key).ok_or(RouteError::bad_request(err))?;
        Ok(child.contents.clone())
    }

    pub fn i32(&self, key: &str) -> Option<i32> {
        let child = self.keys.get(key)?;
        child.contents.parse().ok()
    }

    pub fn validate_i32(&self, key: &str, err: &str) -> Result<i32, RouteError> {
        let child = self.keys.get(key).ok_or(RouteError::bad_request(err))?;
        child.contents.parse().or(Err(RouteError::bad_request(err)))
    }

    pub fn i64(&self, key: &str) -> Option<i64> {
        let child = self.keys.get(key)?;
        child.contents.parse().ok()
    }

    pub fn validate_i64(&self, key: &str, err: &str) -> Result<i64, RouteError> {
        let child = self.keys.get(key).ok_or(RouteError::bad_request(err))?;
        child.contents.parse().or(Err(RouteError::bad_request(err)))
    }

    pub fn f32(&self, key: &str) -> Option<f32> {
        let child = self.keys.get(key)?;
        child.contents.parse().ok()
    }

    pub fn validate_f32(&self, key: &str, err: &str) -> Result<f32, RouteError> {
        let child = self.keys.get(key).ok_or(RouteError::bad_request(err))?;
        child.contents.parse().or(Err(RouteError::bad_request(err)))
    }

    pub fn f64(&self, key: &str) -> Option<f64> {
        let child = self.keys.get(key)?;
        child.contents.parse().ok()
    }

    pub fn validate_f64(&self, key: &str, err: &str) -> Result<f64, RouteError> {
        let child = self.keys.get(key).ok_or(RouteError::bad_request(err))?;
        child.contents.parse().or(Err(RouteError::bad_request(err)))
    }

    pub fn object(&self, key: &str) -> Option<JsonObject> {
        let child = self.keys.get(key)?;
        if child.content_type != JsonType::Object {
            return None;
        }
        Some(JsonObject::from_string(child.contents.clone()))
    }

    pub fn validate_object(&self, key: &str, err: &str) -> Result<JsonObject, RouteError> {
        let child = self.keys.get(key).ok_or(RouteError::bad_request(err))?;
        if child.content_type != JsonType::Object {
            return Err(RouteError::bad_request(err));
        }
        Ok(JsonObject::from_string(child.contents.clone()))
    }

    pub fn array(&self, key: &str) -> Option<JsonArray> {
        let child = self.keys.get(key)?;
        if child.content_type != JsonType::Array {
            return None;
        }
        Some(JsonArray::from_string(child.contents.clone()))
    }

    pub fn validate_array(&self, key: &str, err: &str) -> Result<JsonArray, RouteError> {
        let child = self.keys.get(key).ok_or(RouteError::bad_request(err))?;
        if child.content_type != JsonType::Array {
            return Err(RouteError::bad_request(err));
        }
        Ok(JsonArray::from_string(child.contents.clone()))
    }
}

#[derive(Debug)]
pub struct JsonArray {
    values: Vec<JsonChild>,
}
impl JsonArray {
    pub fn from_string(json: String) -> JsonArray {
        let mut values: Vec<JsonChild> = Vec::new();
        let json = json[1..json.chars().count() - 1].to_string();

        let mut enumerator = json.chars().into_iter().peekable();
        let mut current_value = JsonChild::new();

        loop {
            if enumerator.peek().is_some() {
                let mut value_start = ' ';
                // Trim any extra whitespace
                loop {
                    if let Some(value_spacing) = enumerator.next() {
                        if value_spacing != ' ' {
                            value_start = value_spacing;
                            break;
                        }
                    } else {
                        break;
                    }
                }
                current_value.content_type = JsonType::type_for_delimiter(value_start);
                // Read value
                if current_value.content_type == JsonType::String {
                    let mut last_value = '0';
                    loop {
                        if let Some(inner_value) = enumerator.next() {
                            if inner_value != '"' {
                                current_value.contents.push(inner_value)
                            } else if last_value == '\\'
                                && (inner_value == '"' || inner_value == '\\')
                            {
                                current_value.contents.pop();
                                current_value.contents.push(inner_value)
                            } else {
                                break;
                            }
                            last_value = inner_value;
                        } else {
                            break;
                        }
                    }
                } else if current_value.content_type.is_primitive() {
                    // We need to add the first index to the value.
                    // Because the other types have delimeters (", {, [)
                    // but primitives do not.
                    current_value.contents.push(value_start);
                    loop {
                        if let Some(inner_value) = enumerator.next() {
                            if inner_value != ',' {
                                current_value.contents.push(inner_value)
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                } else if current_value.content_type == JsonType::Object {
                    let mut delimiter_stack_count = 1;
                    current_value.contents.push('{');
                    loop {
                        if let Some(inner_value) = enumerator.next() {
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
                        } else {
                            break;
                        }
                    }
                } else if current_value.content_type == JsonType::Array {
                    let mut delimiter_stack_count = 1;
                    current_value.contents.push('[');
                    loop {
                        if let Some(inner_value) = enumerator.next() {
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
                        } else {
                            break;
                        }
                    }
                }
                // Because the primitive types do not have a ending delimiter
                // and read straight to the comma, we do not search until a comma
                // if our type is primitive.
                if !current_value.content_type.is_primitive() {
                    loop {
                        if let Some(value_skipper) = enumerator.next() {
                            if value_skipper == ',' {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }
                values.push(current_value);
                current_value = JsonChild::new();
            } else {
                break;
            }
        }

        JsonArray { values }
    }

    pub fn get(&self, index: usize) -> &JsonChild {
        return self.values.get(index).unwrap();
    }

    pub fn all(&self) -> &Vec<JsonChild> {
        return &self.values;
    }
}

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

    pub fn string(&self) -> Option<String> {
        Some(self.contents.clone())
    }

    pub fn i32(&self) -> Option<i32> {
        self.contents.parse().ok()
    }    

    pub fn i64(&self) -> Option<i64> {
        self.contents.parse().ok()
    }

    pub fn f32(&self) -> Option<f32> {
        self.contents.parse().ok()
    }

    pub fn f64(&self) -> Option<f64> {
        self.contents.parse().ok()
    }

    pub fn object(&self) -> Option<JsonObject> {
        if self.content_type != JsonType::Object {
            return None;
        }
        Some(JsonObject::from_string(self.contents.clone()))
    }
    pub fn array(&self) -> Option<JsonArray> {
        if self.content_type != JsonType::Array {
            return None;
        }
        Some(JsonArray::from_string(self.contents.clone()))
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
        return *self == JsonType::Number || *self == JsonType::Boolean;
    }
    pub fn type_for_delimiter(dlm: char) -> JsonType {
        if dlm.is_digit(10) {
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

pub trait ToJson {
    fn to_json(&self) -> String;
}

impl ToJson for String {
    fn to_json(&self) -> String {
        let mut o = String::new();
        o += "\"";
        o += self;
        o += "\"";
        return o;
    }
}
impl ToJson for str {
    fn to_json(&self) -> String {
        let mut o = String::new();
        o += "\"";
        o += self;
        o += "\"";
        return o;
    }
}
impl ToJson for i32 {
    fn to_json(&self) -> String {
        return self.to_string();
    }
}
impl ToJson for i64 {
    fn to_json(&self) -> String {
        return self.to_string();
    }
}
impl ToJson for u32 {
    fn to_json(&self) -> String {
        return self.to_string();
    }
}
impl ToJson for u64 {
    fn to_json(&self) -> String {
        return self.to_string();
    }
}
impl ToJson for f32 {
    fn to_json(&self) -> String {
        return self.to_string();
    }
}
impl ToJson for f64 {
    fn to_json(&self) -> String {
        return self.to_string();
    }
}
impl ToJson for bool {
    fn to_json(&self) -> String {
        return if *self {
            "true".to_string()
        } else {
            "false".to_string()
        };
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
        return output;
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
        return output;
    }
}