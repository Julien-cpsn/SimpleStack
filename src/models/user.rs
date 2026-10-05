use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub uuid: Uuid,
    pub username: String,
    pub password: String,
    //pub images: Vec<Image>
}

impl User {
    pub fn new(username: String, password: String) -> Self {
        Self {
            uuid: Uuid::new_v4(),
            username,
            password,
        }
    }
}