use crate::{GNS3_SERVER_PASSWORD, GNS3_SERVER_URL, GNS3_SERVER_USERNAME};
use gns3fy_rs::Gns3Connector;
use std::sync::Arc;

pub fn init_gns3_connector() -> anyhow::Result<Arc<Gns3Connector>> {
    let server_url = GNS3_SERVER_URL.get().unwrap();
    let server_username = GNS3_SERVER_USERNAME.get().unwrap();
    let server_password = GNS3_SERVER_PASSWORD.get().unwrap();
    
    let connector = Gns3Connector::builder(server_url)
        .user(server_username)
        .cred(server_password)
        .build()?;
    
    Ok(Arc::new(connector))
}