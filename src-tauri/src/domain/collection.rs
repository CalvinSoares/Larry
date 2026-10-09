use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::request::RequestDefinition;

pub const CURRENT_COLLECTION_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionFolder {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub requests: Vec<RequestDefinition>,
    #[serde(default)]
    pub folders: Vec<CollectionFolder>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionFile {
    pub schema_version: u32,
    pub name: String,
    #[serde(default)]
    pub requests: Vec<RequestDefinition>,
    #[serde(default)]
    pub folders: Vec<CollectionFolder>,
}

impl CollectionFile {
    pub fn validate(&self) -> Result<(), CollectionValidationError> {
        if self.schema_version != CURRENT_COLLECTION_SCHEMA_VERSION {
            return Err(CollectionValidationError::UnsupportedSchemaVersion {
                received: self.schema_version,
                supported: CURRENT_COLLECTION_SCHEMA_VERSION,
            });
        }

        if self.name.trim().is_empty() {
            return Err(CollectionValidationError::EmptyName);
        }

        let mut ids = HashSet::new();
        for request in &self.requests {
            validate_request(request, &mut ids)?;
        }

        for folder in &self.folders {
            validate_folder(folder, &mut ids)?;
        }

        Ok(())
    }
}

fn validate_request(
    request: &RequestDefinition,
    ids: &mut HashSet<String>,
) -> Result<(), CollectionValidationError> {
    if request.id.trim().is_empty() {
        return Err(CollectionValidationError::EmptyRequestId);
    }

    if !ids.insert(request.id.clone()) {
        return Err(CollectionValidationError::DuplicateRequestId {
            id: request.id.clone(),
        });
    }

    Ok(())
}

fn validate_folder(
    folder: &CollectionFolder,
    ids: &mut HashSet<String>,
) -> Result<(), CollectionValidationError> {
    if folder.id.trim().is_empty() {
        return Err(CollectionValidationError::EmptyFolderId);
    }

    if folder.name.trim().is_empty() {
        return Err(CollectionValidationError::EmptyFolderName);
    }

    if !ids.insert(folder.id.clone()) {
        return Err(CollectionValidationError::DuplicateFolderId {
            id: folder.id.clone(),
        });
    }

    for request in &folder.requests {
        validate_request(request, ids)?;
    }

    for child in &folder.folders {
        validate_folder(child, ids)?;
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollectionValidationError {
    UnsupportedSchemaVersion { received: u32, supported: u32 },
    EmptyName,
    EmptyRequestId,
    DuplicateRequestId { id: String },
    EmptyFolderId,
    EmptyFolderName,
    DuplicateFolderId { id: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::request::{HttpMethod, RequestDefinition};

    fn request(id: &str) -> RequestDefinition {
        RequestDefinition {
            id: id.to_string(),
            name: "Health check".to_string(),
            method: HttpMethod::Get,
            url: "http://localhost:3000/health".to_string(),
            query: vec![],
            headers: vec![],
            cookies: vec![],
            body: None,
            auth: None,
            assertions: vec![],
        }
    }

    #[test]
    fn valida_collection_com_schema_atual() {
        let collection = CollectionFile {
            schema_version: CURRENT_COLLECTION_SCHEMA_VERSION,
            name: "Minha API".to_string(),
            requests: vec![request("health")],
            folders: vec![],
        };

        assert!(collection.validate().is_ok());
    }

    #[test]
    fn rejeita_schema_desconhecido() {
        let collection = CollectionFile {
            schema_version: 999,
            name: "Minha API".to_string(),
            requests: vec![],
            folders: vec![],
        };

        assert_eq!(
            collection.validate(),
            Err(CollectionValidationError::UnsupportedSchemaVersion {
                received: 999,
                supported: CURRENT_COLLECTION_SCHEMA_VERSION,
            })
        );
    }

    #[test]
    fn rejeita_ids_duplicados() {
        let collection = CollectionFile {
            schema_version: CURRENT_COLLECTION_SCHEMA_VERSION,
            name: "Minha API".to_string(),
            requests: vec![request("same"), request("same")],
            folders: vec![],
        };

        assert_eq!(
            collection.validate(),
            Err(CollectionValidationError::DuplicateRequestId {
                id: "same".to_string(),
            })
        );
    }

    #[test]
    fn valida_pasta_e_request_aninhada() {
        let collection = CollectionFile {
            schema_version: CURRENT_COLLECTION_SCHEMA_VERSION,
            name: "Minha API".to_string(),
            requests: vec![],
            folders: vec![CollectionFolder {
                id: "payments".to_string(),
                name: "Payments".to_string(),
                requests: vec![request("create-payment")],
                folders: vec![],
            }],
        };

        assert!(collection.validate().is_ok());
    }

    #[test]
    fn rejeita_pasta_sem_nome() {
        let collection = CollectionFile {
            schema_version: CURRENT_COLLECTION_SCHEMA_VERSION,
            name: "Minha API".to_string(),
            requests: vec![],
            folders: vec![CollectionFolder {
                id: "payments".to_string(),
                name: " ".to_string(),
                requests: vec![],
                folders: vec![],
            }],
        };

        assert_eq!(
            collection.validate(),
            Err(CollectionValidationError::EmptyFolderName)
        );
    }
}
