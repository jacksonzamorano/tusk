use tusk_rs::{Request, RouteError, Route, Response};
use tusk_rs_derive::ToJson;
use models::{RouteData, User};
use tusk_rs::{config::DatabaseConfig, PostgresConn};
use tusk_rs_derive::{route, treatment};
mod models;
mod util;


#[derive(ToJson)]
struct RootResponse {
    user: Option<User>,
    version: i32
}

#[route(Get /)]
pub async fn get_users(_req: Request, db: PostgresConn, _data: RouteData) -> Result<Response, RouteError> {
    Ok(Response::json(
        &User::select_all(&db).await
    ))
}

#[route(Post /)]
pub async fn create_user(req: Request, db: PostgresConn, _data: RouteData) -> Result<Response, RouteError> {
    let json = req.body.as_json_object();
    let user_name = json_string!(json, "name");
    let user_email = json_string!(json, "email");
    let new_user = User {
        id: None,
        name: user_name,
        email: user_email
    };

    let inserted_user = new_user.insert(&db).await;
    Ok(Response::json(&inserted_user))
}

#[treatment]
pub async fn treat_user_data(_req: Request, db: PostgresConn) -> Result<RouteData, RouteError> {
    RouteData {}
}

#[tokio::main]
async fn main() {
    let config = DatabaseConfig::new();
    let mut server = tusk_rs::Server::new(9000, config, treat_user_data()).await;
    server.register(get_users());
    server.register(create_user());
    server.start().await;
}