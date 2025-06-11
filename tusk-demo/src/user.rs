use tusk_rs::{HttpMethod, Response, RouteBlock, RouteError, RouteModule};

use crate::{models::{User, UserDirectory}, AppRequest, ApplicationConfig};

pub struct UserModule;

impl RouteModule<ApplicationConfig> for UserModule {
    fn apply(&self, block: &mut RouteBlock<ApplicationConfig>) {
        block.add(HttpMethod::Get, "/", UserModule::get_all);
        block.add(HttpMethod::Get, "directory", UserModule::get_all_directory);
    }
}

impl UserModule {
    pub async fn get_all(data: AppRequest) -> Result<Response, RouteError> {
        let users: Vec<User> = data.database.select_all("", &[]).await.unwrap();
        Ok(Response::json(&users))
    }
    pub async fn get_all_directory(data: AppRequest) -> Result<Response, RouteError> {
        let org_users: Vec<UserDirectory> = data.database.select_all("", &[]).await.unwrap();
        Ok(Response::json(&org_users))
    }
}
