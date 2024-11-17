use axum::{
    routing::{get, post},
    Router,
    Json,
    Extension,
    extract::{Path, Query},
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;
use csv::Writer;

use crate::{
    models::{FormResponse, CreateFormResponse},
    error::AppError,
    auth::AuthUser,
};

pub fn router() -> Router {
    Router::new()
        .route("/forms/:id/responses", post(create_response))
        .route("/forms/:id/responses", get(list_responses))
        .route("/forms/:id/responses/export", get(export_responses))
}

async fn create_response(
    auth_user: Option<AuthUser>,
    Extension(pool): Extension<PgPool>,
    Path(form_id): Path<Uuid>,
    Json(payload): Json<CreateFormResponse>,
) -> Result<Json<FormResponse>, AppError> {
    // Verify form exists and allows responses
    let form = sqlx::query!(
        "SELECT allow_anonymous FROM forms WHERE id = $1",
        form_id
    )
    .fetch_optional(&pool)
    .await
    .map_err(AppError::DatabaseError)?
    .ok_or_else(|| AppError::NotFound("Form not found".to_string()))?;

    if !form.allow_anonymous && auth_user.is_none() {
        return Err(AppError::AuthorizationError);
    }

    let response = sqlx::query_as::<_, FormResponse>(
        "INSERT INTO form_responses (form_id, respondent_id, response_data)
         VALUES ($1, $2, $3)
         RETURNING *"
    )
    .bind(form_id)
    .bind(auth_user.map(|u| u.user_id))
    .bind(&payload.response_data)
    .fetch_one(&pool)
    .await
    .map_err(AppError::DatabaseError)?;

    Ok(Json(response))
}

async fn list_responses(
    auth_user: AuthUser,
    Extension(pool): Extension<PgPool>,
    Path(form_id): Path<Uuid>,
) -> Result<Json<Vec<FormResponse>>, AppError> {
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

    let responses = sqlx::query_as::<_, FormResponse>(
        "SELECT * FROM form_responses WHERE form_id = $1 ORDER BY created_at DESC",
        form_id
    )
    .fetch_all(&pool)
    .await
    .map_err(AppError::DatabaseError)?;

    Ok(Json(responses))
}

#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    format: String, // "csv" or "sheets"
}

async fn export_responses(
    auth_user: AuthUser,
    Extension(pool): Extension<PgPool>,
    Path(form_id): Path<Uuid>,
    Query(query): Query<ExportQuery>,
) -> Result<String, AppError> {
    // Verify form ownership
    let form = sqlx::query!(
        "SELECT title FROM forms WHERE id = $1 AND user_id = $2",
        form_id,
        auth_user.user_id
    )
    .fetch_optional(&pool)
    .await
    .map_err(AppError::DatabaseError)?
    .ok_or_else(|| AppError::NotFound("Form not found".to_string()))?;

    let responses = sqlx::query_as::<_, FormResponse>(
        "SELECT * FROM form_responses WHERE form_id = $1 ORDER BY created_at",
        form_id
    )
    .fetch_all(&pool)
    .await
    .map_err(AppError::DatabaseError)?;

    match query.format.as_str() {
        "csv" => {
            let mut wtr = Writer::from_writer(vec![]);
            
            // Write headers
            wtr.write_record(&["Response ID", "Created At", "Response Data"])?;

            // Write data
            for response in responses {
                wtr.write_record(&[
                    response.id.to_string(),
                    response.created_at.to_string(),
                    response.response_data.to_string(),
                ])?;
            }

            let csv_data = String::from_utf8(wtr.into_inner()?)
                .map_err(|_| AppError::InternalError)?;
            
            Ok(csv_data)
        }
        "sheets" => {
            // TODO: Implement Google Sheets export
            Err(AppError::ValidationError("Sheets export not implemented yet".to_string()))
        }
        _ => Err(AppError::ValidationError("Invalid export format".to_string())),
    }
}
