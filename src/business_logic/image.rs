use std::path::PathBuf;
use std::str::FromStr;
use crate::utils::directories::TEMP_DIR;
use axum::extract::{Multipart, State};
use axum::extract::multipart::Field;
use axum::http::StatusCode;
use axum::{Extension, Json};
use axum_anyhow::{conflict, ApiError, ApiResult, bad_request, internal_error};
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set, SqlErr, EntityTrait, QueryFilter, ColumnTrait};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;
use crate::{info, CONNECTOR};
use crate::models::image;
use crate::models::image::Architecture;
use crate::models::user::{User};
use crate::server::{ServerState};
use crate::utils::time::now;

const TARGET: &str = "image";

pub const MAX_UPLOAD_SIZE: u64 = 10 * 1024 * 1024 * 1024;


/// What the upload handler knows once the file has been stored in the GNS3 folder.
pub struct NewOsImage {
    pub filename: String,
    pub stored_path: String,
    pub size_bytes: u64,
    pub architecture: Architecture
}

pub async fn list_user_images(State(state): State<ServerState>, Extension(user): Extension<User>) -> ApiResult<Json<Vec<image::Model>>> {
    let images: Vec<image::Model> = image::Entity::find()
        .filter(image::Column::UploadedBy.eq(user.id))
        .all(&state.orm).await?;

    Ok(Json(images))
}

pub async fn upload_image(State(state): State<ServerState>, Extension(user): Extension<User>, mut multipart: Multipart) -> ApiResult<Json<image::Model>> {
    let mut file_path = None;
    let mut file_name = None;
    let mut architecture = None;

    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|e| bad_request(e.to_string().as_str(), ""))?
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
                    Err(err) => return Err(bad_request("Invalid input", err.to_string().as_str()))
                }
            }
            _ => {}
        }

    }

    let file_path = file_path.unwrap();
    let file_name = file_name.unwrap();
    let file_size = tokio::fs::metadata(&file_path).await?.len();
    let Some(architecture) = architecture else {
        return Err(bad_request("Invalid input", "Missing architecture field"));
    };

    let new_path = file_path.with_file_name(&file_name);

    tokio::fs::rename(&file_path, &new_path).await?;

    if new_path.extension().is_none() {
        let _ = tokio::fs::remove_file(&new_path).await;
        return Err(bad_request("Invalid input", "Image has no extension"));
    }

    if let Some(extension) = new_path.extension() {
        if !["qcow2", "raw", "img", "iso"].contains(&extension.to_str().unwrap()) {
            let _ = tokio::fs::remove_file(&new_path).await;
            return Err(bad_request("Invalid input", "Image format not supported"));
        }
    }

    let connector = CONNECTOR.clone();
    let images = connector.get_compute_images("qemu", "local").await?;

    for image in images {
        if file_name == image.filename {
            let _ = tokio::fs::remove_file(&new_path).await;
            return Err(conflict("Conflict", "Image already uploaded"));
        }
    }

    if let Err(error) = connector.upload_compute_image("qemu", &new_path, "local").await {
        let _ = tokio::fs::remove_file(&new_path).await;
        return Err(bad_request("Invalid input", error.to_string().as_str()));
    }
    else {
        let _ = tokio::fs::remove_file(&new_path).await;
    }

    let image = record_upload(
        &state.orm,
        &user,
        NewOsImage {
            filename: file_name,
            stored_path: file_path.to_string_lossy().to_string(),
            size_bytes: file_size,
            architecture,
        },
    ).await?;
    info!("New image: {:?}", image);

    Ok(Json(image))
}

async fn handle_file_field(field: &mut Field<'_>) -> ApiResult<(PathBuf, String)> {
    let file_path;
    let file_name;

    let path = TEMP_DIR.join(Uuid::new_v4().to_string());
    let mut file = File::create(&path)
        .await
        .map_err(|e| internal_error("Internal server error", e.to_string().as_str()))?;

    if let Some(field_file_name) = field.file_name() {
        file_name = field_file_name.to_string();
    }
    else {
        let _ = tokio::fs::remove_file(&path).await;
        return Err(bad_request("Invalid input", "File has no name"));
    }

    let mut size = 0u64;
    while let Some(chunk) = field
        .chunk()
        .await
        .map_err(|e| bad_request("Invalid input", e.to_string().as_str()))?
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
            .map_err(|e| internal_error("Invalid input", e.to_string().as_str()))?;
    }

    file_path = path;
    file.flush().await.ok();


    Ok((file_path, file_name))
}

/// Stores the metadata of an uploaded image and returns the saved row.
async fn record_upload(orm: &DatabaseConnection, user: &User, image: NewOsImage) -> ApiResult<image::Model> {
    let created_at = now();

    let row = image::ActiveModel {
        id: Set(Uuid::new_v4()),
        filename: Set(image.filename),
        stored_path: Set(image.stored_path),
        size_bytes: Set(i64::try_from(image.size_bytes)?),
        uploaded_by: Set(Some(user.id)),
        architecture: Set(image.architecture),
        created_at: Set(created_at),
    };

    row.insert(orm).await.map_err(|e| match e.sql_err() {
        Some(SqlErr::UniqueConstraintViolation(_)) => {
            conflict("Conflict", "an image with this filename already exists")
        }
        _ => e.into(),
    })
}