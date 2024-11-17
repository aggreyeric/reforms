use axum::{
    routing::{post, get},
    Router,
    Json,
    Extension,
    extract::Path,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use reqwest::Client;
use uuid::Uuid;

use crate::{
    models::Subscription,
    error::AppError,
    auth::AuthUser,
    config::Config,
};

pub fn router() -> Router {
    Router::new()
        .route("/payments/initialize", post(initialize_payment))
        .route("/payments/verify/:reference", get(verify_payment))
        .route("/payments/webhook", post(payment_webhook))
}

#[derive(Debug, Serialize)]
struct PaystackInitializeRequest {
    email: String,
    amount: i32,
    callback_url: String,
}

#[derive(Debug, Deserialize)]
struct PaystackInitializeResponse {
    status: bool,
    message: String,
    data: PaystackInitializeData,
}

#[derive(Debug, Deserialize)]
struct PaystackInitializeData {
    authorization_url: String,
    access_code: String,
    reference: String,
}

#[derive(Debug, Serialize)]
pub struct InitializePaymentResponse {
    authorization_url: String,
    reference: String,
}

async fn initialize_payment(
    auth_user: AuthUser,
    Extension(pool): Extension<PgPool>,
) -> Result<Json<InitializePaymentResponse>, AppError> {
    let config = Config::from_env();
    
    // Get user's email
    let user = sqlx::query!(
        "SELECT email FROM users WHERE id = $1",
        auth_user.user_id
    )
    .fetch_one(&pool)
    .await
    .map_err(AppError::DatabaseError)?;

    // Amount in kobo (₦5000 = 500000 kobo)
    let amount = 500000;

    let client = Client::new();
    let response = client
        .post("https://api.paystack.co/transaction/initialize")
        .header("Authorization", format!("Bearer {}", config.paystack_secret_key))
        .json(&PaystackInitializeRequest {
            email: user.email,
            amount,
            callback_url: "http://localhost:3000/payment/callback".to_string(),
        })
        .send()
        .await
        .map_err(|_| AppError::PaymentError("Failed to initialize payment".to_string()))?;

    let paystack_response = response
        .json::<PaystackInitializeResponse>()
        .await
        .map_err(|_| AppError::PaymentError("Invalid response from Paystack".to_string()))?;

    if !paystack_response.status {
        return Err(AppError::PaymentError(paystack_response.message));
    }

    // Create pending subscription
    sqlx::query!(
        "INSERT INTO subscriptions (user_id, plan_type, paystack_reference, status, start_date)
         VALUES ($1, 'unlimited', $2, 'pending', NOW())",
        auth_user.user_id,
        paystack_response.data.reference
    )
    .execute(&pool)
    .await
    .map_err(AppError::DatabaseError)?;

    Ok(Json(InitializePaymentResponse {
        authorization_url: paystack_response.data.authorization_url,
        reference: paystack_response.data.reference,
    }))
}

#[derive(Debug, Deserialize)]
struct PaystackVerifyResponse {
    status: bool,
    message: String,
    data: PaystackVerifyData,
}

#[derive(Debug, Deserialize)]
struct PaystackVerifyData {
    status: String,
    reference: String,
}

async fn verify_payment(
    auth_user: AuthUser,
    Extension(pool): Extension<PgPool>,
    Path(reference): Path<String>,
) -> Result<Json<Subscription>, AppError> {
    let config = Config::from_env();

    let client = Client::new();
    let response = client
        .get(format!("https://api.paystack.co/transaction/verify/{}", reference))
        .header("Authorization", format!("Bearer {}", config.paystack_secret_key))
        .send()
        .await
        .map_err(|_| AppError::PaymentError("Failed to verify payment".to_string()))?;

    let verify_response = response
        .json::<PaystackVerifyResponse>()
        .await
        .map_err(|_| AppError::PaymentError("Invalid response from Paystack".to_string()))?;

    if !verify_response.status {
        return Err(AppError::PaymentError(verify_response.message));
    }

    if verify_response.data.status == "success" {
        // Update subscription and user
        let subscription = sqlx::query_as::<_, Subscription>(
            "UPDATE subscriptions 
             SET status = 'active', 
                 end_date = NOW() + INTERVAL '1 year',
                 updated_at = NOW()
             WHERE paystack_reference = $1 
             AND user_id = $2
             RETURNING *"
        )
        .bind(&reference)
        .bind(auth_user.user_id)
        .fetch_one(&pool)
        .await
        .map_err(AppError::DatabaseError)?;

        sqlx::query!(
            "UPDATE users SET subscription_plan = 'unlimited' WHERE id = $1",
            auth_user.user_id
        )
        .execute(&pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(Json(subscription))
    } else {
        Err(AppError::PaymentError("Payment was not successful".to_string()))
    }
}

#[derive(Debug, Deserialize)]
struct PaystackWebhookEvent {
    event: String,
    data: PaystackWebhookData,
}

#[derive(Debug, Deserialize)]
struct PaystackWebhookData {
    reference: String,
    status: String,
}

async fn payment_webhook(
    Extension(pool): Extension<PgPool>,
    Json(payload): Json<PaystackWebhookEvent>,
) -> Result<(), AppError> {
    match payload.event.as_str() {
        "charge.success" => {
            if payload.data.status == "success" {
                let subscription = sqlx::query!(
                    "SELECT user_id FROM subscriptions WHERE paystack_reference = $1",
                    payload.data.reference
                )
                .fetch_optional(&pool)
                .await
                .map_err(AppError::DatabaseError)?;

                if let Some(subscription) = subscription {
                    // Update subscription status
                    sqlx::query!(
                        "UPDATE subscriptions 
                         SET status = 'active',
                             end_date = NOW() + INTERVAL '1 year',
                             updated_at = NOW()
                         WHERE paystack_reference = $1",
                        payload.data.reference
                    )
                    .execute(&pool)
                    .await
                    .map_err(AppError::DatabaseError)?;

                    // Update user's subscription plan
                    sqlx::query!(
                        "UPDATE users SET subscription_plan = 'unlimited' WHERE id = $1",
                        subscription.user_id
                    )
                    .execute(&pool)
                    .await
                    .map_err(AppError::DatabaseError)?;
                }
            }
        }
        _ => {} // Ignore other events
    }

    Ok(())
}
