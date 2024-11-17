use axum::{
    routing::{post, get},
    Router,
    Json,
    Extension,
    extract::Path,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use validator::Validate;

use crate::{
    models::{User, CreateUser},
    error::AppError,
    auth::{hash_password, verify_password, create_token, AuthUser},
    config::Config,
};

pub fn router() -> Router {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/me", get(me))
}

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(email)]
    email: String,
    #[validate(length(min = 6))]
    password: String,
    full_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    token: String,
    user: User,
}

async fn register(
    Json(payload): Json<RegisterRequest>,
    Extension(pool): Extension<PgPool>,
) -> Result<Json<AuthResponse>, AppError> {
    payload.validate().map_err(|e| AppError::ValidationError(e.to_string()))?;

    let password_hash = hash_password(&payload.password)?;

    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (email, password_hash, full_name, subscription_plan)
         VALUES ($1, $2, $3, 'free')
         RETURNING *"
    )
    .bind(&payload.email)
    .bind(&password_hash)
    .bind(&payload.full_name)
    .fetch_one(&pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(e) if e.constraint() == Some("users_email_key") => {
            AppError::ValidationError("Email already exists".to_string())
        }
        _ => AppError::DatabaseError(e),
    })?;

    let config = Config::from_env();
    let token = create_token(user.id, &config)?;

    Ok(Json(AuthResponse { token, user }))
}

async fn login(
    Json(payload): Json<LoginRequest>,
    Extension(pool): Extension<PgPool>,
) -> Result<Json<AuthResponse>, AppError> {
    let user = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE email = $1"
    )
    .bind(&payload.email)
    .fetch_optional(&pool)
    .await
    .map_err(AppError::DatabaseError)?
    .ok_or_else(|| AppError::AuthError)?;

    if !verify_password(&payload.password, &user.password_hash)? {
        return Err(AppError::AuthError);
    }

    let config = Config::from_env();
    let token = create_token(user.id, &config)?;

    Ok(Json(AuthResponse { token, user }))
}

async fn me(
    auth_user: AuthUser,
    Extension(pool): Extension<PgPool>,
) -> Result<Json<User>, AppError> {
    let user = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE id = $1"
    )
    .bind(auth_user.user_id)
    .fetch_one(&pool)
    .await
    .map_err(AppError::DatabaseError)?;

    Ok(Json(user))
}
