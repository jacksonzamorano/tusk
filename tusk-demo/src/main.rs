use std::env;

use status::StatusModule;
use tusk_rs::{DatabaseConfig, HttpMethod, Request, Response, RouteError, Server};
use user::UserModule;
mod models;
mod status;
mod user;
mod util;

pub struct ApplicationConfig {
    is_production: bool,
}

type AppRequest = Request<ApplicationConfig>;

async fn index(data: AppRequest) -> Result<Response, RouteError> {
    let contents = if data.configuration.is_production {
        "production"
    } else {
        "development"
    };

    Ok(Response::string(contents))
}

#[tokio::main]
async fn main() {
    let config = DatabaseConfig::new()
        .username(env::var("DATABASE_USERNAME").unwrap())
        .password(env::var("DATABASE_PASSWORD").unwrap())
        .host(env::var("DATABASE_HOST").unwrap())
        .database(env::var("DATABASE_NAME").unwrap());
    let mut server = Server::new(
        9000,
        config,
        ApplicationConfig {
            is_production: true,
        },
    )
    .await;
    server.register(HttpMethod::Get, "/", index);
    server.module("status", StatusModule {});
    server.module("users", UserModule {});
    server.set_cors(
        "*",
        "Origin, X-Requested-With, Content-Type, Accept, Authorization",
    );
    server.start().await;
}
