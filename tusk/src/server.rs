use super::{
    BodyContents, HttpMethod, RequestParameters, Response, ResponseStatusCode, RouteError,
};
use crate::route_module::{RouteBlock, RouteModule};
use crate::{config::DatabaseConfig, database::Database};
use crate::{ModernRouteHandler, Request, Route, RouteStorage};
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

/// The core of Tusk, `Server` is a async/await ready
/// web server.
///
/// Server accepts a generic type `T`. This type is injected
/// into all routes as the final argument.
pub struct Server<V> {
    routes: RouteStorage<V>,
    listener: TcpListener,
    database: Database,
    postfix: Option<fn(Response) -> Response>,
    cors_origin: String,
    cors_headers: String,
    debugging_enabled: bool,
    configuration: Arc<V>,
}
impl<V: 'static> Server<V> {
    /// Create a new server.
    /// Specify a port, [`DatabaseConfig`], and an async
    /// function with arguments [`Request`] and a PostgresConn
    /// (alias for [`Object`]) and returns `T`.
    pub async fn new(port: i32, database: DatabaseConfig, configuration: V) -> Server<V> {
        Server {
            routes: RouteStorage::<V>::new(),
            listener: TcpListener::bind(format!("127.0.0.1:{}", port))
                .await
                .unwrap(),
            database: Database::new(database).await.unwrap(),
            postfix: None,
            cors_origin: "*".to_string(),
            cors_headers: "Origin, X-Requested-With, Content-Type, Accept, Authorization"
                .to_string(),
            debugging_enabled: false,
            configuration: Arc::new(configuration),
        }
    }

    /// Enable debugging. This will enable printing verbose information.
    /// This is useful for debugging queries and other issues.
    pub fn enable_debugging(&mut self) {
        self.debugging_enabled = true
    }
    /// Disable debugging. This will disable printing verbose information.
    /// This is the default state.
    pub fn disable_debugging(&mut self) {
        self.debugging_enabled = false
    }

    /// Register a [`Route`]. Routes should NOT be registered
    /// after calling `Server::start`, as all routes are sorted
    /// for peformance when `start` is called.
    ///
    /// See [`Server::register`] for a better way to register routes.
    pub fn register<H, Fut>(&mut self, method: HttpMethod, path: &str, f: H)
    where
        H: Fn(Request<V>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Response, RouteError>> + 'static,
    {
        self.routes.add(Route::new(
            path.to_string(),
            method,
            Box::new(move |req| Box::pin(f(req))),
        ));
    }
    /// Register many [`Route`]s at once. Routes should NOT be registered
    /// after calling `Server::start`, as all routes are sorted
    /// for peformance when `start` is called.
    ///
    /// The recommended pattern for this is to break out
    /// related routes into their own module and decorate
    /// each route with #[route], then export a module function
    /// which returns a Vec of all the routes within.
    /// Note that this has no effect on performance, this just
    /// keeps your code organized.
    pub fn module<T: RouteModule<V>>(&mut self, prefix: &str, module: T) {
        let applied_prefix = if prefix.ends_with('/') {
            prefix[0..prefix.len()].to_string()
        } else {
            prefix.to_string()
        };
        let mut block = RouteBlock {
            routes: Vec::new(),
            prefix: applied_prefix,
        };
        module.apply(&mut block);
        for r in block.routes {
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
    pub async fn start(mut self) {
        let default_route: ModernRouteHandler<V> =
            Box::new(move |req| Box::pin(Server::default_error(req)));
        self.routes.prep();
        loop {
            if let Ok(conn) = self.listener.accept().await {
                let (mut req_stream, _) = conn;
                let req_parsed = self.create_request_object(&mut req_stream).await;
                if req_parsed.request_type == HttpMethod::Options {
                    let mut bytes = Vec::new();
                    let body = self.handle_options();
                    bytes.append(&mut body.get_header_data());
                    bytes.append(&mut body.bytes());
                    _ = req_stream.write(&bytes).await;
                    continue;
                }
                let mut matched_path: &ModernRouteHandler<V> = &default_route;
                if let Some(handler) = self
                    .routes
                    .handler(&req_parsed.request_type, &req_parsed.path)
                {
                    matched_path = &handler.handler;
                }

                let mut bytes = Vec::new();
                let mut response = match self.database.get_connection().await {
                    Ok(db_inst) => {
                        let data = Request {
                            database: db_inst,
                            parameters: req_parsed,
                            configuration: self.configuration.clone(),
                        };
                        matched_path(data).await.unwrap_or_else(|x| x.to_response())
                    }
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
                    let written_bytes = req_stream.write(write_bytes).await;
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

    async fn create_request_object(&self, stream: &mut TcpStream) -> RequestParameters {
        let ip = stream.peer_addr().map(|x| x.ip().to_string()).unwrap_or(String::new());
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

        let mut created_request = RequestParameters {
            path: if wo_query_sect.ends_with('/') {
                wo_query_sect[0..wo_query_sect.len() - 1].to_string()
            } else {
                wo_query_sect.to_string()
            },
            request_type: HttpMethod::type_for_method(head[0]),
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
            ip_address: ip
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

    async fn default_error(_: Request<V>) -> Result<Response, RouteError> {
        Ok(Response::string("404 not found").status(ResponseStatusCode::NotFound))
    }

    pub fn handle_options(&self) -> Response {
        let mut r = Response::data(Vec::new());
        r.apply_cors(&self.cors_origin, &self.cors_headers);
        r
    }
}
