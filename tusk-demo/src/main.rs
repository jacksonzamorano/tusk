use tusk_rs::{Request, RouteError, Route, Response};
use tusk_rs_derive::ToJson;
use models::{RouteData, User, Client};
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
        &Client::select_all(&db).await
    ))
}

#[route(Post / : test_interceptor)]
pub async fn create_user(req: Request, db: PostgresConn, data: RouteData) -> Result<Response, RouteError> {
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

pub async fn test_interceptor(_req: &Request, _db: &PostgresConn, _data: &RouteData) -> Result<(), RouteError> {
    return Ok(())
}

#[treatment]
pub async fn treat_user_data(_req: Request, db: PostgresConn) -> Result<RouteData, RouteError> {
    RouteData {}
}

#[tokio::main]
async fn main() {
    let config = DatabaseConfig::new()
        .host("192.168.1.3")
        .username("jacksonzamorano")
        .password("LastBastion080202")
        .database("onecallmaterials_next");
    let mut server = tusk_rs::Server::new(9000, config, treat_user_data()).await;
    server.register(get_users());
    server.register(create_user());
    server.set_cors("*", "Origin, X-Requested-With, Content-Type, Accept, Authorization");
    server.start().await;
}