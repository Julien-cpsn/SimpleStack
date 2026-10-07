use crate::CONNECTOR;
use crate::utils::gns3::project::project_name;
use axum_anyhow::{internal_error, ApiResult};
use gns3fy_rs::{Lookup, Project};

pub async fn get_user_project(username: &str) -> ApiResult<Project> {
    let connector = CONNECTOR.clone();
    
    let Some(project) = connector.get_project(Lookup::Name(&project_name(username))).await? else {
        return Err(internal_error("No associated GNS3 project", "Please contact admins"));
    };
    
    Ok(project)
}