use models::{RouteData};
use tusk_rs::{
    DatabaseConfig, PostgresConn, Request, Response, RouteError, SelectQuery,
};
use tusk_rs_derive::{route, treatment, autoquery, ToJson};
mod models;
mod util;

#[autoquery]
#[derive(Debug, ToJson)]
pub struct User {
    email: String
}

#[route(Post /)]
pub async fn echo(
    _req: Request,
    db: PostgresConn,
    _data: RouteData,
) -> Result<Response, RouteError> {
    let users = SelectQuery::new()
        .query_all::<User>(&db)
        .await;
    Ok(Response::json(&users?))
}

#[treatment]
pub async fn treat_user_data(_req: Request, db: PostgresConn) -> RouteData {
    RouteData {}
}

#[tokio::main]
async fn main() {
    let config = DatabaseConfig::new();
    let mut server = tusk_rs::Server::new(9000, config, treat_user_data()).await;
    server.register(echo());
    server.set_cors(
        "*",
        "Origin, X-Requested-With, Content-Type, Accept, Authorization",
    );
    server.start().await;
}
