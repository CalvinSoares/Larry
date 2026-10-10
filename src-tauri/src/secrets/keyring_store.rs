use serde::Serialize;

const SERVICE_NAME: &str = "com.larry.api-client";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretStoreError {
    pub kind: String,
    pub message: String,
}

impl SecretStoreError {
    fn new(kind: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            message: message.into(),
        }
    }
}

pub fn set_secret(secret_ref: &str, value: &str) -> Result<(), SecretStoreError> {
    validate_secret_ref(secret_ref)?;
    if value.is_empty() {
        return Err(SecretStoreError::new(
            "empty_secret",
            "O secret não pode estar vazio.",
        ));
    }

    let entry = keyring::Entry::new(SERVICE_NAME, secret_ref)
        .map_err(|error| SecretStoreError::new("credential_store", error.to_string()))?;
    entry
        .set_password(value)
        .map_err(|error| SecretStoreError::new("credential_store", error.to_string()))
}

pub fn get_secret(secret_ref: &str) -> Result<String, SecretStoreError> {
    validate_secret_ref(secret_ref)?;

    let entry = keyring::Entry::new(SERVICE_NAME, secret_ref)
        .map_err(|error| SecretStoreError::new("credential_store", error.to_string()))?;
    entry.get_password().map_err(|error| {
        SecretStoreError::new(
            keyring_error_kind(&error),
            "Não foi possível obter o secret do armazenamento seguro.",
        )
    })
}

pub fn delete_secret(secret_ref: &str) -> Result<(), SecretStoreError> {
    validate_secret_ref(secret_ref)?;

    let entry = keyring::Entry::new(SERVICE_NAME, secret_ref)
        .map_err(|error| SecretStoreError::new("credential_store", error.to_string()))?;
    entry
        .delete_credential()
        .map_err(|error| SecretStoreError::new("credential_store", error.to_string()))
}

fn validate_secret_ref(secret_ref: &str) -> Result<(), SecretStoreError> {
    let trimmed = secret_ref.trim();
    if trimmed.is_empty() {
        return Err(SecretStoreError::new(
            "invalid_secret_ref",
            "A referência do secret não pode estar vazia.",
        ));
    }

    if trimmed.len() > 160 || trimmed.chars().any(char::is_control) {
        return Err(SecretStoreError::new(
            "invalid_secret_ref",
            "A referência do secret excede o limite ou contém caracteres inválidos.",
        ));
    }

    Ok(())
}

fn keyring_error_kind(error: &keyring::Error) -> &'static str {
    if matches!(error, keyring::Error::NoEntry) {
        "missing_secret"
    } else {
        "credential_store"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejeita_referencia_vazia() {
        let error = validate_secret_ref(" ").unwrap_err();
        assert_eq!(error.kind, "invalid_secret_ref");
    }

    #[test]
    fn rejeita_secret_vazio() {
        let error = set_secret("payments.token", "").unwrap_err();
        assert_eq!(error.kind, "empty_secret");
    }

    #[test]
    fn classifica_secret_ausente_sem_expor_valor() {
        let error = SecretStoreError::new(
            keyring_error_kind(&keyring::Error::NoEntry),
            "Não foi possível obter o secret do armazenamento seguro.",
        );

        assert_eq!(error.kind, "missing_secret");
        assert!(!error.message.contains("token"));
    }
}
