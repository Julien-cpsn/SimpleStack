use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

/// Metadata of an operating system image. The file itself lives in the GNS3 folder;
/// `stored_path` points to it.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "os_images")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// File name as stored by GNS3 (unique: GNS3 identifies images by name).
    #[sea_orm(unique)]
    pub filename: String,
    pub size_bytes: i64,
    /// `None` if the uploading user was deleted later.
    pub uploaded_by: Option<Uuid>,
    pub architecture: Architecture,
    /// Unix timestamp (seconds).
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}


#[derive(Debug, Clone, PartialEq, DeriveValueType, Serialize, Deserialize, Display, EnumString)]
#[sea_orm(value_type = "String")]
pub enum Architecture {
    #[strum(to_string = "x86")]
    #[serde(rename = "x86")]
    X86,
    #[strum(to_string = "x86_64")]
    #[serde(rename = "x86_64")]
    X86_64,
    #[strum(to_string = "arm64")]
    #[serde(rename = "arm64")]
    ARM64
}