use axum::{
    routing::get,
    Router,
    http::StatusCode,
};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/health", get(health_check));

    // Define the address to run the server
    let addr = SocketAddr::from(([127.0.0.1], 3000));

    println!("Server running at http://{}", addr);
    Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .expect("failed to run server");
}

async fn health_check() -> StatusCode {
    StatusCode::OK
}
