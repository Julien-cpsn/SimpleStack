use axum::Json;
use axum_anyhow::ApiResult;
use gns3fy_rs::ProjectSummary;
use crate::{CONNECTOR, GNS3_PROJECT_PREFIX};

pub async fn get_project_summary() -> ApiResult<Json<Vec<ProjectSummary>>> {
    let connector = CONNECTOR.clone();
    
    let project_summary = connector
        .projects_summary()
        .await?
        .drain(..)
        .filter(|p| p.name.starts_with(GNS3_PROJECT_PREFIX.get().unwrap()))
        .collect();

    Ok(Json(project_summary))
}