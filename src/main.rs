use crate::models::user::User;
use crate::utils::env::harvest_env_variables;
use crate::utils::gns3::connector::init_gns3_connector;
use gns3fy_rs::{Gns3Connector};
use once_cell::sync::{Lazy, OnceCell};
use std::sync::Arc;
use tokio::net::TcpListener;
use crate::business_logic::routes::define_routes;
use crate::utils::directories::create_temp_dir;

mod models;
mod utils;
mod business_logic;


const TARGET: &str = "main";

pub static GNS3_SERVER_URL: OnceCell<String> = OnceCell::new();
pub static GNS3_SERVER_USERNAME: OnceCell<String> = OnceCell::new();
pub static GNS3_SERVER_PASSWORD: OnceCell<String> = OnceCell::new();
pub static GNS3_PROJECT_PREFIX: OnceCell<String> = OnceCell::new();
pub static GNS3_TEMPLATE_PREFIX: OnceCell<String> = OnceCell::new();

pub static CONNECTOR: Lazy<Arc<Gns3Connector>> = Lazy::new(|| init_gns3_connector().expect("Could not initialize gns3 connector"));


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    harvest_env_variables();
    create_temp_dir()?;

    let mut clients = Vec::new();

    let client_1 = User::new(String::from("admin"), String::from("admin"));

    clients.push(client_1);

    let app = define_routes();

    let listener = TcpListener::bind("0.0.0.0:3000").await?;

    info!("Running server...");
    axum::serve(listener, app).await?;

    Ok(())
}
