use axum_anyhow::{not_found, ApiResult};
use gns3fy_rs::ComputeImage;
use crate::CONNECTOR;
use crate::models::image;

pub async fn find_gns3_image(image: &image::Model) -> ApiResult<ComputeImage> {
    let connector = CONNECTOR.clone();

    let mut gns3_images = connector.get_compute_images("qemu", "local").await?;
    let gns3_image = gns3_images
        .drain(..)
        .find(|i| i.filename == image.filename)
        .ok_or(not_found("Not found", "The requested image was not found in GNS3"))?;

    Ok(gns3_image)
}