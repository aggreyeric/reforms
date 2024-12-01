use std::collections::HashMap;
use axum::response::IntoResponse;
use axum::extract::{Path,Query, Json};


// pub async fn jsy(Json(json): Json<CreateUser>) -> Json<CreateUser> {
//     Json(json)
// }
use serde::Deserialize;

pub async fn root()-> impl IntoResponse {
    "HELLO WORLD"
    
}


pub async fn path_t(Query(params):Query<HashMap<String,String>>)->String {
    format!("USER ID {:?}", params.into_values().collect::<Vec<String>>())
    
}

pub async fn root2()->String {
    "HELLO WORLD".to_lowercase()
    
}



