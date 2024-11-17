use axum::{
    routing::{get, post, put, delete},
    Router,
    Json,
    Extension,
    extract::{Path, Query},
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

use crate::{
    models::{Form, CreateForm, FormElement, CreateFormElement, FormShare},
    error::AppError,
    auth::AuthUser,
    config::{Config, FREE_PLAN_MAX_FORMS},
};

pub fn router() -> Router {
    Router::new()
        .route("/forms", post(create_form))
        .route("/forms", get(list_forms))
        .route("/forms/:id", get(get_form))
        .route("/forms/:id", put(update_form))
        .route("/forms/:id", delete(delete_form))
        .route("/forms/:id/elements", post(create_element))
        .route("/forms/:id/elements", get(list_elements))
        .route("/forms/:id/elements/:element_id", put(update_element))
        .route("/forms/:id/elements/:element_id", delete(delete_element))
        .route("/forms/:id/share", post(create_share))
        .route("/forms/:id/share", get(get_share))
}

async fn create_form(
    auth_user: AuthUser,
    Extension(pool): Extension<PgPool>,
    Json(payload): Json<CreateForm>,
) -> Result<Json<Form>, AppError> {
    // Check form limit for free plan
    let form_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM forms WHERE user_id = $1"
    )
    .bind(auth_user.user_id)
    .fetch_one(&pool)
    .await
    .map_err(AppError::DatabaseError)?;

    let user = sqlx::query!(
        "SELECT subscription_plan FROM users WHERE id = $1",
        auth_user.user_id
    )
    .fetch_one(&pool)
    .await
    .map_err(AppError::DatabaseError)?;

    if user.subscription_plan == "free" && form_count >= FREE_PLAN_MAX_FORMS {
        return Err(AppError::ValidationError(
            "Free plan users can only create up to 3 forms".to_string()
        ));
    }

    let form = sqlx::query_as::<_, Form>(
        "INSERT INTO forms (user_id, title, description, is_public, allow_anonymous)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING *"
    )
    .bind(auth_user.user_id)
    .bind(&payload.title)
    .bind(&payload.description)
    .bind(payload.is_public)
    .bind(payload.allow_anonymous)
    .fetch_one(&pool)
    .await
    .map_err(AppError::DatabaseError)?;

    Ok(Json(form))
}

async fn list_forms(
    auth_user: AuthUser,
    Extension(pool): Extension<PgPool>,
) -> Result<Json<Vec<Form>>, AppError> {
    let forms = sqlx::query_as::<_, Form>(
        "SELECT * FROM forms WHERE user_id = $1 ORDER BY created_at DESC"
    )
    .bind(auth_user.user_id)
    .fetch_all(&pool)
    .await
    .map_err(AppError::DatabaseError)?;

    Ok(Json(forms))
}

async fn get_form(
    auth_user: AuthUser,
    Extension(pool): Extension<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<Form>, AppError> {
    let form = sqlx::query_as::<_, Form>(
        "SELECT * FROM forms WHERE id = $1 AND (user_id = $2 OR is_public = true)"
    )
    .bind(id)
    .bind(auth_user.user_id)
    .fetch_optional(&pool)
    .await
    .map_err(AppError::DatabaseError)?
    .ok_or_else(|| AppError::NotFound("Form not found".to_string()))?;

    Ok(Json(form))
}

async fn update_form(
    auth_user: AuthUser,
    Extension(pool): Extension<PgPool>,
    Path(id): Path<Uuid>,
    Json(payload): Json<CreateForm>,
) -> Result<Json<Form>, AppError> {
    let form = sqlx::query_as::<_, Form>(
        "UPDATE forms 
         SET title = $1, description = $2, is_public = $3, allow_anonymous = $4, updated_at = NOW()
         WHERE id = $5 AND user_id = $6
         RETURNING *"
    )
    .bind(&payload.title)
    .bind(&payload.description)
    .bind(payload.is_public)
    .bind(payload.allow_anonymous)
    .bind(id)
    .bind(auth_user.user_id)
    .fetch_optional(&pool)
    .await
    .map_err(AppError::DatabaseError)?
    .ok_or_else(|| AppError::NotFound("Form not found".to_string()))?;

    Ok(Json(form))
}

async fn delete_form(
    auth_user: AuthUser,
    Extension(pool): Extension<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<(), AppError> {
    let result = sqlx::query!(
        "DELETE FROM forms WHERE id = $1 AND user_id = $2",
        id,
        auth_user.user_id
    )
    .execute(&pool)
    .await
    .map_err(AppError::DatabaseError)?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Form not found".to_string()));
    }

    Ok(())
}

async fn create_element(
    auth_user: AuthUser,
    Extension(pool): Extension<PgPool>,
    Path(form_id): Path<Uuid>,
    Json(payload): Json<CreateFormElement>,
) -> Result<Json<FormElement>, AppError> {
    // Verify form ownership
    let form = sqlx::query!(
        "SELECT id FROM forms WHERE id = $1 AND user_id = $2",
        form_id,
        auth_user.user_id
    )
    .fetch_optional(&pool)
    .await
    .map_err(AppError::DatabaseError)?
    .ok_or_else(|| AppError::NotFound("Form not found".to_string()))?;

    let element = sqlx::query_as::<_, FormElement>(
        "INSERT INTO form_elements (form_id, element_type, question, required, options, order_index)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING *"
    )
    .bind(form_id)
    .bind(&payload.element_type)
    .bind(&payload.question)
    .bind(payload.required)
    .bind(&payload.options)
    .bind(payload.order_index)
    .fetch_one(&pool)
    .await
    .map_err(AppError::DatabaseError)?;

    Ok(Json(element))
}

async fn list_elements(
    auth_user: AuthUser,
    Extension(pool): Extension<PgPool>,
    Path(form_id): Path<Uuid>,
) -> Result<Json<Vec<FormElement>>, AppError> {
    // Verify form access
    let form = sqlx::query!(
        "SELECT id FROM forms WHERE id = $1 AND (user_id = $2 OR is_public = true)",
        form_id,
        auth_user.user_id
    )
    .fetch_optional(&pool)
    .await
    .map_err(AppError::DatabaseError)?
    .ok_or_else(|| AppError::NotFound("Form not found".to_string()))?;

    let elements = sqlx::query_as::<_, FormElement>(
        "SELECT * FROM form_elements WHERE form_id = $1 ORDER BY order_index"
    )
    .bind(form_id)
    .fetch_all(&pool)
    .await
    .map_err(AppError::DatabaseError)?;

    Ok(Json(elements))
}

async fn update_element(
    auth_user: AuthUser,
    Extension(pool): Extension<PgPool>,
    Path((form_id, element_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<CreateFormElement>,
) -> Result<Json<FormElement>, AppError> {
    let element = sqlx::query_as::<_, FormElement>(
        "UPDATE form_elements 
         SET element_type = $1, question = $2, required = $3, options = $4, order_index = $5, updated_at = NOW()
         WHERE id = $6 AND form_id = $7 
         AND EXISTS (SELECT 1 FROM forms WHERE id = $7 AND user_id = $8)
         RETURNING *"
    )
    .bind(&payload.element_type)
    .bind(&payload.question)
    .bind(payload.required)
    .bind(&payload.options)
    .bind(payload.order_index)
    .bind(element_id)
    .bind(form_id)
    .bind(auth_user.user_id)
    .fetch_optional(&pool)
    .await
    .map_err(AppError::DatabaseError)?
    .ok_or_else(|| AppError::NotFound("Element not found".to_string()))?;

    Ok(Json(element))
}

async fn delete_element(
    auth_user: AuthUser,
    Extension(pool): Extension<PgPool>,
    Path((form_id, element_id)): Path<(Uuid, Uuid)>,
) -> Result<(), AppError> {
    let result = sqlx::query!(
        "DELETE FROM form_elements 
         WHERE id = $1 AND form_id = $2
         AND EXISTS (SELECT 1 FROM forms WHERE id = $2 AND user_id = $3)",
        element_id,
        form_id,
        auth_user.user_id
    )
    .execute(&pool)
    .await
    .map_err(AppError::DatabaseError)?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Element not found".to_string()));
    }

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShareRequest {
    pub share_type: String,
    pub expires_at: Option<time::OffsetDateTime>,
}

async fn create_share(
    auth_user: AuthUser,
    Extension(pool): Extension<PgPool>,
    Path(form_id): Path<Uuid>,
    Json(payload): Json<ShareRequest>,
) -> Result<Json<FormShare>, AppError> {
    // Verify form ownership
    let form = sqlx::query!(
        "SELECT id FROM forms WHERE id = $1 AND user_id = $2",
        form_id,
        auth_user.user_id
    )
    .fetch_optional(&pool)
    .await
    .map_err(AppError::DatabaseError)?
    .ok_or_else(|| AppError::NotFound("Form not found".to_string()))?;

    let share_token = Uuid::new_v4().to_string();

    let share = sqlx::query_as::<_, FormShare>(
        "INSERT INTO form_shares (form_id, share_type, share_token, expires_at)
         VALUES ($1, $2, $3, $4)
         RETURNING *"
    )
    .bind(form_id)
    .bind(&payload.share_type)
    .bind(&share_token)
    .bind(payload.expires_at)
    .fetch_one(&pool)
    .await
    .map_err(AppError::DatabaseError)?;

    Ok(Json(share))
}

async fn get_share(
    auth_user: AuthUser,
    Extension(pool): Extension<PgPool>,
    Path(form_id): Path<Uuid>,
) -> Result<Json<FormShare>, AppError> {
    let share = sqlx::query_as::<_, FormShare>(
        "SELECT * FROM form_shares 
         WHERE form_id = $1 
         AND EXISTS (SELECT 1 FROM forms WHERE id = $1 AND user_id = $2)
         AND (expires_at IS NULL OR expires_at > NOW())
         ORDER BY created_at DESC
         LIMIT 1"
    )
    .bind(form_id)
    .bind(auth_user.user_id)
    .fetch_optional(&pool)
    .await
    .map_err(AppError::DatabaseError)?
    .ok_or_else(|| AppError::NotFound("Share not found".to_string()))?;

    Ok(Json(share))
}
