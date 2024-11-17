use std::env;

pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub paystack_secret_key: String,
    pub paystack_public_key: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            database_url: env::var("DATABASE_URL")
                .expect("DATABASE_URL must be set"),
            jwt_secret: env::var("JWT_SECRET")
                .expect("JWT_SECRET must be set"),
            paystack_secret_key: env::var("PAYSTACK_SECRET_KEY")
                .expect("PAYSTACK_SECRET_KEY must be set"),
            paystack_public_key: env::var("PAYSTACK_PUBLIC_KEY")
                .expect("PAYSTACK_PUBLIC_KEY must be set"),
        }
    }
}

pub const FREE_PLAN_MAX_FORMS: i64 = 3;
pub const SUBSCRIPTION_PLANS: &[&str] = &["free", "unlimited"];
