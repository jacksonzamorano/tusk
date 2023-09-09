use super::{JsonArray, JsonObject, ToJson};
use std::{collections::HashMap, fmt::{Display, Formatter}, matches};

#[derive(Debug)]
pub struct Request {
    pub request_type: RequestType,
    pub path: String,
    pub query: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub body: BodyContents,
}

pub struct Response {
    data: Vec<u8>,
    status: ResponseStatusCode,
    headers: HashMap<String, String>,
}
impl Response {
    pub fn new() -> Response {
        Response {
            data: Vec::new(),
            status: ResponseStatusCode::Ok,
            headers: HashMap::new(),
        }
    }

    pub fn data(data: Vec<u8>) -> Response {
        Response {
            data,
            status: ResponseStatusCode::Ok,
            headers: HashMap::new(),
        }
    }
    pub fn string<S: AsRef<str>>(s: S) -> Response {
        Response::data(s.as_ref().bytes().collect()).header("Content-Type", "text/plain")
    }
    pub fn json<S: ToJson>(s: &S) -> Response {
        Response::data(s.to_json().into_bytes()).header("Content-Type", "application/json; charset=utf-8")
    }
    pub fn html(s: Vec<u8>) -> Response {
        Response::data(s).header("Content-Type", "text/html")
    }

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

    pub fn status(mut self, status: ResponseStatusCode) -> Response {
        self.status = status;
        self
    }

    pub fn header<S: AsRef<str>, T: AsRef<str>>(mut self, key: S, value: T) -> Response {
        self.headers.insert(key.as_ref().to_string(), value.as_ref().to_string());
        self
    }

    // Get bytes out
    pub fn bytes(self) -> Vec<u8> {
        self.data
    }
}
impl Default for Response {
    fn default() -> Self {
        Response::new()
    }
}

pub struct RouteError {
    pub message: String,
    pub status_code: ResponseStatusCode,
    pub override_output: bool,
}
impl RouteError {
    pub fn bad_request(msg: &str) -> RouteError {
        RouteError {
            message: msg.to_string(),
            status_code: ResponseStatusCode::BadRequest,
            override_output: false,
        }
    }
    pub fn forbidden(msg: &str) -> RouteError {
        RouteError {
            message: msg.to_string(),
            status_code: ResponseStatusCode::Forbidden,
            override_output: false,
        }
    }
    pub fn not_found(msg: &str) -> RouteError {
        RouteError {
            message: msg.to_string(),
            status_code: ResponseStatusCode::NotFound,
            override_output: false,
        }
    }
    pub fn conflict(msg: &str) -> RouteError {
        RouteError {
            message: msg.to_string(),
            status_code: ResponseStatusCode::Conflict,
            override_output: false,
        }
    }
    pub fn server_error(msg: &str) -> RouteError {
        RouteError {
            message: msg.to_string(),
            status_code: ResponseStatusCode::InternalServerError,
            override_output: false,
        }
    }
    pub fn custom(msg: &str, status_code: ResponseStatusCode) -> RouteError {
        RouteError {
            message: msg.to_string(),
            status_code,
            override_output: true,
        }
    }

    pub fn output(&self) -> String {
        if self.override_output {
            return self.message.clone();
        }
        let mut o = String::new();
        o += "{\n";
        o += "\t\"code\":\"";
        o += &self.status_code.code().to_string();
        o += "\",\n";
        o += "\t\"message\":\"";
        o += &self.message;
        o += "\"\n}";
        o
    }

    pub fn header(&self) -> Vec<u8> {
        let mut output = String::from("HTTP/1.1 ");
        output += &self.status_code.http_string();
        if !self.override_output {
            output += "\r\nContent-Type: application/json; charset=utf-8";
        }
        output += "\r\n\r\n";
        output.into_bytes()
    }
}

#[derive(Clone)]
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
    PlainText(String),
    None,
}
impl BodyContents {
    const TYPE_JSON: &str = "application/json";
    const TYPE_OCTET_STREAM: &str = "application/octet-stream";
    const TYPE_LD_JSON: &str = "application/ld+json";
    const TYPE_PLAIN_TEXT: &str = "text/plain";

    pub fn type_from_mime(mime: &str, data: Vec<u8>) -> BodyContents {
        match mime {
            BodyContents::TYPE_OCTET_STREAM => BodyContents::Binary(data),
            BodyContents::TYPE_JSON | BodyContents::TYPE_LD_JSON => {
                let contents_string = String::from_utf8(data).unwrap();
                if contents_string.starts_with('[') {
                    BodyContents::JsonArray(JsonArray::from_string(contents_string))
                } else {
                    BodyContents::JsonObject(JsonObject::from_string(contents_string))
                }
            }
            BodyContents::TYPE_PLAIN_TEXT => {
                BodyContents::PlainText(String::from_utf8(data).unwrap())
            }
            _ => BodyContents::Binary(data),
        }
    }

    pub fn json_object(&self) -> Result<&JsonObject, RouteError> {
        match self {
            BodyContents::JsonObject(j) => Ok(j),
            _ => Err(RouteError::bad_request("Expected JSON object.")),
        }
    }
    pub fn as_json_object(self) -> JsonObject {
        match self {
            BodyContents::JsonObject(j) => j,
            _ => JsonObject::from_string("{}".to_string()),
        }
    }
    pub fn json_array(&self) -> Result<&JsonArray, RouteError> {
        match self {
            BodyContents::JsonArray(j) => Ok(j),
            _ => Err(RouteError::bad_request("Expected JSON array.")),
        }
    }
    pub fn as_json_array(self) -> JsonArray {
        match self {
            BodyContents::JsonArray(j) => j,
            _ => JsonArray::from_string("[]".to_string()),
        }
    }
    pub fn as_bytes(self) -> Vec<u8> {
        match self {
            BodyContents::Binary(j) => j,
            _ => Vec::new(),
        }
    }
}

#[derive(Debug)]
pub enum RequestType {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Any,
}

impl RequestType {
    const GET_TYPE: &str = "GET";
    const POST_TYPE: &str = "POST";
    const PATCH_TYPE: &str = "PATCH";
    const PUT_TYPE: &str = "PUT";
    const DELETE_TYPE: &str = "DELETE";
    const ANY_TYPE: &str = "ANY";

    pub fn type_for_method(method: &str) -> RequestType {
        match method {
            RequestType::GET_TYPE => RequestType::Get,
            RequestType::POST_TYPE => RequestType::Post,
            RequestType::PUT_TYPE => RequestType::Put,
            RequestType::PATCH_TYPE => RequestType::Patch,
            RequestType::DELETE_TYPE => RequestType::Delete,
            _ => RequestType::Any,
        }
    }

    pub fn is_any(&self) -> bool {
        matches!(self, RequestType::Any)
    }
}
impl Display for RequestType {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", match self {
            RequestType::Get => RequestType::GET_TYPE.to_string(),
            RequestType::Post => RequestType::POST_TYPE.to_string(),
            RequestType::Put => RequestType::PUT_TYPE.to_string(),
            RequestType::Delete => RequestType::DELETE_TYPE.to_string(),
            RequestType::Patch => RequestType::PATCH_TYPE.to_string(),
            RequestType::Any => RequestType::ANY_TYPE.to_string(),
        })
    }
}