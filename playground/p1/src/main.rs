use axum::routing::get;
use axum::Router;

mod routes;

#[tokio::main]
async fn main() {
   

     let app = Router::new()
     .route("/", get(routes::crud::root))
     .route("/:path_id", get(routes::crud::path_t));



    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    
}
