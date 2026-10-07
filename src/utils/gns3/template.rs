use crate::GNS3_TEMPLATE_PREFIX;
use gns3fy_rs::{Lookup, Node, Template};

pub fn template_name(username: &str, vm_name: &str) -> String {
    format!("{}{}_{}", GNS3_TEMPLATE_PREFIX.get().unwrap(), username, vm_name)
}

pub async fn get_template_from_node(node: &Node) -> anyhow::Result<Template> {
    let connector = node.connector.as_ref().unwrap().clone();
    let template_id = node.template_id.as_ref().unwrap();

    let template = connector.get_template(Lookup::Id(template_id)).await?.unwrap();

    Ok(template)
}