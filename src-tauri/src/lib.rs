mod domain;
mod execution;
mod integrations;
mod persistence;

use crate::domain::collection::CollectionFile;
use crate::domain::request::{HeaderEntry, HttpMethod, QueryParam, RequestBody, RequestDefinition};
use crate::persistence::collections::{load_collection, save_collection, StorageError};
use serde::Serialize;
use serde_json::json;

use execution::http::{execute, ExecutionError, HttpResponse};
use integrations::git::{inspect_collection, GitError, GitSnapshot};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    app_name: String,
    version: String,
    operating_system: String,
    architecture: String,
}

#[tauri::command]
fn get_app_info() -> AppInfo {
    AppInfo {
        app_name: "Larry API Client".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        operating_system: std::env::consts::OS.to_string(),
        architecture: std::env::consts::ARCH.to_string(),
    }
}

#[tauri::command]
fn get_sample_request() -> RequestDefinition {
    RequestDefinition {
        id: "request-1".to_string(),
        name: "Criar pagamento".to_string(),
        method: HttpMethod::Post,
        url: "https://api.example.com/payments".to_string(),
        query: vec![QueryParam {
            name: "environment".to_string(),
            value: "sandbox".to_string(),
            enabled: true,
        }],
        headers: vec![HeaderEntry {
            name: "Content-Type".to_string(),
            value: "application/json".to_string(),
            enabled: true,
        }],
        body: Some(RequestBody::Json(json!({
            "amount": 100,
            "currency": "BRL"
        }))),
    }
}

#[tauri::command]
async fn execute_request(request: RequestDefinition) -> Result<HttpResponse, ExecutionError> {
    execute(request).await
}

#[tauri::command]
fn save_collection_file(path: String, collection: CollectionFile) -> Result<(), StorageError> {
    save_collection(&path, &collection)
}

#[tauri::command]
fn load_collection_file(path: String) -> Result<CollectionFile, StorageError> {
    load_collection(&path)
}

#[tauri::command]
fn get_git_snapshot(path: String) -> Result<GitSnapshot, GitError> {
    inspect_collection(&path)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_app_info,
            get_sample_request,
            execute_request,
            save_collection_file,
            load_collection_file,
            get_git_snapshot
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
