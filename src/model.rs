use std::{sync::Arc, time::SystemTime};

use crate::config;
use actix_web::web;
use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::env;
use tokio::sync::Mutex;

pub struct AppState {
    pub init_ts: SystemTime,
    // pub half_db: Arc<Mutex<Vec<User>>>,
    // pub db: Arc<Mutex<Vec<CRUser>>>,
    pub pool: Arc<Mutex<SqlitePool>>,
    pub env: config::Config,
}

impl AppState {
    pub async fn init() -> AppState {
        AppState {
            init_ts: SystemTime::now(),
            pool: Arc::new(Mutex::new(
                SqlitePool::connect(&env::var("DATABASE_URL").expect("missing DATABASE_URL"))
                    .await
                    .unwrap(),
            )),
            env: config::Config::init(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenClaims {
    pub sub: String,
    pub iat: usize,
    pub exp: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RawUser {
    pub id: String,
    pub email: String,
    pub name: String,
    pub lastopen_ts: Option<String>,
    pub photo: String,
    pub verified: i64,
    pub created_at: String,
    pub updated_at: String,
    pub admin: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub id: String,
    pub email: String,
    pub name: String,
    pub lastopen_ts: Option<DateTime<Utc>>,
    pub photo: String,
    pub verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub admin: bool,
}

impl From<RawUser> for User {
    fn from(value: RawUser) -> Self {
        let lastopen_ts = value
            .lastopen_ts
            .map(|v| DateTime::parse_from_rfc3339(&v).unwrap_or_default().into());
        Self {
            id: value.id,
            email: value.email,
            name: value.name,
            lastopen_ts,
            photo: value.photo,
            verified: value.verified == 1,
            created_at: DateTime::parse_from_rfc3339(&value.created_at)
                .unwrap_or_default()
                .into(),
            updated_at: DateTime::parse_from_rfc3339(&value.updated_at)
                .unwrap_or_default()
                .into(),
            admin: value.admin == 1,
        }
    }
}

impl From<User> for RawUser {
    fn from(value: User) -> Self {
        let lastopen_ts = value.lastopen_ts.map(|v| v.to_rfc3339());
        Self {
            id: value.id,
            email: value.email,
            name: value.name,
            lastopen_ts,
            photo: value.photo,
            verified: if value.verified { 1 } else { 0 },
            created_at: value.created_at.to_rfc3339(),
            updated_at: value.updated_at.to_rfc3339(),
            admin: if value.admin { 1 } else { 0 },
        }
    }
}

impl User {
    pub async fn get_by_id(id: &str, data: &web::Data<AppState>) -> Result<User, ()> {
        let mut db = data.pool.lock().await.acquire().await.unwrap();
        if let Ok(Some(v)) = sqlx::query_as!(
            RawUser,
            r#"
    SELECT *
    FROM users
    WHERE id = ?"#,
            id
        )
        .fetch_optional(&mut db)
        .await
        {
            let user: User = v.into();
            Ok(user)
        } else {
            Err(())
        }
    }
    pub async fn insert(self, data: &web::Data<AppState>) -> Result<(), ()> {
        let mut db = data.pool.lock().await.acquire().await.unwrap();
        let user: RawUser = self.into();
        if sqlx::query_as!(RawUser,
                            r#"
                            INSERT INTO users (id, email, name, lastopen_ts, photo, verified, created_at, updated_at, admin)
                            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                            "#, user.id, user.email, user.name, user.lastopen_ts, user.photo, user.verified, user.created_at, user.updated_at, user.admin).execute(&mut db).await.is_ok() {
            Ok(())
                            } else {
                                Err(())
                            }
    }
    pub async fn update(self, data: &web::Data<AppState>) -> Result<(), ()> {
        let mut db = data.pool.lock().await.acquire().await.unwrap();
        let user: RawUser = self.into();
        if sqlx::query_as!(RawUser,
                            r#"
                            UPDATE users SET id = ?1, email = ?2, name = ?3, lastopen_ts = ?4, photo = ?5, verified = ?6, created_at = ?7, updated_at = ?8, admin = ?9 WHERE id = ?10                             "#, user.id, user.email, user.name, user.lastopen_ts, user.photo, user.verified, user.created_at, user.updated_at, user.admin, user.id).execute(&mut db).await.is_ok() {
            Ok(())
                            } else {
                                Err(())
                            }
    }
}
