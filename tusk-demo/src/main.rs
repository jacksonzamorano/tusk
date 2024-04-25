use models::{RouteData};
use tusk_rs::{DatabaseConfig, Request, RouteError, DatabaseConnection, Response};
use tusk_rs_derive::{treatment, ToJson, embed_binary, route, PostgresJoins};
mod models;
mod util;

pub const PDF_DATA: &[u8] = embed_binary!("test.pdf");

#[derive(Debug, ToJson, PostgresJoins)]
pub struct User {
    email: String,
}

#[route(Get /pdf)]
pub async fn pdf(_req: Request, _db: DatabaseConnection, _params: RouteData) -> Result<Response, RouteError> {
    Ok(Response::data(PDF_DATA.to_vec()).header("Content-Type", "application/pdf"))
}

#[treatment]
pub async fn treat_user_data(_req: Request, db: DatabaseConnection, params: std::rc::Rc<User>) -> RouteData {
    dbg!(&params);
    RouteData {}
}

#[tokio::main]
async fn main() {
    let config = DatabaseConfig::new();
    let mut server = tusk_rs::Server::new(9000, config, treat_user_data(), User { email: String::new() }).await;
    server.register(pdf());
    server.set_cors(
        "*",
        "Origin, X-Requested-With, Content-Type, Accept, Authorization",
    );
    server.start().await;
}
