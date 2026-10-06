use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use uuid::Uuid;

/// The authenticated user. Never contains the password hash.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub role: Role,
    pub gns3_project_id: String
}

#[derive(FromRow)]
pub struct UserWithHash {
    pub id: Uuid,
    pub username: String,
    pub role: Role,
    pub password_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum Role {
    User,
    Admin,
}