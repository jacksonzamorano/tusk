use std::{collections::{HashMap, BTreeMap}};

/// Representation of `application/x-www-form-urlencoded` data.
#[derive(Debug)]
pub struct UrlEncoded {
    values: HashMap<String, String>,
}
impl UrlEncoded {
    /// Parse a URL encoded string into a [`UrlEncoded`] structure.
    pub fn from_string(d: String) -> UrlEncoded {
        UrlEncoded {
            values: d
                .split('&')
                .filter_map(|x| {
                    let x_spl = x.split('=').collect::<Vec<&str>>();
                    Some((
                        x_spl.first()?.to_string().decode_url(),
                        x_spl.get(1)?.to_string(),
                    ))
                })
                .collect(),
        }
    }

    /// Retrieve a value and convert it via [`FromUrlEncoded`].
    pub fn get<S: AsRef<str>, T: FromUrlEncoded>(&self, key: S) -> Option<T> {
        T::from_url_encoded(&self.values.get(key.as_ref())?.clone().decode_url())
    }

    /// Retrieve nested dictionary-style data.
    pub fn get_dict<S: AsRef<str>>(&self, key: S) -> Option<UrlEncoded> {
        let matched_values = self
            .values
            .iter()
            .filter(|x| {
                let sw = format!("{}[", key.as_ref());
                x.0.starts_with(&sw)
            })
            .map(|(k, v)| { 
                let mut new_k = k.clone();
                new_k.replace_range(0..key.as_ref().len() + 1, "");
                new_k = new_k.replacen(']', "", 1);
                format!("{}={}", new_k, v)
            })
            .collect::<Vec<String>>()
            .join("&");
        Some(UrlEncoded::from_string(matched_values))
    }

    /// Retrieve an array of dictionaries encoded in the form data.
    pub fn get_vec_dict<S: AsRef<str>>(&self, key: S) -> Option<Vec<UrlEncoded>> {
        let matched_values = self
            .values
            .iter()
            .filter(|x| {
                let sw = format!("{}[", key.as_ref());
                x.0.starts_with(&sw)
            })
            .map(|(k, v)| { 
                let mut new_k = k.clone();
                new_k.replace_range(0..key.as_ref().len() + 1, "");
                (new_k, v.to_owned())
            })
            .collect::<Vec<(String, String)>>();
        let mut values = BTreeMap::<i32, String>::new();

        for v in matched_values {
            let split = v.0.replacen('[', "", 1);
            let new_key_data = split.split(']').collect::<Vec<&str>>();
            let array_num: i32 = new_key_data[0].parse().unwrap();
            let i_value = format!("{}={}", new_key_data[1], v.1);
            if let Some(x) = values.get_mut(&array_num) { x.push('&'); x.push_str(&i_value) }
            else { values.insert(array_num, i_value); }
        }

        Some(values.into_values().map(UrlEncoded::from_string).collect::<Vec<_>>())
    }

    /// Retrieve an array of primitive values encoded in the form data.
    pub fn get_vec<S: AsRef<str>, T: FromUrlEncoded>(&self, key: S) -> Option<Vec<T>> {
        let matched_values = self
            .values
            .iter()
            .filter(|x| {
                let sw = format!("{}[", key.as_ref());
                x.0.starts_with(&sw)
            })
            .map(|(k, v)| { 
                let mut new_k = k.clone();
                new_k.replace_range(0..key.as_ref().len() + 1, "");
                (new_k, v.to_owned())
            })
            .collect::<Vec<(String, String)>>();
        let mut values = BTreeMap::<i32, String>::new();

        for v in matched_values {
            let split = v.0.replacen('[', "", 1);
            let new_key_data = split.split(']').collect::<Vec<&str>>();
            let array_num: i32 = new_key_data[0].parse().unwrap();
            values.insert(array_num, v.1);
        }

        Some(values.into_values().filter_map(|x| T::from_url_encoded(&x.decode_url())).collect::<Vec<_>>())
    }
}

/// Trait for types that can be deserialized from a form encoded string value.
pub trait FromUrlEncoded {
    /// Attempt to parse `data` into `Self` returning `None` on failure.
    fn from_url_encoded(data: &str) -> Option<Self>
    where
        Self: Sized;
}

impl FromUrlEncoded for String {
    fn from_url_encoded(data: &str) -> Option<Self> {
        Some(data.to_string())
    }
}

impl FromUrlEncoded for i32 {
    fn from_url_encoded(data: &str) -> Option<Self> {
        data.parse().ok()
    }
}
impl FromUrlEncoded for f64 {
    fn from_url_encoded(data: &str) -> Option<Self> {
        data.parse().ok()
    }
}

trait UrlEncodedParse {
    fn decode_url(self) -> String;
}
impl UrlEncodedParse for String {
    fn decode_url(self) -> String {
        self.replace('+', " ")
            .replace("%20", " ")
            .replace("%21", "!")
            .replace("%22", "\"")
            .replace("%23", "#")
            .replace("%24", "$")
            .replace("%26", "&")
            .replace("%27", "'")
            .replace("%28", "(")
            .replace("%29", ")")
            .replace("%2A", "*")
            .replace("%2B", "+")
            .replace("%2C", ",")
            .replace("%25", "%")
            .replace("%5B", "[")
            .replace("%5D", "]")
    }
}
