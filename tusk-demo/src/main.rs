use models::{RouteData, TestFromPostgres};
use tusk_rs::{DatabaseConfig, PostgresWriteable, Request, RouteError, DatabaseConnection};
use tusk_rs_derive::{treatment, ToJson};
mod models;
mod util;

#[derive(Debug, ToJson)]
pub struct User {
    email: String,
}

#[treatment]
pub async fn treat_user_data(_req: Request, db: DatabaseConnection, params: std::rc::Rc<User>) -> RouteData {
    dbg!(&params);
    RouteData {}
}

#[tokio::main]
async fn main() {
    let test = TestFromPostgres {
        username: "hello@world.com".to_string(),
        password: "verysecret".to_string(),
    };
    let _write = test.write();
    let config = DatabaseConfig::new();
    let mut server = tusk_rs::Server::new(9000, config, treat_user_data(), User { email: String::new() }).await;
    server.set_cors(
        "*",
        "Origin, X-Requested-With, Content-Type, Accept, Authorization",
    );
    server.start().await;
}
