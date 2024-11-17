use serde::{Deserialize, Serialize};
use sqlx::types::JsonValue;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub full_name: Option<String>,
    pub subscription_plan: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUser {
    pub email: String,
    pub password: String,
    pub full_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Form {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub is_public: bool,
    pub allow_anonymous: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateForm {
    pub title: String,
    pub description: Option<String>,
    pub is_public: bool,
    pub allow_anonymous: bool,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct FormElement {
    pub id: Uuid,
    pub form_id: Uuid,
    pub element_type: String,
    pub question: String,
    pub required: bool,
    pub options: Option<JsonValue>,
    pub order_index: i32,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateFormElement {
    pub element_type: String,
    pub question: String,
    pub required: bool,
    pub options: Option<JsonValue>,
    pub order_index: i32,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct FormResponse {
    pub id: Uuid,
    pub form_id: Uuid,
    pub respondent_id: Option<Uuid>,
    pub response_data: JsonValue,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateFormResponse {
    pub response_data: JsonValue,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Subscription {
    pub id: Uuid,
    pub user_id: Uuid,
    pub plan_type: String,
    pub paystack_reference: Option<String>,
    pub status: String,
    pub start_date: OffsetDateTime,
    pub end_date: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSubscription {
    pub plan_type: String,
    pub paystack_reference: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct FormShare {
    pub id: Uuid,
    pub form_id: Uuid,
    pub share_type: String,
    pub share_token: String,
    pub expires_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
}
