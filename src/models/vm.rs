use crate::models::image;
use crate::models::image::Architecture;
use crate::utils::gns3::template::get_template_from_node;
use gns3fy_rs::Node;
use sea_orm::QueryFilter;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait};
use serde::Serialize;
use std::str::FromStr;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
pub struct Vm {
    pub id: Uuid,
    pub name: String,
    pub cpu: u64,
    pub ram: u64,
    pub architecture: Architecture,
}

impl Vm {
    pub async fn from_node(node: &Node, orm: &DatabaseConnection) -> anyhow::Result<Self> {
        let template = get_template_from_node(&node).await?;
        let qemu_template = template.kind.as_qemu().unwrap();

        let image_name = qemu_template.hda_disk_image.as_ref().unwrap();
        let image = image::Entity::find()
            .filter(image::Column::Filename.eq(image_name))
            .one(orm)
            .await?
            .unwrap();

        Ok(Self {
            id: Uuid::from_str(node.node_id.as_ref().unwrap().as_str())?,
            name: node.name.as_ref().unwrap().to_string(),
            cpu: qemu_template.cpus.unwrap() as u64,
            ram: qemu_template.ram.unwrap() as u64,
            architecture: image.architecture,
        })
    }
}