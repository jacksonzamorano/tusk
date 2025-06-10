use tusk_rs::{HttpMethod, Response, RouteBlock, RouteError, RouteModule};

use crate::{AppRequest, ApplicationConfig};

pub struct StatusModule;

impl StatusModule {
    async fn get_status(data: AppRequest) -> Result<Response, RouteError> {
        return Ok(Response::string(format!(
            "ok: {}\n\n",
            data.configuration.is_production
        )));
    }
}
impl RouteModule<ApplicationConfig> for StatusModule {
    fn apply(&self, block: &mut RouteBlock<ApplicationConfig>) {
        block.add(HttpMethod::Get, "/", StatusModule::get_status);
    }
}
