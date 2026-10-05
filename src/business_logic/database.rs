use std::{str::FromStr, time::Duration};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;
use crate::business_logic::auth::purge_expired_sessions;
use crate::DATABASE_URL;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS users (
    id            BLOB PRIMARY KEY,
    username      TEXT NOT NULL UNIQUE COLLATE NOCASE,
    password_hash TEXT NOT NULL,
    role          TEXT NOT NULL CHECK (role IN ('User', 'Admin')),
    created_at    INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS sessions (
    token      TEXT PRIMARY KEY,
    user_id    BLOB NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sessions_user    ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_expires ON sessions(expires_at);
"#;

/// Opens (and creates if needed) the SQLite database and applies the schema.
/// `url` looks like `sqlite://simple_stack.db`.
pub async fn init_database() -> anyhow::Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str(DATABASE_URL.get().unwrap().as_str())?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    sqlx::raw_sql(SCHEMA).execute(&pool).await?;
    Ok(pool)
}

pub async fn purge_sessions(purge_pool: SqlitePool) {
    tokio::spawn(async move {
        loop {
            let _ = purge_expired_sessions(&purge_pool).await;
            tokio::time::sleep(Duration::from_secs(3600)).await;
        }
    });
}