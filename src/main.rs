use std::fmt;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use axum::{
    extract::{Path as AxumPath, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

const ADDR: &str = "127.0.0.1:3000";

#[derive(Default, Deserialize, Serialize, Debug)]
struct Contact {
    id: String,
    name: String,
    email: String,
    phone: String,
}

impl FromStr for Contact {
    type Err = String;

    fn from_str(vcard: &str) -> Result<Self, Self::Err> {
        let mut id = None;
        let mut name = None;
        let mut email = None;
        let mut phone = None;

        for line in vcard.lines() {
            if line.starts_with("ID:") {
                id = Some(line.trim_start_matches("ID:").to_string());
            } else if line.starts_with("FN:") {
                name = Some(line.trim_start_matches("FN:").to_string());
            } else if line.starts_with("EMAIL:") {
                email = Some(line.trim_start_matches("EMAIL:").to_string());
            } else if line.starts_with("TEL:") {
                phone = Some(line.trim_start_matches("TEL:").to_string());
            }
        }

        match (id.as_ref(), name.as_ref(), email.as_ref(), phone.as_ref()) {
            (None, None, None, None) => Err("contact is empty".to_string()),
            (None, _, _, _) => Err("contact ID is empty".to_string()),
            _ => Ok(Contact {
                id: id.unwrap_or_default(),
                name: name.unwrap_or_default(),
                email: email.unwrap_or_default(),
                phone: phone.unwrap_or_default(),
            }),
        }
    }
}

impl fmt::Display for Contact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BEGIN:VCARD\nVERSION:4.0\nID:{}\nFN:{}\nEMAIL:{}\nTEL:{}\nEND:VCARD\n",
            self.id, self.name, self.email, self.phone
        )
    }
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
        .route(
            "/contacts/:id",
            get(contact_by_id)
                .put(modify_contact)
                .delete(delete_contact),
        )
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
) -> (StatusCode, String) {
    let mut file_path = state.data_dir.join(contact.id);
    file_path.set_extension("vcf");

    let vcard = format!(
        "BEGIN:VCARD\nVERSION:4.0\nFN:{}\nEMAIL:{}\nTEL:{}\nEND:VCARD\n",
        contact.name, contact.email, contact.phone
    );

    match fs::File::create(&file_path) {
        Ok(mut file) => {
            if let Err(e) = file.write_all(vcard.as_bytes()) {
                error!("Error writing to file: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to save contact".to_string(),
                );
            }

            (StatusCode::CREATED, "Contact created".to_string())
        }
        Err(e) => {
            error!("failed to create file at {}: {}", file_path.display(), e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to create file".to_string(),
            )
        }
    }
}

async fn modify_contact(
    AxumPath(id): AxumPath<String>,
    State(state): State<Arc<AppState>>,
    Json(updated_contact): Json<Contact>,
) -> (StatusCode, String) {
    let mut file_path = state.data_dir.join(&id);
    file_path.set_extension("vcf");

    if !file_path.exists() {
        warn!("contact not found for update: {}", file_path.display());
        return (StatusCode::NOT_FOUND, "contact not found".to_string());
    }

    if id != updated_contact.id {
        warn!("ID '{}' does not match body ID: {}", id, updated_contact.id);
        return (
            StatusCode::BAD_REQUEST,
            "ID in URL and body must match".to_string(),
        );
    }

    match fs::write(&file_path, updated_contact.to_string()) {
        Ok(_) => {
            info!("contact updated: {}", file_path.display());
            (StatusCode::OK, "Contact updated".to_string())
        }
        Err(e) => {
            error!("failed to update contact {}: {}", file_path.display(), e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to update contact".to_string(),
            )
        }
    }
}

async fn delete_contact(
    AxumPath(id): AxumPath<String>,
    State(state): State<Arc<AppState>>,
) -> (StatusCode, String) {
    let mut file_path = state.data_dir.join(id);
    file_path.set_extension("vcf");

    if file_path.exists() {
        match fs::remove_file(&file_path) {
            Ok(_) => {
                info!("Contact deleted: {}", file_path.display());
                (StatusCode::OK, "Contact deleted".to_string())
            }
            Err(e) => {
                error!("failed to delete contact {}: {}", file_path.display(), e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to delete contact".to_string(),
                )
            }
        }
    } else {
        warn!("contact not found for deletion: {}", file_path.display());
        (StatusCode::NOT_FOUND, "contact not found".to_string())
    }
}

async fn contact_by_id(
    AxumPath(id): AxumPath<String>,
    State(state): State<Arc<AppState>>,
) -> (StatusCode, String) {
    let mut file_path = state.data_dir.join(id);
    file_path.set_extension("vcf");

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

                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(contact) = content.parse::<Contact>() {
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
