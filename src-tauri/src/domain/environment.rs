use std::collections::HashSet;

use serde::{Deserialize, Serialize};

pub const CURRENT_ENVIRONMENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentFile {
    pub schema_version: u32,
    pub name: String,
    pub variables: Vec<EnvironmentVariable>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentVariable {
    pub name: String,
    pub value: Option<String>,
    pub secret_ref: Option<String>,
}

impl EnvironmentFile {
    pub fn validate(&self) -> Result<(), EnvironmentValidationError> {
        if self.schema_version != CURRENT_ENVIRONMENT_SCHEMA_VERSION {
            return Err(EnvironmentValidationError::UnsupportedSchemaVersion {
                received: self.schema_version,
                supported: CURRENT_ENVIRONMENT_SCHEMA_VERSION,
            });
        }

        if self.name.trim().is_empty() {
            return Err(EnvironmentValidationError::EmptyName);
        }

        let mut names = HashSet::new();
        for variable in &self.variables {
            if variable.name.trim().is_empty() {
                return Err(EnvironmentValidationError::EmptyVariableName);
            }

            if !names.insert(variable.name.to_ascii_lowercase()) {
                return Err(EnvironmentValidationError::DuplicateVariableName {
                    name: variable.name.clone(),
                });
            }

            match (&variable.value, &variable.secret_ref) {
                (Some(_), Some(_)) => {
                    return Err(EnvironmentValidationError::ValueAndSecretReference {
                        name: variable.name.clone(),
                    });
                }
                (None, None) => {
                    return Err(EnvironmentValidationError::MissingValue {
                        name: variable.name.clone(),
                    });
                }
                _ => {}
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvironmentValidationError {
    UnsupportedSchemaVersion { received: u32, supported: u32 },
    EmptyName,
    EmptyVariableName,
    DuplicateVariableName { name: String },
    ValueAndSecretReference { name: String },
    MissingValue { name: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn environment() -> EnvironmentFile {
        EnvironmentFile {
            schema_version: CURRENT_ENVIRONMENT_SCHEMA_VERSION,
            name: "Local".to_string(),
            variables: vec![EnvironmentVariable {
                name: "baseUrl".to_string(),
                value: Some("http://localhost:3000".to_string()),
                secret_ref: None,
            }],
        }
    }

    #[test]
    fn aceita_variavel_publica() {
        assert!(environment().validate().is_ok());
    }

    #[test]
    fn rejeita_valor_publico_junto_com_referencia_secreta() {
        let mut environment = environment();
        environment.variables[0].secret_ref = Some("base-url".to_string());

        assert_eq!(
            environment.validate(),
            Err(EnvironmentValidationError::ValueAndSecretReference {
                name: "baseUrl".to_string(),
            })
        );
    }
}
