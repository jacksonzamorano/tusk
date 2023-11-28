use super::{BodyContents, Request, RequestType, Response, ResponseStatusCode, RouteError};
use crate::DatabaseConnection;
use crate::{config::DatabaseConfig, database::Database};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

/// The core of Tusk, `Server` is a async/await ready
/// web server.
///
/// Server accepts a generic type `T`. This type is injected
/// into all routes as the final argument.
pub struct Server<T> {
    routes: RouteStorage<T>,
    listener: TcpListener,
    database: Database,
    treatment: AsyncTreatmentHandler<T>,
    postfix: Option<fn(Response) -> Response>,
    cors_origin: String,
    cors_headers: String,
    debugging_enabled: bool
}
impl<T: 'static> Server<T> {
    /// Create a new server.
    /// Specify a port, [`DatabaseConfig`], and an async
    /// function with arguments [`Request`] and a PostgresConn
    /// (alias for [`Object`]) and returns `T`.
    pub async fn new(
        port: i32,
        database: DatabaseConfig,
        treatment: AsyncTreatmentHandler<T>,
    ) -> Server<T> {
        Server {
            routes: RouteStorage::new(),
            listener: TcpListener::bind(format!("127.0.0.1:{}", port))
                .await
                .unwrap(),
            database: Database::new(database).await.unwrap(),
            treatment,
            postfix: None,
            cors_origin: "*".to_string(),
            cors_headers: "Origin, X-Requested-With, Content-Type, Accept, Authorization"
                .to_string(),
            debugging_enabled: false
        }
    }
    
    pub fn enable_debugging(&mut self) {
        self.debugging_enabled = true
    }
    pub fn disable_debugging(&mut self) {
        self.debugging_enabled = false
    }

    /// Register a [`Route`]. Routes should NOT be registered
    /// after calling `Server::start`, as all routes are sorted
    /// for peformance when `start` is called.
    pub fn register(&mut self, r: Route<T>) {
        self.routes.add(r);
    }
    /// Register many `Route`s at once.
    pub fn module(&mut self, prefix: &str, rs: Vec<Route<T>>) {
        let mut applied_prefix = if prefix.ends_with('/') {
            prefix[0..prefix.len()].to_string()
        } else {
            prefix.to_string()
        };
        applied_prefix = if !applied_prefix.starts_with('/') {
            format!("/{}", applied_prefix)
        } else {
            applied_prefix
        };
        for mut r in rs {
            r.path = format!("{}{}", applied_prefix, r.path);
            self.routes.add(r);
        }
    }

    /// Add function that can modify all outgoing requests.
    /// Useful for setting headers.
    pub fn set_postfix(&mut self, f: fn(Response) -> Response) {
        self.postfix = Some(f);
    }

    /// Set CORS data
    pub fn set_cors(&mut self, origin: &str, headers: &str) {
        self.cors_origin = origin.to_string();
        self.cors_headers = headers.to_string();
    }

    /// Prepares Tusk for serving applications
    /// and then begins listening.
    pub async fn start(&mut self) {
        self.routes.prep();
        let default: AsyncRouteHandler<T> =
            Box::new(move |a, b, c| Box::pin(Server::default_error(a, b, c)));
        loop {
            if let Ok(conn) = self.listener.accept().await {
                let (mut req_stream, _) = conn;
                let req_parsed = self.create_request_object(&mut req_stream).await;
                if req_parsed.request_type == RequestType::Options {
                    let mut bytes = Vec::new();
                    let body = self.handle_options();
                    bytes.append(&mut body.get_header_data());
                    bytes.append(&mut body.bytes());
                    _ = req_stream.write(&bytes).await;
                    continue;
                }
                let mut matched_path: &AsyncRouteHandler<T> = &default;
                if let Some(handler) = self
                    .routes
                    .handler(&req_parsed.request_type, &req_parsed.path)
                {
                    matched_path = &handler.handler;
                }

                let mut req = IncomingRequest {
                    request: req_parsed,
                    stream: req_stream,
                };
                let mut bytes = Vec::new();
                let mut response = match self.database.get_connection().await {
                    Ok(db_inst) => match (self.treatment)(req.request, db_inst).await {
                        Ok((treat, req, obj)) => {
                            let mut body = matched_path(req, obj, treat)
                                .await
                                .unwrap_or_else(|x| x.to_response());
                            if self.postfix.is_some() {
                                body = self.postfix.unwrap()(body)
                            }
                            body
                        }
                        Err(error) => error.to_response(),
                    },
                    Err(err) => {
                        if self.debugging_enabled {
                            dbg!(err);
                        }
                        RouteError::server_error("Cannot connect to database.").to_response()
                    }
                };
                response.apply_cors(&self.cors_origin, &self.cors_headers);
                bytes.append(&mut response.get_header_data());
                bytes.append(&mut response.bytes());

                let mut write_bytes = bytes.as_slice();
                // Write stream
                loop {
                    let written_bytes = req.stream.write(write_bytes).await;
                    if let Ok(wr_byt) = written_bytes {
                        if wr_byt == write_bytes.len() {
                            break;
                        };
                        write_bytes = &write_bytes[wr_byt..];
                    } else {
                        break;
                    }
                }
            }
        }
    }

    async fn create_request_object(&self, stream: &mut TcpStream) -> Request {
        let mut buffer = BufReader::new(stream);
        let mut headers_content = String::new();

        let mut cur_char: [u8; 1] = [0];
        let mut whitespace_count = 0;

        // Obtain headers
        while buffer.read_exact(&mut cur_char).await.is_ok() {
            let cur_char_val = char::from_u32(cur_char[0] as u32).unwrap();
            headers_content.push(cur_char_val);
            if cur_char_val == '\u{a}' || cur_char_val == '\u{d}' {
                whitespace_count += 1;
            } else {
                whitespace_count = 0;
            }
            // When we have a blank line, exit.
            if whitespace_count == 4 {
                break;
            }
        }
        // Process headers
        let req: Vec<String> = headers_content
            .lines()
            .map(|a| a.to_string())
            .take_while(|a| !a.is_empty())
            .collect();
        let head = &req[0].split(' ').collect::<Vec<&str>>();

        let head_path = head[1].to_string();
        let path = head_path.split('?').collect::<Vec<&str>>();
        let wo_query_sect = path[0].to_string();

        let mut created_request = Request {
            path: if wo_query_sect.ends_with('/') {
                wo_query_sect[0..wo_query_sect.len() - 1].to_string()
            } else {
                wo_query_sect.to_string()
            },
            request_type: RequestType::type_for_method(head[0]),
            query: if let Some(q_d) = path.get(1) {
                q_d.split('&')
                    .map(|x| {
                        let q = x.split('=').collect::<Vec<&str>>();
                        (q[0].to_string(), q.get(1).unwrap_or(&"").to_string())
                    })
                    .collect()
            } else {
                HashMap::new()
            },
            headers: req[1..]
                .to_vec()
                .iter()
                .map(|a| {
                    let d: Vec<&str> = a.split(": ").collect();
                    (d[0].to_string().to_lowercase(), d[1].to_string())
                })
                .collect(),
            body: BodyContents::None,
        };

        if let Some(content_length_str) = created_request.headers.get("content-length") {
            // We have a body.
            let content_len: usize = content_length_str.parse().unwrap_or(0);
            let mut content: Vec<u8> = Vec::new();
            // Read body
            loop {
                if content.len() == content_len {
                    break;
                }
                if buffer.read_exact(&mut cur_char).await.is_ok() {
                    content.push(cur_char[0]);
                }
            }
            if let Some(content_type) = created_request.headers.get("content-type") {
                let no_charset = content_type.split(' ').collect::<Vec<&str>>()[0].replace(';', "");
                created_request.body = BodyContents::type_from_mime(&no_charset, content);
            } else {
                created_request.body = BodyContents::type_from_mime("", content);
            }
        }
        created_request
    }

    async fn default_error(_: Request, _: DatabaseConnection, _: T) -> Result<Response, RouteError> {
        Ok(Response::string("404 not found").status(ResponseStatusCode::NotFound))
    }

    pub fn handle_options(&self) -> Response {
        let mut r = Response::data(Vec::new());
        r.apply_cors(&self.cors_origin, &self.cors_headers);
        r
    }
}

/// A wrapper for a route.
///
/// This struct is created by the `#[route(METHOD path)]` macro,
/// when a function is decorated with that macro, the function is
/// rewritten such that it returns the Route.
///
/// The route function should be an async function
/// with arguments for:
/// - [`Request`]
/// - [`Object`] (aliased to `tusk_rs::PostgresConn` for readability)
/// - `T`
///
/// It should return a `Result<Response, RouteError>`.
///
/// Finally, it must be annotated with the `#[route(METHOD path)]`
/// macro, as it rewrites your function to be passable
/// to a route.
pub struct Route<T> {
    pub path: String,
    pub request_type: RequestType,
    pub handler: AsyncRouteHandler<T>,
}
impl<T> Route<T> {
    /// A route can be manually created, but it is not
    /// recommended.
    pub fn new(path: String, request_type: RequestType, handler: AsyncRouteHandler<T>) -> Route<T> {
        Route {
            path: {
                let mut s_path = path;
                if !s_path.starts_with('/') {
                    s_path = format!("/{}", s_path)
                }
                if s_path.ends_with('/') {
                    s_path = s_path[0..s_path.len() - 1].to_string();
                }
                s_path
            },
            request_type,
            handler,
        }
    }
}

#[derive(Debug)]
pub struct IncomingRequest {
    pub request: Request,
    pub stream: TcpStream,
}

struct RouteStorage<T> {
    routes_get: Vec<Route<T>>,
    routes_post: Vec<Route<T>>,
    routes_put: Vec<Route<T>>,
    routes_patch: Vec<Route<T>>,
    routes_delete: Vec<Route<T>>,
    routes_any: Vec<Route<T>>,
}

impl<T> RouteStorage<T> {
    fn new() -> RouteStorage<T> {
        RouteStorage {
            routes_get: Vec::new(),
            routes_post: Vec::new(),
            routes_put: Vec::new(),
            routes_patch: Vec::new(),
            routes_delete: Vec::new(),
            routes_any: Vec::new(),
        }
    }

    fn handler(&self, request_type: &RequestType, path: &String) -> Option<&Route<T>> {
        let handler_cat = match request_type {
            RequestType::Get => &self.routes_get,
            RequestType::Post => &self.routes_post,
            RequestType::Put => &self.routes_put,
            RequestType::Patch => &self.routes_patch,
            RequestType::Delete => &self.routes_delete,
            _ => &self.routes_any,
        };
        if let Ok(handler_ix) = handler_cat.binary_search_by(|a| a.path.cmp(path)) {
            Some(&handler_cat[handler_ix])
        } else if !request_type.is_any() {
            let any_ix = self
                .routes_any
                .binary_search_by(|a| a.path.cmp(path))
                .ok()?;
            Some(&self.routes_any[any_ix])
        } else {
            None
        }
    }
    fn add(&mut self, route: Route<T>) {
        let handler_cat = match route.request_type {
            RequestType::Get => &mut self.routes_get,
            RequestType::Post => &mut self.routes_post,
            RequestType::Put => &mut self.routes_put,
            RequestType::Patch => &mut self.routes_patch,
            RequestType::Delete => &mut self.routes_delete,
            _ => &mut self.routes_any,
        };
        handler_cat.push(route);
    }

    fn prep(&mut self) {
        self.routes_get.sort_by(|a, b| a.path.cmp(&b.path));
        self.routes_post.sort_by(|a, b| a.path.cmp(&b.path));
        self.routes_put.sort_by(|a, b| a.path.cmp(&b.path));
        self.routes_delete.sort_by(|a, b| a.path.cmp(&b.path));
        self.routes_any.sort_by(|a, b| a.path.cmp(&b.path));
    }
}

type AsyncRouteHandler<T> = Box<
    fn(
        Request,
        crate::DatabaseConnection,
        T,
    ) -> Pin<Box<dyn Future<Output = Result<Response, RouteError>>>>,
>;
type AsyncTreatmentHandler<T> = Box<
    fn(
        Request,
        crate::DatabaseConnection,
    ) -> Pin<Box<dyn Future<Output = Result<(T, Request, crate::DatabaseConnection), RouteError>>>>,
>;
