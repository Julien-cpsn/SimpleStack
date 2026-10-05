use std::fs;
use std::path::PathBuf;
use directories::ProjectDirs;
use once_cell::sync::Lazy;

pub static PROJECT_DIRECTORIES: Lazy<ProjectDirs> = Lazy::new(|| ProjectDirs::from("com", "Julien-cpsn", "simple-stack").unwrap());
pub static DATA_LOCAL_DIR: Lazy<PathBuf> = Lazy::new(|| PROJECT_DIRECTORIES.data_local_dir().to_path_buf());
pub static TEMP_DIR: Lazy<PathBuf> = Lazy::new(|| DATA_LOCAL_DIR.join("temp"));

pub fn create_temp_dir() -> anyhow::Result<()> {
    fs::create_dir_all(&*TEMP_DIR)?;

    Ok(())
}