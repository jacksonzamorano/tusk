use crate::json::{ToJson, JsonArray, JsonObject};
use chrono::{Utc, Datelike, Timelike};
use crate::UrlEncoded;

use std::{collections::{HashMap, BTreeMap}, fmt::{Display, Formatter}, matches};

/// An incoming request. Information is extracted from 
/// the HTTP request and placed nicely into the following fields.
///
/// Body data can be extracted is several different types by 
/// accessing `body`. See more details in [`BodyContents`].
#[derive(Debug)]
pub struct RequestParameters {
    pub request_type: HttpMethod,
    pub path: String,
    pub query: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub body: BodyContents,
    pub ip_address: String
}

/// An outgoing response. This will be converted to HTTP
/// values before being sent. It is recommended to use
/// convenience methods to create this, as it vastly simplifies
/// sending data.
///
/// HTML, JSON, Strings, and Data (`Vec<u8>`) can easily be sent
/// using the respective methods.
pub struct Response {
    pub data: Vec<u8>,
    pub status: ResponseStatusCode,
    pub headers: BTreeMap<String, String>,
}
impl Response {
    const WEEKDAY_MAP: [&'static str;7] = [
        "Mon",
        "Tue",
        "Wed",
        "Thu",
        "Fri",
        "Sat",
        "Sun"
    ];
    const MONTH_MAP: [&'static str;12] = [
        "Jan",
        "Feb",
        "Mar",
        "Apr",
        "May",
        "Jun",
        "Jul",
        "Aug",
        "Sep",
        "Oct",
        "Nov",
        "Dec"
    ];

    /// Create a new, empty response.
    pub fn new() -> Response {
        Response {
            data: Vec::new(),
            status: ResponseStatusCode::Ok,
            headers: BTreeMap::new(),
        }
    }

    /// Create a new response which transmits the data
    /// passed in as raw bytes.
    pub fn data(data: Vec<u8>) -> Response {
        let cur_time = Utc::now();
        let len = data.len();
        Response {
            data,
            status: ResponseStatusCode::Ok,
            headers: BTreeMap::new(),
        }
        .header("Content-Type", "text/html").header("Content-Length", len.to_string())
        .header("Date", format!("{}, {} {} {} {:0>2}:{:0>2}:{:0>2} GMT",
            Self::WEEKDAY_MAP[cur_time.weekday().num_days_from_monday() as usize],
            cur_time.day(),
            Self::MONTH_MAP[(cur_time.month() - 1) as usize],
            cur_time.year(),
            cur_time.hour(),
            cur_time.minute(),
            cur_time.second()
        ))
        .header("Connection", "close")
    }

    /// Create a new response which transmits a string
    /// with `Content-Type` as `text/plain`
    pub fn string<S: AsRef<str>>(s: S) -> Response {
        Response::data(s.as_ref().bytes().collect()).header("Content-Type", "text/plain")
    }

    /// Create a new response which transmits any struct
    /// which implements [`ToJson`].
    /// Data is sent with `Content-Type` as `application/json; charset=utf-8`
    pub fn json<S: ToJson>(s: &S) -> Response {
        Response::data(s.to_json().into_bytes()).header("Content-Type", "application/json; charset=utf-8")
    }

    /// Create a new response which transmits HTML read
    /// from a file. Sends `Content-Type` as `text/html`.
    pub fn html(s: Vec<u8>) -> Response {
        Response::data(s)
    }

    /// Used internally to generate header data
    /// in properly formatted HTTP.
    pub fn get_header_data(&self) -> Vec<u8> {
        let mut output = String::from("HTTP/1.1 ");
        output += &self.status.http_string();
        if !self.headers.is_empty() {
            output += "\r\n";
            output += &self.headers.iter().map(|x| {
                let mut o = String::new();
                o += x.0;
                o += ": ";
                o += x.1;
                o
            }).collect::<Vec<String>>().join("\n");
        }
        output += "\r\n\r\n";
        output.into_bytes()
    }

    /// Set the status. Statuses in Tusk are strongly typed,
    ///  reference [`ResponseStatusCode`].
    pub fn status(mut self, status: ResponseStatusCode) -> Response {
        self.status = status;
        self
    }

    /// Set header values.
    pub fn header<S: AsRef<str>, T: AsRef<str>>(mut self, key: S, value: T) -> Response {
        self.headers.insert(key.as_ref().to_string(), value.as_ref().to_string());
        self
    }

    /// Apply CORS values.
    pub fn apply_cors(&mut self, origin: &String, headers: &String) {
        self.headers.insert("Access-Control-Allow-Origin".to_string(), origin.to_string());
        self.headers.insert("Access-Control-Allow-Headers".to_string(), headers.to_string());
        self.headers.insert("Access-Control-Allow-Methods".to_string(), "POST, PATCH, GET, OPTIONS, DELETE, PUT".to_string());
    }

    /// Convert the body of the request into bytes, consuming
    /// the Response.
    pub fn bytes(self) -> Vec<u8> {
        self.data
    }
}
impl Default for Response {
    fn default() -> Self {
        Response::new()
    }
}

/// RouteError is a struct that lets Tusk know
/// that something went wrong in your application.
/// It automatically can handle simple `String`s as errors,
/// but a custom `String` can be provided by using the `custom`
/// method.
///
/// If the default implementation is used, the response returns
/// in this format:
/// `{
/// code: HTTP CODE,
/// message: "your_message"
/// }`
#[derive(Debug)]
pub struct RouteError {
    pub message: String,
    pub status_code: ResponseStatusCode,
    pub override_output: bool,
}
impl RouteError {
    /// Sends a 400 error with a message.
    pub fn bad_request(msg: &str) -> RouteError {
        RouteError {
            message: msg.to_string(),
            status_code: ResponseStatusCode::BadRequest,
            override_output: false,
        }
    }

    /// Sends a 403 error with a message.
    pub fn forbidden(msg: &str) -> RouteError {
        RouteError {
            message: msg.to_string(),
            status_code: ResponseStatusCode::Forbidden,
            override_output: false,
        }
    }

    /// Sends a 404 error with a message.
    pub fn not_found(msg: &str) -> RouteError {
        RouteError {
            message: msg.to_string(),
            status_code: ResponseStatusCode::NotFound,
            override_output: false,
        }
    }

    /// Sends a 409 error with a message.
    pub fn conflict(msg: &str) -> RouteError {
        RouteError {
            message: msg.to_string(),
            status_code: ResponseStatusCode::Conflict,
            override_output: false,
        }
    }

    /// Sends a 500 error with a message.
    pub fn server_error(msg: &str) -> RouteError {
        RouteError {
            message: msg.to_string(),
            status_code: ResponseStatusCode::InternalServerError,
            override_output: false,
        }
    }

    /// Return a custom error message. The string provided is 
    /// directly sent with no formatting.
    /// A status code is also provided.
    pub fn custom(msg: &str, status_code: ResponseStatusCode) -> RouteError {
        RouteError {
            message: msg.to_string(),
            status_code,
            override_output: false,
        }
    }

    pub fn to_response(self) -> Response {
        let mut o = String::new();
        o += "{\n";
        o += "\t\"code\":\"";
        o += &self.status_code.code().to_string();
        o += "\",\n";
        o += "\t\"message\":\"";
        o += &self.message;
        o += "\"\n}";
        Response::data(o.as_bytes().to_vec())
            .status(self.status_code)
            .header("Content-Type", "Content-Type: application/json; charset=utf-8")
    }
}

/// Struct which strongly types HTTP status code names
/// to their corresponding codes.
#[derive(Clone, Debug)]
pub enum ResponseStatusCode {
    Ok,
    Created,
    Accepted,
    NonAuthoritativeInformation,
    NoContent,
    ResetContent,
    PartialContent,
    MultipleChoices,
    MovedPermanently,
    Found,
    SeeOther,
    NotModified,
    TemporaryRedirect,
    PermanentRedirect,
    BadRequest,
    Unauthorized,
    PaymentRequired,
    Forbidden,
    NotFound,
    MethodNotAllowed,
    NotAcceptable,
    RequestTimeout,
    Conflict,
    Gone,
    LengthRequired,
    PreconditionFailed,
    PayloadTooLarge,
    UriTooLong,
    UnsupportedMediaType,
    RangeNotSatisfiable,
    ExpectationFailed,
    ImATeapot,
    TooEarly,
    PreconditionRequired,
    TooManyRequests,
    InternalServerError,
    NotImplemented,
    BadGateway,
    ServiceUnavailable,
    GatewayTimeout,
    HttpVersionNotSupported,
}
impl ResponseStatusCode {
    pub fn code(&self) -> i32 {
        match self {
            ResponseStatusCode::Ok => 200,
            ResponseStatusCode::Created => 201,
            ResponseStatusCode::Accepted => 202,
            ResponseStatusCode::NonAuthoritativeInformation => 203,
            ResponseStatusCode::NoContent => 204,
            ResponseStatusCode::ResetContent => 205,
            ResponseStatusCode::PartialContent => 206,
            ResponseStatusCode::MultipleChoices => 300,
            ResponseStatusCode::MovedPermanently => 301,
            ResponseStatusCode::Found => 302,
            ResponseStatusCode::SeeOther => 303,
            ResponseStatusCode::NotModified => 304,
            ResponseStatusCode::TemporaryRedirect => 307,
            ResponseStatusCode::PermanentRedirect => 308,
            ResponseStatusCode::BadRequest => 400,
            ResponseStatusCode::Unauthorized => 401,
            ResponseStatusCode::PaymentRequired => 402,
            ResponseStatusCode::Forbidden => 403,
            ResponseStatusCode::NotFound => 404,
            ResponseStatusCode::MethodNotAllowed => 405,
            ResponseStatusCode::NotAcceptable => 406,
            ResponseStatusCode::RequestTimeout => 408,
            ResponseStatusCode::Conflict => 409,
            ResponseStatusCode::Gone => 410,
            ResponseStatusCode::LengthRequired => 411,
            ResponseStatusCode::PreconditionFailed => 412,
            ResponseStatusCode::PayloadTooLarge => 413,
            ResponseStatusCode::UriTooLong => 414,
            ResponseStatusCode::UnsupportedMediaType => 415,
            ResponseStatusCode::RangeNotSatisfiable => 416,
            ResponseStatusCode::ExpectationFailed => 417,
            ResponseStatusCode::ImATeapot => 418,
            ResponseStatusCode::TooEarly => 425,
            ResponseStatusCode::PreconditionRequired => 428,
            ResponseStatusCode::TooManyRequests => 429,
            ResponseStatusCode::InternalServerError => 500,
            ResponseStatusCode::NotImplemented => 501,
            ResponseStatusCode::BadGateway => 502,
            ResponseStatusCode::ServiceUnavailable => 503,
            ResponseStatusCode::GatewayTimeout => 504,
            ResponseStatusCode::HttpVersionNotSupported => 505,
        }
    }
    pub fn http_string(&self) -> String {
        match self {
            ResponseStatusCode::Ok => "200 OK".to_string(),
            ResponseStatusCode::Created => "201 Created".to_string(),
            ResponseStatusCode::Accepted => "202 Accepted".to_string(),
            ResponseStatusCode::NonAuthoritativeInformation => {
                "203 Non-Authoritative Information".to_string()
            }
            ResponseStatusCode::NoContent => "204 No Content".to_string(),
            ResponseStatusCode::ResetContent => "205 Reset Content".to_string(),
            ResponseStatusCode::PartialContent => "206 Partial Content".to_string(),
            ResponseStatusCode::MultipleChoices => "300 Multiple Choices".to_string(),
            ResponseStatusCode::MovedPermanently => "301 Moved Permanently".to_string(),
            ResponseStatusCode::Found => "302 Found".to_string(),
            ResponseStatusCode::SeeOther => "303 See Other".to_string(),
            ResponseStatusCode::NotModified => "304 Not Modified".to_string(),
            ResponseStatusCode::TemporaryRedirect => "307 Temporary Redirect".to_string(),
            ResponseStatusCode::PermanentRedirect => "308 Permanent Redirect".to_string(),
            ResponseStatusCode::BadRequest => "400 Bad Request".to_string(),
            ResponseStatusCode::Unauthorized => "401 Unauthorized".to_string(),
            ResponseStatusCode::PaymentRequired => "402 Payment Required".to_string(),
            ResponseStatusCode::Forbidden => "403 Forbidden".to_string(),
            ResponseStatusCode::NotFound => "404 Not Found".to_string(),
            ResponseStatusCode::MethodNotAllowed => "405 Method Not Allowed".to_string(),
            ResponseStatusCode::NotAcceptable => "406 Not Allowed".to_string(),
            ResponseStatusCode::RequestTimeout => "408 Request Timeout".to_string(),
            ResponseStatusCode::Conflict => "409 Conflict".to_string(),
            ResponseStatusCode::Gone => "410 Gone".to_string(),
            ResponseStatusCode::LengthRequired => "411 Length Required".to_string(),
            ResponseStatusCode::PreconditionFailed => "412 Precondition Failed".to_string(),
            ResponseStatusCode::PayloadTooLarge => "413 Payload Too Large".to_string(),
            ResponseStatusCode::UriTooLong => "414 URI Too Long".to_string(),
            ResponseStatusCode::UnsupportedMediaType => "415 Unsupported Media Type".to_string(),
            ResponseStatusCode::RangeNotSatisfiable => "416 Range Not Satisfiable".to_string(),
            ResponseStatusCode::ExpectationFailed => "417 Expectation Failed".to_string(),
            ResponseStatusCode::ImATeapot => "418 I'm a teapot".to_string(),
            ResponseStatusCode::TooEarly => "425 Too Early".to_string(),
            ResponseStatusCode::PreconditionRequired => "428 Precondition Required".to_string(),
            ResponseStatusCode::TooManyRequests => "429 Too Many Requests".to_string(),
            ResponseStatusCode::InternalServerError => "500 Internal Server Error".to_string(),
            ResponseStatusCode::NotImplemented => "501 Not Implemented".to_string(),
            ResponseStatusCode::BadGateway => "502 Bad Gateway".to_string(),
            ResponseStatusCode::ServiceUnavailable => "503 Service Unavailable".to_string(),
            ResponseStatusCode::GatewayTimeout => "504 Gateway Timeout".to_string(),
            ResponseStatusCode::HttpVersionNotSupported => {
                "505 HTTP Version Not Supported".to_string()
            }
        }
    }
}

#[derive(Debug)]
pub enum BodyContents {
    Binary(Vec<u8>),
    JsonObject(JsonObject),
    JsonArray(JsonArray),
    UrlEncoded(UrlEncoded),
    PlainText(String),
    None,
}
impl BodyContents {
    const TYPE_JSON: &'static str = "application/json";
    const TYPE_OCTET_STREAM: &'static str = "application/octet-stream";
    const TYPE_URL_ENCODED: &'static str = "application/x-www-form-urlencoded";
    const TYPE_LD_JSON: &'static str = "application/ld+json";
    const TYPE_PLAIN_TEXT: &'static str = "text/plain";

    /// Convert raw bytes and a mime type into a [`BodyContents`] variant.
    pub fn type_from_mime(mime: &str, data: Vec<u8>) -> BodyContents {
        match mime {
            BodyContents::TYPE_OCTET_STREAM => BodyContents::Binary(data),
            BodyContents::TYPE_JSON | BodyContents::TYPE_LD_JSON => {
                let contents_string = String::from_utf8(data).unwrap();
                if contents_string.starts_with('[') {
                    BodyContents::JsonArray(JsonArray::from_string(&contents_string))
                } else {
                    BodyContents::JsonObject(JsonObject::from_string(&contents_string))
                }
            }
            BodyContents::TYPE_PLAIN_TEXT => {
                BodyContents::PlainText(String::from_utf8(data).unwrap())
            }
            BodyContents::TYPE_URL_ENCODED => {
                BodyContents::UrlEncoded(UrlEncoded::from_string(String::from_utf8(data).unwrap()))
            }
            _ => BodyContents::Binary(data),
        }
    }

    /// Interpret this body as a JSON object.
    pub fn to_json_object(&self) -> Result<&JsonObject, RouteError> {
        match self {
            BodyContents::JsonObject(j) => Ok(j),
            _ => Err(RouteError::bad_request("Expected JSON object.")),
        }
    }
    /// Interpret this body as a JSON array.
    pub fn to_json_array(&self) -> Result<&JsonArray, RouteError> {
        match self {
            BodyContents::JsonArray(j) => Ok(j),
            _ => Err(RouteError::bad_request("Expected JSON array.")),
        }
    }
    /// Consume the body and return a JSON object.
    pub fn into_json_object(self) -> Result<JsonObject, RouteError> {
        match self {
            BodyContents::JsonObject(j) => Ok(j),
            _ => Err(RouteError::bad_request("Expected JSON object.")),
        }
    }
    /// Consume the body and return a JSON array.
    pub fn into_json_array(self) -> Result<JsonArray, RouteError> {
        match self {
            BodyContents::JsonArray(j) => Ok(j),
            _ => Err(RouteError::bad_request("Expected JSON array")),
        }
    }
    /// Interpret this body as URL encoded form data.
    pub fn url_encoded(&self) -> Result<&UrlEncoded, RouteError> {
        match self {
            BodyContents::UrlEncoded(j) => Ok(j),
            _ => Err(RouteError::bad_request("Expected URL encoded data.")),
        }
    }
    /// Consume the body converting it to URL encoded form data. Returns an empty
    /// structure if the body was of another type.
    pub fn as_url_encoded(self) -> UrlEncoded {
        match self {
            BodyContents::UrlEncoded(j) => j,
            _ => UrlEncoded::from_string("".to_string()),
        }
    }
    /// Consume the body returning the raw bytes.
    pub fn as_bytes(self) -> Vec<u8> {
        match self {
            BodyContents::Binary(j) => j,
            _ => Vec::new(),
        }
    }
}

/// Enum representing supported HTTP methods.
#[derive(Debug)]
#[derive(PartialEq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Any,
    Options,
}

impl HttpMethod {
    const GET_TYPE: &'static str = "GET";
    const POST_TYPE: &'static str = "POST";
    const PATCH_TYPE: &'static str = "PATCH";
    const PUT_TYPE: &'static str = "PUT";
    const DELETE_TYPE: &'static str = "DELETE";
    const ANY_TYPE: &'static str = "ANY";
    const OPTIONS_TYPE: &'static str = "OPTIONS";

    /// Convert a string method from the HTTP request line into an `HttpMethod`.
    pub fn type_for_method(method: &str) -> HttpMethod {
        match method {
            HttpMethod::GET_TYPE => HttpMethod::Get,
            HttpMethod::POST_TYPE => HttpMethod::Post,
            HttpMethod::PUT_TYPE => HttpMethod::Put,
            HttpMethod::PATCH_TYPE => HttpMethod::Patch,
            HttpMethod::DELETE_TYPE => HttpMethod::Delete,
            HttpMethod::OPTIONS_TYPE => HttpMethod::Options,
            _ => HttpMethod::Any,
        }
    }

    /// Returns `true` if this method is [`HttpMethod::Any`].
    pub fn is_any(&self) -> bool {
        matches!(self, HttpMethod::Any)
    }
}
impl Display for HttpMethod {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", match self {
            HttpMethod::Get => HttpMethod::GET_TYPE.to_string(),
            HttpMethod::Post => HttpMethod::POST_TYPE.to_string(),
            HttpMethod::Put => HttpMethod::PUT_TYPE.to_string(),
            HttpMethod::Delete => HttpMethod::DELETE_TYPE.to_string(),
            HttpMethod::Patch => HttpMethod::PATCH_TYPE.to_string(),
            HttpMethod::Any => HttpMethod::ANY_TYPE.to_string(),
            HttpMethod::Options => HttpMethod::OPTIONS_TYPE.to_string()
        })
    }
}
