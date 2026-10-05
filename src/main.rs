use crate::utils::env::harvest_env_variables;
use crate::utils::gns3::connector::init_gns3_connector;
use gns3fy_rs::{Gns3Connector};
use once_cell::sync::{Lazy, OnceCell};
use std::sync::Arc;
use tokio::net::TcpListener;
use crate::business_logic::auth::{bootstrap_admin};
use crate::business_logic::database::{init_database, purge_sessions};
use crate::business_logic::routes::define_routes;
use crate::server::ServerState;
use crate::utils::directories::create_temp_dir;

mod models;
mod utils;
mod business_logic;
mod server;

const TARGET: &str = "main";

pub static DATABASE_URL: OnceCell<String> = OnceCell::new();
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

    let pool = init_database().await?;
    bootstrap_admin(&pool).await?;
    purge_sessions(pool.clone()).await;

    let server = ServerState::new(pool, false);
    let app  = define_routes(server);

    let listener = TcpListener::bind("0.0.0.0:3000").await?;

    info!("Running server...");
    axum::serve(listener, app).await?;

    Ok(())
}
