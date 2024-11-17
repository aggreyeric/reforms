use axum::response::IntoResponse;
use axum::extract::{Path,Query, Json};

pub async fn root()-> impl IntoResponse {
    "HELLO WORLD"
    
}


pub async fn path_t(Path(path_id):Path<u32>)->String {
    format!("USER ID {}", path_id)
    
}

pub async fn root2()->String {
    "HELLO WORLD".to_lowercase()
    
}