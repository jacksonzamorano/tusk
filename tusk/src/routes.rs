use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    DatabaseConnection, JsonParseError, RequestParameters, HttpMethod, Response, RouteError,
};

/// A wrapper for a route.
///
/// This is created by calling register on a server instance.
pub(crate) struct Route<T> {
    pub(crate) path: String,
    pub(crate) request_type: HttpMethod,
    pub(crate) handler: ModernRouteHandler<T>,
}
impl<T> Route<T> {
    pub(crate) fn new(
        path: String,
        request_type: HttpMethod,
        handler: ModernRouteHandler<T>,
    ) -> Route<T> {
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
impl<T> core::fmt::Debug for Route<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Route")
            .field("path", &self.path)
            .field("request_type", &self.request_type)
            .finish()
    }
}
/// Request passed to each route handler.
pub struct Request<V> {
    pub parameters: RequestParameters,
    pub database: DatabaseConnection,

    pub configuration: Arc<V>,
}
pub type Reply = Result<Response, RouteError>;
pub type ModernRouteHandler<V> = Box<
    dyn Fn(Request<V>) -> Pin<Box<dyn Future<Output = Reply>>> + Send + Sync,
>;

/// Internal storage for all registered routes on a [`Server`](crate::Server).
///
/// Routes are grouped by [`HttpMethod`] and stored in separate vectors.  The
/// vectors are sorted once with [`RouteStorage::prep`], allowing route lookups
/// with a binary search.  This keeps handler retrieval to `O(log n)` even as
/// your application grows.
pub(crate) struct RouteStorage<V> {
    routes_get: Vec<Route<V>>,
    routes_post: Vec<Route<V>>,
    routes_put: Vec<Route<V>>,
    routes_patch: Vec<Route<V>>,
    routes_delete: Vec<Route<V>>,
    routes_any: Vec<Route<V>>,
}

impl<T> RouteStorage<T> {
    /// Create an empty [`RouteStorage`].
    pub(crate) fn new() -> RouteStorage<T> {
        RouteStorage {
            routes_get: Vec::new(),
            routes_post: Vec::new(),
            routes_put: Vec::new(),
            routes_patch: Vec::new(),
            routes_delete: Vec::new(),
            routes_any: Vec::new(),
        }
    }

    /// Retrieve the route for the given method and path, if one exists.
    ///
    /// Because the route lists are kept sorted, lookups use `binary_search_by`
    /// and therefore scale logarithmically with the number of routes.
    pub(crate) fn handler(&self, request_type: &HttpMethod, path: &String) -> Option<&Route<T>> {
        let handler_cat = match request_type {
            HttpMethod::Get => &self.routes_get,
            HttpMethod::Post => &self.routes_post,
            HttpMethod::Put => &self.routes_put,
            HttpMethod::Patch => &self.routes_patch,
            HttpMethod::Delete => &self.routes_delete,
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
    /// Add a new route to this storage.
    pub(crate) fn add(&mut self, route: Route<T>) {
        let handler_cat = match route.request_type {
            HttpMethod::Get => &mut self.routes_get,
            HttpMethod::Post => &mut self.routes_post,
            HttpMethod::Put => &mut self.routes_put,
            HttpMethod::Patch => &mut self.routes_patch,
            HttpMethod::Delete => &mut self.routes_delete,
            _ => &mut self.routes_any,
        };
        handler_cat.push(route);
    }

    /// Sort routes for efficient lookup. Called automatically by the server
    /// before it starts listening.
    ///
    /// Sorting occurs only once so there is no runtime cost after startup.
    pub(crate) fn prep(&mut self) {
        self.routes_get.sort_by(|a, b| a.path.cmp(&b.path));
        self.routes_post.sort_by(|a, b| a.path.cmp(&b.path));
        self.routes_put.sort_by(|a, b| a.path.cmp(&b.path));
        self.routes_patch.sort_by(|a, b| a.path.cmp(&b.path));
        self.routes_delete.sort_by(|a, b| a.path.cmp(&b.path));
        self.routes_any.sort_by(|a, b| a.path.cmp(&b.path));
    }
}
impl From<JsonParseError> for RouteError {
    fn from(val: JsonParseError) -> Self {
        match val {
            JsonParseError::NotFound(k) => RouteError::bad_request(&format!("Key {} not found", k)),
            JsonParseError::InvalidType(k, t) => {
                RouteError::bad_request(&format!("Key {} expected type {}", k, t))
            }
        }
    }
}
