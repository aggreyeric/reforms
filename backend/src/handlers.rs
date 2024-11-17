use axum::{
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_json::json;

#[derive(Debug, Serialize)]
pub struct SuccessResponse<T> {
    pub data: T,
}

impl<T: Serialize> IntoResponse for SuccessResponse<T> {
    fn into_response(self) -> Response {
        Json(json!({
            "status": "success",
            "data": self.data
        }))
        .into_response()
    }
}