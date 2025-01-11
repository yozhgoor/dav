use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    extract::{Path as AxumPath, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use tokio::{fs, io::AsyncWriteExt};
use tracing::{error, info};

const ADDR: &str = "127.0.0.1:3000";

#[derive(Deserialize, Serialize, Debug)]
struct Contact {
    id: String,
    name: String,
    email: String,
    phone: String,
}

#[derive(Clone)]
struct AppState {
    data_dir: Arc<PathBuf>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let base_path = ProjectDirs::from("", "", "dav").expect("failed to determine base directories");
    let data_dir = base_path.data_dir().join("contacts");

    if let Err(e) = tokio::fs::create_dir_all(&data_dir).await {
        error!("failed to create contact directory: {}", e);
        return;
    }
    info!("Data directory created at: {}", data_dir.display());

    let state = AppState {
        data_dir: Arc::new(data_dir),
    };

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/contacts/:id", get(contact_by_id))
        .route("/contacts", post(create_contact))
        .with_state(Arc::new(state));

    let listener = match tokio::net::TcpListener::bind(ADDR).await {
        Ok(listener) => listener,
        Err(e) => {
            error!("failed to bind to address {}: {}", ADDR, e);
            return;
        }
    };

    info!("Server running at http://{}", ADDR);
    if let Err(e) = axum::serve(listener, app).await {
        error!("failed to run server: {}", e);
    }
}

async fn health_check() -> StatusCode {
    StatusCode::OK
}

async fn create_contact(
    State(state): State<Arc<AppState>>,
    Json(contact): Json<Contact>,
) -> impl IntoResponse {
    let file_path = format!("{}/{}.vcf", state.data_dir.display(), contact.id);

    let vcard = format!(
        "BEGIN:VCARD\nVERSION:4.0\nFN:{}\nEMAIL:{}\nTEL:{}\nEND:VCARD\n",
        contact.name, contact.email, contact.phone
    );

    match fs::File::create(&file_path).await {
        Ok(mut file) => {
            if let Err(e) = file.write_all(vcard.as_bytes()).await {
                error!("Error writing to file: {}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, "failed to save contact");
            }

            (StatusCode::CREATED, "Contact created")
        }
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "failed to create file"),
    }
}

async fn contact_by_id(
    AxumPath(id): AxumPath<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let file_path = format!("{}/{}.vcf", state.data_dir.display(), id);

    match fs::read_to_string(&file_path).await {
        Ok(content) => (StatusCode::OK, content),
        Err(_) => (StatusCode::NOT_FOUND, "Contact not found".to_string()),
    }
}
