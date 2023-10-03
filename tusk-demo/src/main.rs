
use models::{NewUser, RouteData};
use tusk_rs::{
    DatabaseConfig, PostgresConn, Request, Response, RouteError,
};
use tusk_rs_derive::{route, treatment};
mod models;
mod util;

// #[derive(ToJson)]
// struct RootResponse {
//     user: Option<NewUser>,
//     version: i32
// }

#[route(Get /)]
pub async fn get_users(
    _req: Request,
    db: PostgresConn,
    _data: RouteData,
) -> Result<Response, RouteError> {
    let users = NewUser::all_users(&db).await;
    // dbg!(user);
    // UpdateQuery::from(user).condition(NewUserQuery::new().id(QueryDetails::equals(Some(3))));
    Ok(Response::json(&users))
}

#[treatment]
pub async fn treat_user_data(_req: Request, db: PostgresConn) -> RouteData {
    RouteData {}
}

#[tokio::main]
async fn main() {
    let config = DatabaseConfig::new();
    let mut server = tusk_rs::Server::new(9000, config, treat_user_data()).await;
    server.register(get_users());
    server.set_cors(
        "*",
        "Origin, X-Requested-With, Content-Type, Accept, Authorization",
    );
    server.start().await;
}
