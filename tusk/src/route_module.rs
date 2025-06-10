use std::future::Future;

use crate::{Request, HttpMethod, Response, Route, RouteError};

pub trait RouteModule<V> {
    fn apply(&self, block: &mut RouteBlock<V>);
}

pub struct RouteBlock<V> {
    pub(crate) prefix: String,
    pub(crate) routes: Vec<Route<V>>,
}
impl<V> RouteBlock<V> {
    pub fn add<H, Fut>(&mut self, method: HttpMethod, path: &str, handler: H)
    where
        H: Fn(Request<V>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Response, RouteError>> + Send + 'static,
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
