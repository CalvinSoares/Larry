mod domain;
mod execution;
mod integrations;
mod persistence;
mod secrets;

use std::fs;
use std::path::PathBuf;

use crate::domain::collection::CollectionFile;
use crate::domain::environment::EnvironmentFile;
use crate::domain::request::{HeaderEntry, HttpMethod, QueryParam, RequestBody, RequestDefinition};
use crate::persistence::collections::{load_collection, save_collection, StorageError};
use crate::persistence::environments::{
    load_environment, save_environment, EnvironmentStorageError,
};
use crate::persistence::history::{
    compare_history_entries, get_history_entry, list_history, record_execution, HistoryEntry,
    HistoryError, HistorySummary,
};
use crate::secrets::keyring_store::{delete_secret, set_secret, SecretStoreError};
use serde::Serialize;
use serde_json::json;
use tauri::{AppHandle, Manager};

use execution::http::{execute_with_redactions, ExecutionError, HttpResponse};
use execution::variables::resolve_request;
use integrations::git::{inspect_collection, GitError, GitSnapshot};
use integrations::postman::{preview_collection, PostmanImportError, PostmanImportPreview};

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
async fn execute_request(
    app: AppHandle,
    request: RequestDefinition,
    environment: Option<EnvironmentFile>,
) -> Result<HttpResponse, ExecutionError> {
    let history_request = request.clone();
    let environment_name = environment.as_ref().map(|value| value.name.as_str());
    let resolved = match resolve_request(request, environment.as_ref()) {
        Ok(resolved) => resolved,
        Err(error) => {
            let execution_error = ExecutionError {
                kind: error.kind,
                message: error.message,
                diagnostic: None,
            };
            record_history(
                &app,
                &history_request,
                environment_name,
                &Err(execution_error.clone()),
                &[],
            );
            return Err(execution_error);
        }
    };

    let redactions = resolved.redactions.clone();
    let result = execute_with_redactions(resolved.request, &redactions).await;
    record_history(
        &app,
        &history_request,
        environment_name,
        &result,
        &redactions,
    );
    result
}

#[tauri::command]
fn list_history_entries(
    app: AppHandle,
    limit: Option<u32>,
) -> Result<Vec<HistorySummary>, HistoryError> {
    let path = history_database_path(&app)?;
    list_history(&path.to_string_lossy(), limit)
}

#[tauri::command]
fn get_history_entry_command(app: AppHandle, id: String) -> Result<HistoryEntry, HistoryError> {
    let path = history_database_path(&app)?;
    get_history_entry(&path.to_string_lossy(), &id)
}

#[tauri::command]
fn compare_history_entry_command(
    app: AppHandle,
    left_id: String,
    right_id: String,
) -> Result<crate::persistence::history::HistoryComparison, HistoryError> {
    let path = history_database_path(&app)?;
    compare_history_entries(&path.to_string_lossy(), &left_id, &right_id)
}

fn record_history(
    app: &AppHandle,
    request: &RequestDefinition,
    environment_name: Option<&str>,
    result: &Result<HttpResponse, ExecutionError>,
    redactions: &[String],
) {
    let Ok(path) = history_database_path(app) else {
        return;
    };

    let _ = record_execution(
        &path.to_string_lossy(),
        request,
        environment_name,
        result,
        redactions,
    );
}

fn history_database_path(app: &AppHandle) -> Result<PathBuf, HistoryError> {
    let directory = app
        .path()
        .app_local_data_dir()
        .map_err(|error| HistoryError {
            kind: "history_path".to_string(),
            message: error.to_string(),
        })?;

    fs::create_dir_all(&directory).map_err(|error| HistoryError {
        kind: "history_path".to_string(),
        message: error.to_string(),
    })?;

    Ok(directory.join("history.sqlite3"))
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
fn save_environment_file(
    path: String,
    environment: EnvironmentFile,
) -> Result<(), EnvironmentStorageError> {
    save_environment(&path, &environment)
}

#[tauri::command]
fn load_environment_file(path: String) -> Result<EnvironmentFile, EnvironmentStorageError> {
    load_environment(&path)
}

#[tauri::command]
fn set_environment_secret(secret_ref: String, value: String) -> Result<(), SecretStoreError> {
    set_secret(&secret_ref, &value)
}

#[tauri::command]
fn delete_environment_secret(secret_ref: String) -> Result<(), SecretStoreError> {
    delete_secret(&secret_ref)
}

#[tauri::command]
fn get_git_snapshot(path: String) -> Result<GitSnapshot, GitError> {
    inspect_collection(&path)
}

#[tauri::command]
fn preview_postman_collection(path: String) -> Result<PostmanImportPreview, PostmanImportError> {
    preview_collection(&path)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_app_info,
            get_sample_request,
            execute_request,
            list_history_entries,
            get_history_entry_command,
            compare_history_entry_command,
            save_collection_file,
            load_collection_file,
            save_environment_file,
            load_environment_file,
            set_environment_secret,
            delete_environment_secret,
            get_git_snapshot,
            preview_postman_collection
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
