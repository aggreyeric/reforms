use axum::routing::{post, get};
use axum::Router;
use axum::extract::Json;
use serde::Deserialize;

mod routes;


#[derive(Debug)]
#[derive(Deserialize)]
struct CreateUser {
    email: String,
    password: String,
}

 async fn jsy(Json(a):Json<CreateUser>) {
    println!("{:?}",a)
       
   }

#[tokio::main]
async fn main() {
   

     let app = Router::new()
     .route("/", get(routes::crud::root))
     .route("/box", get(routes::crud::path_t))
     .route("/users", post(jsy));
    
    




    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    
}
