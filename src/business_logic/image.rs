use std::path::PathBuf;
use std::str::FromStr;
use crate::utils::directories::TEMP_DIR;
use axum::extract::Multipart;
use axum::extract::multipart::Field;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum_anyhow::{ApiError, ApiResult};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;
use crate::{info, CONNECTOR};
use crate::models::image::{Architecture, Image};

const TARGET: &str = "image";

pub const MAX_UPLOAD_SIZE: u64 = 10 * 1024 * 1024 * 1024;

pub async fn upload_image(mut multipart: Multipart) -> ApiResult<impl IntoResponse> {
    let mut file_path = None;
    let mut file_name = None;
    let mut architecture = None;

    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::builder().status(StatusCode::BAD_REQUEST).title(e.to_string()).build())?
    {
        match field.name() {
            Some("file") => {
                let (p, n) = handle_file_field(&mut field).await?;
                file_path = Some(p);
                file_name = Some(n);
            },
            Some("architecture") => {
                let value = field.text().await?;

                match Architecture::from_str(value.as_str()) {
                    Ok(archi) => {
                        architecture = Some(archi);
                    }
                    Err(err) => return Err(ApiError::builder().status(StatusCode::BAD_REQUEST).title(err.to_string()).build())
                }
            }
            _ => {}
        }

    }

    let file_path = file_path.unwrap();
    let file_name = file_name.unwrap();
    let Some(architecture) = architecture else {
        return Err(ApiError::builder().status(StatusCode::BAD_REQUEST).title("Missing architecture field").build());
    };

    let new_path = file_path.with_file_name(&file_name);

    tokio::fs::rename(&file_path, &new_path).await?;

    if new_path.extension().is_none() {
        let _ = tokio::fs::remove_file(&new_path).await;
        return Err(ApiError::builder().status(StatusCode::BAD_REQUEST).title("Image has no extension").build());
    }

    if let Some(extension) = new_path.extension() {
        if !["qcow2", "raw", "img", "iso"].contains(&extension.to_str().unwrap()) {
            let _ = tokio::fs::remove_file(&new_path).await;
            return Err(ApiError::builder().status(StatusCode::BAD_REQUEST).title("Image format not supported").build());
        }
    }

    let connector = CONNECTOR.clone();
    let images = connector.get_compute_images("qemu", "local").await?;

    for image in images {
        if file_name == image.filename {
            let _ = tokio::fs::remove_file(&new_path).await;
            return Err(ApiError::builder().status(StatusCode::BAD_REQUEST).title("Image already uploaded").build());
        }
    }

    if let Err(error) = connector.upload_compute_image("qemu", &new_path, "local").await {
        let _ = tokio::fs::remove_file(&new_path).await;
        return Err(ApiError::builder().status(StatusCode::BAD_REQUEST).title(error.to_string()).build());
    }
    else {
        let _ = tokio::fs::remove_file(&new_path).await;
    }

    let image = Image {
        name: file_name,
        path: file_path,
        architecture,
    };

    info!("New image: {:?}", image);

    Ok(StatusCode::OK)
}

async fn handle_file_field(field: &mut Field<'_>) -> ApiResult<(PathBuf, String)> {
    let file_path;
    let file_name;

    let path = TEMP_DIR.join(Uuid::new_v4().to_string());
    let mut file = File::create(&path)
        .await
        .map_err(|e| ApiError::builder().status(StatusCode::INTERNAL_SERVER_ERROR).title(e.to_string()).build())?;

    if let Some(field_file_name) = field.file_name() {
        file_name = field_file_name.to_string();
    }
    else {
        let _ = tokio::fs::remove_file(&path).await;
        return Err(ApiError::builder().status(StatusCode::BAD_REQUEST).title("File has no name").build());
    }

    let mut size = 0u64;
    while let Some(chunk) = field
        .chunk()
        .await
        .map_err(|e| ApiError::builder().status(StatusCode::BAD_REQUEST).title(e.to_string()).build())?
    {
        size += chunk.len() as u64;

        if size > MAX_UPLOAD_SIZE {
            drop(file);
            let _ = tokio::fs::remove_file(&path).await;
            return Err(
                ApiError::builder()
                    .status(StatusCode::PAYLOAD_TOO_LARGE)
                    .title("File too large")
                    .build()
            );
        }

        file.write_all(&chunk)
            .await
            .map_err(|e| ApiError::builder().status(StatusCode::INTERNAL_SERVER_ERROR).title(e.to_string()).build())?;
    }

    file_path = path;
    file.flush().await.ok();


    Ok((file_path, file_name))
}