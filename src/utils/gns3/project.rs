use crate::GNS3_PROJECT_PREFIX;

pub fn project_name(username: &str) -> String {
    format!("{}{}", GNS3_PROJECT_PREFIX.get().unwrap(), username)
}