use models::{RouteData, TestFromPostgres};
use tusk_rs::{DatabaseConfig, PostgresConn, PostgresWriteable, Request, Response, RouteError};
use tusk_rs_derive::{route, treatment, ToJson};
mod models;
mod util;

#[derive(Debug, ToJson)]
pub struct User {
    email: String,
}

#[treatment]
pub async fn treat_user_data(_req: Request, db: PostgresConn) -> RouteData {
    RouteData {}
}

#[tokio::main]
async fn main() {
    let test = TestFromPostgres {
        username: "hello@world.com".to_string(),
        password: "verysecret".to_string(),
    };
    let write = test.write();
    let config = DatabaseConfig::new();
    let mut server = tusk_rs::Server::new(9000, config, treat_user_data()).await;
    server.set_cors(
        "*",
        "Origin, X-Requested-With, Content-Type, Accept, Authorization",
    );
    server.start().await;
}
