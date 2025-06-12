use std::future::Future;

use crate::{Request, HttpMethod, Response, Route, RouteError};

/// Trait implemented by types that group multiple routes together.
///
/// The trait is generic over `V`, the same configuration type that a [`Server`](crate::Server)
/// is parameterized with.  This means a `RouteModule` can freely access whatever user
/// defined state is injected into each [`Request`].  Because Rust monomorphizes generic
/// code, this additional layer of abstraction has zero runtime cost.
///
/// Implementors should populate the provided [`RouteBlock`] with their routes when
/// [`RouteModule::apply`] is called.  Routes added in this way are indistinguishable from
/// those registered directly on the server.
pub trait RouteModule<V> {
    /// Insert routes into the supplied [`RouteBlock`].
    ///
    /// This method is called by [`Server::module`](crate::Server::module).  Use the
    /// provided [`RouteBlock`] to add individual routes with [`RouteBlock::add`].
    fn apply(&self, block: &mut RouteBlock<V>);
}

/// Helper used when building a route module via [`RouteModule`].
///
/// `RouteBlock` carries the same generic configuration type `V` so that handlers added
/// through it receive the exact same [`Request`] type as the rest of your application.
/// The `prefix` field is pre-applied to every route path, avoiding repeated string
/// concatenations when the server starts.
pub struct RouteBlock<V> {
    pub(crate) prefix: String,
    pub(crate) routes: Vec<Route<V>>,
}
impl<V> RouteBlock<V> {
    /// Add a new route to this module.
    ///
    /// `method` and `path` describe the route as usual while `handler` is any async
    /// function taking a [`Request<V>`].  The handler is boxed so using this helper has
    /// the same performance characteristics as registering routes manually on the
    /// [`Server`](crate::Server).
    pub fn add<H, Fut>(&mut self, method: HttpMethod, path: &str, handler: H)
    where
        H: Fn(Request<V>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Response, RouteError>> + 'static,
    {
        let n_path = if path.starts_with("/") {
            format!("{}{}", self.prefix, path)
        } else {
            format!("{}/{}", self.prefix, path)
        };
        self.routes.push(Route::new(
            n_path,
            method,
            Box::new(move |req| Box::pin(handler(req))),
        ));
    }
}
