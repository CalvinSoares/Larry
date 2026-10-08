use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::domain::collection::{CollectionFile, CollectionValidationError};

const MAX_COLLECTION_BYTES: u64 = 5 * 1024 * 1024;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageError {
    pub kind: String,
    pub message: String,
}

impl StorageError {
    fn new(kind: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            message: message.into(),
        }
    }
}

pub fn save_collection(path: &str, collection: &CollectionFile) -> Result<(), StorageError> {
    let path = validate_path(path)?;
    collection.validate().map_err(validation_error)?;

    let content = serde_yaml::to_string(collection)
        .map_err(|error| StorageError::new("serialize", error.to_string()))?;
    let temporary_path = temporary_path(&path);
    let backup_path = backup_path(&path);

    fs::write(&temporary_path, content)
        .map_err(|error| StorageError::new("write", error.to_string()))?;

    let had_existing_file = path.exists();
    if had_existing_file {
        if backup_path.exists() {
            let _ = fs::remove_file(&temporary_path);
            return Err(StorageError::new(
                "stale_backup",
                format!("Já existe um backup temporário: {}", backup_path.display()),
            ));
        }

        if let Err(error) = fs::rename(&path, &backup_path) {
            let _ = fs::remove_file(&temporary_path);
            return Err(StorageError::new("backup", error.to_string()));
        }
    }

    if let Err(error) = fs::rename(&temporary_path, &path) {
        let _ = fs::remove_file(&temporary_path);

        if had_existing_file {
            let _ = fs::rename(&backup_path, &path);
        }

        return Err(StorageError::new("rename", error.to_string()));
    }

    if had_existing_file {
        let _ = fs::remove_file(&backup_path);
    }

    Ok(())
}

pub fn load_collection(path: &str) -> Result<CollectionFile, StorageError> {
    let path = validate_path(path)?;
    let metadata =
        fs::metadata(&path).map_err(|error| StorageError::new("metadata", error.to_string()))?;

    if metadata.len() > MAX_COLLECTION_BYTES {
        return Err(StorageError::new(
            "collection_too_large",
            format!(
                "A collection excede o limite local de {} MiB.",
                MAX_COLLECTION_BYTES / 1024 / 1024
            ),
        ));
    }

    let content =
        fs::read_to_string(&path).map_err(|error| StorageError::new("read", error.to_string()))?;
    let collection: CollectionFile = serde_yaml::from_str(&content)
        .map_err(|error| StorageError::new("parse", error.to_string()))?;

    collection.validate().map_err(validation_error)?;

    Ok(collection)
}

fn validate_path(path: &str) -> Result<PathBuf, StorageError> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(StorageError::new(
            "invalid_path",
            "O caminho da collection não pode estar vazio.",
        ));
    }

    let path = PathBuf::from(trimmed);
    if path.extension().and_then(|extension| extension.to_str()) != Some("yaml") {
        return Err(StorageError::new(
            "invalid_path",
            "Collections devem usar a extensão .yaml.",
        ));
    }

    if path.file_name().is_none() {
        return Err(StorageError::new(
            "invalid_path",
            "O caminho precisa apontar para um arquivo.",
        ));
    }

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            return Err(StorageError::new(
                "missing_parent",
                format!("A pasta pai não existe: {}", parent.display()),
            ));
        }
    }

    Ok(path)
}

fn temporary_path(path: &Path) -> PathBuf {
    let mut temporary_path = path.to_path_buf();
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("yaml");
    temporary_path.set_extension(format!("{extension}.tmp"));
    temporary_path
}

fn backup_path(path: &Path) -> PathBuf {
    let mut backup_path = path.to_path_buf();
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("yaml");
    backup_path.set_extension(format!("{extension}.bak"));
    backup_path
}

fn validation_error(error: CollectionValidationError) -> StorageError {
    let message = match error {
        CollectionValidationError::UnsupportedSchemaVersion {
            received,
            supported,
        } => format!("schemaVersion {received} não é suportado; esperado {supported}."),
        CollectionValidationError::EmptyName => {
            "O nome da collection não pode estar vazio.".to_string()
        }
        CollectionValidationError::EmptyRequestId => "Toda request precisa de um id.".to_string(),
        CollectionValidationError::DuplicateRequestId { id } => {
            format!("O id de request está duplicado: {id}.")
        }
    };

    StorageError::new("invalid_collection", message)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::domain::collection::CURRENT_COLLECTION_SCHEMA_VERSION;
    use crate::domain::request::{HttpMethod, RequestDefinition};

    fn collection() -> CollectionFile {
        CollectionFile {
            schema_version: CURRENT_COLLECTION_SCHEMA_VERSION,
            name: "Fixture local".to_string(),
            requests: vec![RequestDefinition {
                id: "health".to_string(),
                name: "Health check".to_string(),
                method: HttpMethod::Get,
                url: "http://localhost:3000/health".to_string(),
                query: vec![],
                headers: vec![],
                body: None,
            }],
        }
    }

    #[test]
    fn salva_e_carrega_collection_yaml() {
        let directory = std::env::temp_dir();
        let path = directory.join(format!("larry-test-{}-collection.yaml", std::process::id()));
        let path_string = path.to_string_lossy().to_string();

        save_collection(&path_string, &collection()).unwrap();
        let loaded = load_collection(&path_string).unwrap();

        assert_eq!(loaded.name, "Fixture local");
        assert_eq!(loaded.requests[0].url, "http://localhost:3000/health");

        save_collection(&path_string, &collection()).unwrap();
        assert!(!backup_path(&path).exists());

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn rejeita_extensao_incorreta() {
        let error = save_collection("collection.json", &collection()).unwrap_err();

        assert_eq!(error.kind, "invalid_path");
    }
}
