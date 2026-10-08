use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::request::RequestDefinition;

pub const CURRENT_COLLECTION_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionFile {
    pub schema_version: u32,
    pub name: String,
    pub requests: Vec<RequestDefinition>,
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
            if request.id.trim().is_empty() {
                return Err(CollectionValidationError::EmptyRequestId);
            }

            if !ids.insert(&request.id) {
                return Err(CollectionValidationError::DuplicateRequestId {
                    id: request.id.clone(),
                });
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollectionValidationError {
    UnsupportedSchemaVersion { received: u32, supported: u32 },
    EmptyName,
    EmptyRequestId,
    DuplicateRequestId { id: String },
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
            body: None,
        }
    }

    #[test]
    fn valida_collection_com_schema_atual() {
        let collection = CollectionFile {
            schema_version: CURRENT_COLLECTION_SCHEMA_VERSION,
            name: "Minha API".to_string(),
            requests: vec![request("health")],
        };

        assert!(collection.validate().is_ok());
    }

    #[test]
    fn rejeita_schema_desconhecido() {
        let collection = CollectionFile {
            schema_version: 999,
            name: "Minha API".to_string(),
            requests: vec![],
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
        };

        assert_eq!(
            collection.validate(),
            Err(CollectionValidationError::DuplicateRequestId {
                id: "same".to_string(),
            })
        );
    }
}
