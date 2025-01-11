use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    extract::{Path as AxumPath, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

const ADDR: &str = "127.0.0.1:3000";

#[derive(Default, Deserialize, Serialize, Debug)]
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

    if let Err(e) = fs::create_dir_all(&data_dir) {
        error!("failed to create contact directory: {}", e);
        return;
    }
    info!("Data directory created at: {}", data_dir.display());

    let state = AppState {
        data_dir: Arc::new(data_dir),
    };

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/contacts", get(list_contacts).post(create_contact))
        .route("/contacts/:id", get(contact_by_id))
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
    let file_path = state.data_dir.join(contact.id);

    let vcard = format!(
        "BEGIN:VCARD\nVERSION:4.0\nFN:{}\nEMAIL:{}\nTEL:{}\nEND:VCARD\n",
        contact.name, contact.email, contact.phone
    );

    match fs::File::create(&file_path) {
        Ok(mut file) => {
            if let Err(e) = file.write_all(vcard.as_bytes()) {
                error!("Error writing to file: {}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, "failed to save contact");
            }

            (StatusCode::CREATED, "Contact created")
        }
        Err(e) => {
            error!("failed to create file at {}: {}", file_path.display(), e);
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to create file")
        }
    }
}

async fn contact_by_id(
    AxumPath(id): AxumPath<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let file_path = state.data_dir.join(id);

    match fs::read_to_string(&file_path) {
        Ok(content) => {
            info!("Contact found at {}", file_path.display());
            (StatusCode::OK, content)
        }
        Err(e) => {
            error!("contact not found at {}: {}", file_path.display(), e);
            (StatusCode::NOT_FOUND, "Contact not found".to_string())
        }
    }
}

async fn list_contacts(
    State(state): State<Arc<AppState>>,
) -> Result<(StatusCode, Json<Vec<Contact>>), (StatusCode, String)> {
    match fs::read_dir(&*state.data_dir) {
        Ok(entries) => {
            let mut contacts = Vec::new();
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();

                let id = match path.file_stem().and_then(|stem| stem.to_str()) {
                    Some(stem) => stem.to_string(),
                    None => {
                        error!("invalid file name: {}", path.display());
                        return Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "invalid file name".to_string(),
                        ));
                    }
                };

                if let Ok(content) = fs::read_to_string(&path) {
                    if let Some(contact) = parse_vcard(id, content) {
                        contacts.push(contact);
                    }
                }
            }

            info!("Contacts list created successfully");
            Ok((StatusCode::OK, Json(contacts)))
        }
        Err(e) => {
            error!("failed to list contacts: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to list contacts".to_string(),
            ))
        }
    }
}

fn parse_vcard(id: String, vcard: String) -> Option<Contact> {
    let mut name = None;
    let mut email = None;
    let mut phone = None;

    for line in vcard.lines() {
        if line.starts_with("FN:") {
            name = Some(line.trim_start_matches("FN:").to_string());
        } else if line.starts_with("EMAIL:") {
            email = Some(line.trim_start_matches("EMAIL:").to_string());
        } else if line.starts_with("TEL:") {
            phone = Some(line.trim_start_matches("TEL:").to_string());
        }
    }

    match (name.as_ref(), email.as_ref(), phone.as_ref()) {
        (None, None, None) => None,
        _ => Some(Contact {
            id,
            name: name.unwrap_or_default(),
            email: email.unwrap_or_default(),
            phone: phone.unwrap_or_default(),
        }),
    }
}
