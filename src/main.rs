mod utils;

use axum::{
    Json, Router,
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use utils::{
    handler::{get_component, list_components, setup_config},
    structs::ComponentsList,
    utils::{check_dir_exist_or_create, execute_commande},
};

// Error handling
#[derive(Debug)]
enum AppError {
    NotFound,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            AppError::NotFound => (StatusCode::NOT_FOUND, "Component not found"),
        };
        (status, message).into_response()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Component {}

// Server setup
#[tokio::main]
async fn main() {
    // // Initialize logging
    // tracing_subscriber::fmt()
    //     .with_max_level(tracing::Level::DEBUG)
    //     .init();
    // Create router

    // varibale that will host our current git version hash
    let mut current_head_ver = String::new();

    // variable that will host le list of components we will be sending to the client
    let mut components_liste: Vec<ComponentsList> = Vec::new();

    check_dir_exist_or_create("/etc/compo-doc/config/rand.file");
    check_dir_exist_or_create("/etc/compo-doc/tmp/rand.file");

    let app = Router::new()
        .route("/config", post(setup_config))
        .route("/components", get(list_components))
        .route("/components/:id", get(get_component));

    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    // tracing::info!("Server running on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
