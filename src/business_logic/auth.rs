use argon2::Argon2;
use axum::{Json, Extension};
use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::http::header::AUTHORIZATION;
use axum::middleware::Next;
use axum::response::Response;
use axum_anyhow::{unauthorized, ApiResult, forbidden, bad_request, conflict, internal_error};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use gns3fy_rs::{Lookup, Project};
use once_cell::sync::Lazy;
use password_hash::{PasswordHasher, PasswordVerifier};
use password_hash::phc::{PasswordHash, SaltString};
use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool};
use uuid::Uuid;
use crate::{info, CONNECTOR};
use crate::models::user::{Role, User, UserWithHash};
use crate::server::{ServerState};
use crate::utils::gns3::project::project_name;
use crate::utils::time::now;

const TARGET: &str = "auth";

pub const SESSION_COOKIE: &str = "session";
const SESSION_TTL_SECS: i64 = 60 * 60 * 24 * 7; // 7 days


// -------------------------------------------------------------- passwords ---

async fn hash_password(password: String) -> ApiResult<String> {
    let hash = tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate();
        Argon2::default()
            .hash_password_with_salt(password.as_bytes(), salt.as_bytes())
            .map(|h| h.to_string())
            .map_err(|e| anyhow::anyhow!("password hashing failed: {e}"))
    })
        .await??;
    Ok(hash)
}

async fn verify_password(password: String, hash: String) -> bool {
    tokio::task::spawn_blocking(move || match PasswordHash::new(&hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    })
        .await
        .unwrap_or(false)
}

/// Verified against when the username doesn't exist, so response time
/// doesn't reveal which usernames are valid.
static DUMMY_HASH: Lazy<String> = Lazy::new(|| {
    let salt = SaltString::generate();
    Argon2::default()
        .hash_password_with_salt(b"dummy-password", salt.as_bytes())
        .expect("dummy hash")
        .to_string()
});

// --------------------------------------------------------------- sessions ---

fn new_token() -> String {
    // 2 x 122 random bits from the OS CSPRNG.
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

async fn create_session(db: &SqlitePool, user_id: Uuid) -> ApiResult<String> {
    let token = new_token();
    let created = now();
    sqlx::query("INSERT INTO sessions (token, user_id, created_at, expires_at) VALUES (?1, ?2, ?3, ?4)")
        .bind(&token)
        .bind(user_id)
        .bind(created)
        .bind(created + SESSION_TTL_SECS)
        .execute(db)
        .await?;
    Ok(token)
}

/// Removes expired sessions. Call periodically (see `main`).
pub async fn purge_expired_sessions(db: &SqlitePool) -> ApiResult<u64> {
    Ok(sqlx::query("DELETE FROM sessions WHERE expires_at <= ?1")
        .bind(now())
        .execute(db)
        .await?
        .rows_affected())
}

fn extract_token(headers: &HeaderMap, jar: &CookieJar) -> Option<String> {
    headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::to_owned)
        .or_else(|| jar.get(SESSION_COOKIE).map(|c| c.value().to_owned()))
}

// ------------------------------------------------------------ middleware ---

/// Runs before every protected request: validates the session, loads the
/// user and inserts it into the request extensions.
pub async fn require_auth(State(state): State<ServerState>, jar: CookieJar, mut req: Request, next: Next) -> ApiResult<Response> {
    let token = extract_token(req.headers(), &jar).ok_or(unauthorized("Unauthorized", ""))?;

    let user = sqlx::query_as::<_, User>(
        "SELECT u.id, u.username, u.role, u.gns3_project_id
           FROM sessions s JOIN users u ON u.id = s.user_id
          WHERE s.token = ?1 AND s.expires_at > ?2",
    )
        .bind(&token)
        .bind(now())
        .fetch_optional(&state.db)
        .await?
        .ok_or(unauthorized("Unauthorized", ""))?;

    req.extensions_mut().insert(user);
    Ok(next.run(req).await)
}

/// Must be layered *inside* `require_auth` (it reads the `User` extension).
pub async fn require_admin(Extension(user): Extension<User>, req: Request, next: Next) -> ApiResult<Response> {
    if user.role != Role::Admin {
        return Err(forbidden("Forbidden", ""));
    }
    Ok(next.run(req).await)
}

// ---------------------------------------------------------------- users ---

async fn create_user_record(db: &SqlitePool, username: &str, password: &str, role: Role) -> ApiResult<User> {
    let username = username.trim();
    if !(3..=32).contains(&username.chars().count()) {
        return Err(bad_request("Bad request", "username must be 3-32 characters"));
    }
    if password.len() < 8 {
        return Err(bad_request("Bad request", "password must be at least 8 characters"));
    }

    let id = Uuid::new_v4();
    let hash = hash_password(password.to_owned()).await?;

    let connector = CONNECTOR.clone();
    let mut project = Project::with_connector(connector).with_name(project_name(username));
    project.create().await?;

    let project_id = project.project_id.unwrap();

    sqlx::query("INSERT INTO users (id, username, password_hash, role, gns3_project_id, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)")
        .bind(id)
        .bind(username)
        .bind(hash)
        .bind(role)
        .bind(&project_id)
        .bind(now())
        .execute(db)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(d) if d.is_unique_violation() => {
                conflict("Conflict", "username already taken")
            }
            other => other.into(),
        })?;

    let user = User {
        id,
        username: username.to_owned(),
        role,
        gns3_project_id: project_id,
    };

    Ok(user)
}

/// If the users table is empty, creates an admin from ADMIN_USERNAME / ADMIN_PASSWORD.
pub async fn bootstrap_admin(db: &SqlitePool) -> anyhow::Result<()> {
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users").fetch_one(db).await?;
    if count > 0 {
        return Ok(());
    }

    create_user_record(db, "admin", "superadmin", Role::Admin)
        .await
        .map_err(|e| anyhow::anyhow!("bootstrap admin failed: {e:?}"))?;

    info!("created initial admin user \"admin\" \"superadmin\"");

    Ok(())
}

// -------------------------------------------------------------- handlers ---

#[derive(Deserialize)]
pub struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    user: User,
    /// Same value as the cookie; for non-browser clients (`Authorization: Bearer <token>`).
    token: String,
}

pub async fn login(State(state): State<ServerState>, jar: CookieJar, Json(body): Json<LoginRequest>) -> ApiResult<(CookieJar, Json<LoginResponse>)> {
    let row = sqlx::query_as::<_, UserWithHash>(
        "SELECT id, username, role, password_hash FROM users WHERE username = ?1",
    )
        .bind(body.username.trim())
        .fetch_optional(&state.db)
        .await?;

    let (hash, row) = match row {
        Some(r) => (r.password_hash.clone(), Some(r)),
        None => (DUMMY_HASH.clone(), None),
    };
    let valid = verify_password(body.password, hash).await;

    let row = match (valid, row) {
        (true, Some(r)) => r,
        _ => return Err(unauthorized("Unauthorized", "")),
    };

    let token = create_session(&state.db, row.id).await?;
    let cookie = Cookie::build((SESSION_COOKIE, token.clone()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(state.cookie_secure)
        .max_age(time::Duration::seconds(SESSION_TTL_SECS))
        .build();

    let connector = CONNECTOR.clone();
    let Some(project) = connector.get_project(Lookup::Name(&project_name(row.username.as_str()))).await? else {
        return Err(internal_error("No associated GNS3 project", "Please contact admins"));
    };

    let user = User {
        id: row.id,
        username: row.username,
        role: row.role,
        gns3_project_id: project.project_id.unwrap(),
    };

    Ok((jar.add(cookie), Json(LoginResponse { user, token })))
}

pub async fn logout(State(state): State<ServerState>, headers: HeaderMap, jar: CookieJar) -> ApiResult<(CookieJar, StatusCode)> {
    if let Some(token) = extract_token(&headers, &jar) {
        sqlx::query("DELETE FROM sessions WHERE token = ?1")
            .bind(token)
            .execute(&state.db)
            .await?;
    }
    let removal = Cookie::build((SESSION_COOKIE, "")).path("/").build();
    Ok((jar.remove(removal), StatusCode::NO_CONTENT))
}

/// Example of a handler that receives the user.
pub async fn me(Extension(user): Extension<User>) -> Json<User> {
    Json(user)
}

#[derive(Deserialize)]
pub struct NewUser {
    username: String,
    password: String,
    role: Role,
}

pub async fn create_user(State(state): State<ServerState>, Json(body): Json<NewUser>) -> ApiResult<(StatusCode, Json<User>)> {
    let user = create_user_record(&state.db, &body.username, &body.password, body.role).await?;
    Ok((StatusCode::CREATED, Json(user)))
}