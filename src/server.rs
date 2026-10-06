use sea_orm::DatabaseConnection;
use sqlx::SqlitePool;

/// Shared application state, cloned cheaply into every handler/middleware.
#[derive(Clone)]
pub struct ServerState {
    pub db: SqlitePool,
    pub orm: DatabaseConnection,
    /// Set the `Secure` flag on the session cookie (enable when served over HTTPS).
    pub cookie_secure: bool,
}

impl ServerState {
    pub fn new(db: SqlitePool, orm: DatabaseConnection, cookie_secure: bool) -> Self {
        Self {
            db,
            orm,
            cookie_secure,
        }
    }
}