use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::domain::environment::{EnvironmentFile, EnvironmentValidationError};

const MAX_ENVIRONMENT_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentStorageError {
    pub kind: String,
    pub message: String,
}

impl EnvironmentStorageError {
    fn new(kind: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            message: message.into(),
        }
    }
}

pub fn save_environment(
    path: &str,
    environment: &EnvironmentFile,
) -> Result<(), EnvironmentStorageError> {
    let path = validate_path(path)?;
    environment.validate().map_err(validation_error)?;

    let content = serde_yaml::to_string(environment)
        .map_err(|error| EnvironmentStorageError::new("serialize", error.to_string()))?;
    let temporary_path = temporary_path(&path);

    fs::write(&temporary_path, content)
        .map_err(|error| EnvironmentStorageError::new("write", error.to_string()))?;

    if let Err(error) = fs::rename(&temporary_path, &path) {
        let _ = fs::remove_file(&temporary_path);
        return Err(EnvironmentStorageError::new("rename", error.to_string()));
    }

    Ok(())
}

pub fn load_environment(path: &str) -> Result<EnvironmentFile, EnvironmentStorageError> {
    let path = validate_path(path)?;
    let metadata = fs::metadata(&path)
        .map_err(|error| EnvironmentStorageError::new("metadata", error.to_string()))?;

    if metadata.len() > MAX_ENVIRONMENT_BYTES {
        return Err(EnvironmentStorageError::new(
            "environment_too_large",
            "O environment excede o limite local de 1 MiB.",
        ));
    }

    let content = fs::read_to_string(&path)
        .map_err(|error| EnvironmentStorageError::new("read", error.to_string()))?;
    let environment: EnvironmentFile = serde_yaml::from_str(&content)
        .map_err(|error| EnvironmentStorageError::new("parse", error.to_string()))?;

    environment.validate().map_err(validation_error)?;
    Ok(environment)
}

fn validate_path(path: &str) -> Result<PathBuf, EnvironmentStorageError> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(EnvironmentStorageError::new(
            "invalid_path",
            "O caminho do environment não pode estar vazio.",
        ));
    }

    let path = PathBuf::from(trimmed);
    if path.extension().and_then(|extension| extension.to_str()) != Some("yaml") {
        return Err(EnvironmentStorageError::new(
            "invalid_path",
            "Environments devem usar a extensão .yaml.",
        ));
    }

    if path.file_name().is_none() {
        return Err(EnvironmentStorageError::new(
            "invalid_path",
            "O caminho precisa apontar para um arquivo.",
        ));
    }

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            return Err(EnvironmentStorageError::new(
                "missing_parent",
                format!("A pasta pai não existe: {}", parent.display()),
            ));
        }
    }

    Ok(path)
}

fn temporary_path(path: &Path) -> PathBuf {
    let mut temporary_path = path.to_path_buf();
    temporary_path.set_extension("yaml.tmp");
    temporary_path
}

fn validation_error(error: EnvironmentValidationError) -> EnvironmentStorageError {
    let message = match error {
        EnvironmentValidationError::UnsupportedSchemaVersion {
            received,
            supported,
        } => format!("schemaVersion {received} não é suportado; esperado {supported}."),
        EnvironmentValidationError::EmptyName => {
            "O nome do environment não pode estar vazio.".to_string()
        }
        EnvironmentValidationError::EmptyVariableName => {
            "Toda variável precisa de um nome.".to_string()
        }
        EnvironmentValidationError::DuplicateVariableName { name } => {
            format!("A variável está duplicada: {name}.")
        }
        EnvironmentValidationError::ValueAndSecretReference { name } => {
            format!(
                "A variável {name} não pode ter valor público e referência secreta ao mesmo tempo."
            )
        }
        EnvironmentValidationError::MissingValue { name } => {
            format!("A variável {name} precisa de um valor ou referência secreta.")
        }
    };

    EnvironmentStorageError::new("invalid_environment", message)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::domain::environment::{EnvironmentVariable, CURRENT_ENVIRONMENT_SCHEMA_VERSION};

    fn environment() -> EnvironmentFile {
        EnvironmentFile {
            schema_version: CURRENT_ENVIRONMENT_SCHEMA_VERSION,
            name: "Fixture local".to_string(),
            variables: vec![EnvironmentVariable {
                name: "baseUrl".to_string(),
                value: Some("http://localhost:3000".to_string()),
                secret_ref: None,
            }],
        }
    }

    #[test]
    fn salva_e_carrega_environment_yaml() {
        let path = std::env::temp_dir().join(format!(
            "larry-test-{}-environment.yaml",
            std::process::id()
        ));
        let path_string = path.to_string_lossy().to_string();

        save_environment(&path_string, &environment()).unwrap();
        let loaded = load_environment(&path_string).unwrap();

        assert_eq!(loaded.name, "Fixture local");
        assert_eq!(
            loaded.variables[0].value.as_deref(),
            Some("http://localhost:3000")
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn rejeita_environment_yaml_invalido() {
        let path = std::env::temp_dir().join(format!(
            "larry-test-{}-invalid-environment.yaml",
            std::process::id()
        ));
        fs::write(&path, "schemaVersion: [incompleto").unwrap();

        let error = load_environment(path.to_str().unwrap()).unwrap_err();

        assert_eq!(error.kind, "parse");
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn rejeita_environment_ausente() {
        let path = std::env::temp_dir().join(format!(
            "larry-test-{}-missing-environment.yaml",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);

        let error = load_environment(path.to_str().unwrap()).unwrap_err();

        assert_eq!(error.kind, "metadata");
    }

    #[test]
    fn rejeita_environment_acima_do_limite() {
        let path = std::env::temp_dir().join(format!(
            "larry-test-{}-large-environment.yaml",
            std::process::id()
        ));
        fs::write(&path, vec![b'x'; MAX_ENVIRONMENT_BYTES as usize + 1]).unwrap();

        let error = load_environment(path.to_str().unwrap()).unwrap_err();

        assert_eq!(error.kind, "environment_too_large");
        fs::remove_file(path).unwrap();
    }
}
