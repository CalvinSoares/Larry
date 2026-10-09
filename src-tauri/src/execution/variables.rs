use serde::Serialize;
use serde_json::Value;

use crate::domain::environment::{EnvironmentFile, EnvironmentVariable};
use crate::domain::request::{
    AssertionDefinition, FormField, RequestAuth, RequestBody, RequestDefinition,
};
use crate::secrets::keyring_store::get_secret;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VariableResolutionError {
    pub kind: String,
    pub message: String,
}

impl VariableResolutionError {
    fn new(kind: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug)]
pub struct ResolvedRequest {
    pub request: RequestDefinition,
    pub redactions: Vec<String>,
}

pub fn resolve_request(
    mut request: RequestDefinition,
    environment: Option<&EnvironmentFile>,
) -> Result<ResolvedRequest, VariableResolutionError> {
    let Some(environment) = environment else {
        return Ok(ResolvedRequest {
            request,
            redactions: Vec::new(),
        });
    };

    environment.validate().map_err(|error| {
        VariableResolutionError::new("invalid_environment", format!("{error:?}"))
    })?;

    let mut redactions = Vec::new();
    request.url = resolve_string(&request.url, environment, &mut redactions)?;

    for query in &mut request.query {
        query.name = resolve_string(&query.name, environment, &mut redactions)?;
        query.value = resolve_string(&query.value, environment, &mut redactions)?;
    }

    for header in &mut request.headers {
        header.name = resolve_string(&header.name, environment, &mut redactions)?;
        header.value = resolve_string(&header.value, environment, &mut redactions)?;
    }

    for cookie in &mut request.cookies {
        cookie.name = resolve_string(&cookie.name, environment, &mut redactions)?;
        cookie.value = resolve_string(&cookie.value, environment, &mut redactions)?;
    }

    if let Some(body) = &mut request.body {
        match body {
            RequestBody::Json(value) => resolve_json(value, environment, &mut redactions)?,
            RequestBody::Text(value) => {
                *value = resolve_string(value, environment, &mut redactions)?;
            }
            RequestBody::FormUrlEncoded(fields) => {
                resolve_form_fields(fields, environment, &mut redactions)?;
            }
            RequestBody::Multipart(multipart) => {
                resolve_form_fields(&mut multipart.fields, environment, &mut redactions)?;
                for file in &mut multipart.files {
                    file.name = resolve_string(&file.name, environment, &mut redactions)?;
                    file.path = resolve_string(&file.path, environment, &mut redactions)?;
                }
            }
        }
    }

    if let Some(auth) = &mut request.auth {
        match auth {
            RequestAuth::Bearer { token } => {
                *token = resolve_string(token, environment, &mut redactions)?;
            }
            RequestAuth::Basic { username, password } => {
                *username = resolve_string(username, environment, &mut redactions)?;
                *password = resolve_string(password, environment, &mut redactions)?;
            }
            RequestAuth::ApiKey { name, value, .. } => {
                *name = resolve_string(name, environment, &mut redactions)?;
                *value = resolve_string(value, environment, &mut redactions)?;
            }
        }
    }

    for assertion in &mut request.assertions {
        match assertion {
            AssertionDefinition::StatusEquals { .. } => {}
            AssertionDefinition::HeaderContains { name, value } => {
                *name = resolve_string(name, environment, &mut redactions)?;
                *value = resolve_string(value, environment, &mut redactions)?;
            }
            AssertionDefinition::BodyContains { value } => {
                *value = resolve_string(value, environment, &mut redactions)?;
            }
        }
    }

    Ok(ResolvedRequest {
        request,
        redactions,
    })
}

fn resolve_json(
    value: &mut Value,
    environment: &EnvironmentFile,
    redactions: &mut Vec<String>,
) -> Result<(), VariableResolutionError> {
    match value {
        Value::String(text) => {
            *text = resolve_string(text, environment, redactions)?;
        }
        Value::Array(values) => {
            for value in values {
                resolve_json(value, environment, redactions)?;
            }
        }
        Value::Object(values) => {
            for value in values.values_mut() {
                resolve_json(value, environment, redactions)?;
            }
        }
        _ => {}
    }

    Ok(())
}

fn resolve_form_fields(
    fields: &mut [FormField],
    environment: &EnvironmentFile,
    redactions: &mut Vec<String>,
) -> Result<(), VariableResolutionError> {
    for field in fields {
        field.name = resolve_string(&field.name, environment, redactions)?;
        field.value = resolve_string(&field.value, environment, redactions)?;
    }

    Ok(())
}

fn resolve_string(
    value: &str,
    environment: &EnvironmentFile,
    redactions: &mut Vec<String>,
) -> Result<String, VariableResolutionError> {
    let mut result = String::with_capacity(value.len());
    let mut cursor = 0;

    while let Some(relative_start) = value[cursor..].find("{{") {
        let start = cursor + relative_start;
        result.push_str(&value[cursor..start]);

        let Some(relative_end) = value[start + 2..].find("}}") else {
            result.push_str(&value[start..]);
            return Ok(result);
        };
        let end = start + 2 + relative_end;
        let placeholder = value[start + 2..end].trim();

        if placeholder.is_empty() {
            return Err(VariableResolutionError::new(
                "invalid_placeholder",
                "O placeholder {{}} não possui nome.",
            ));
        }

        let resolved = resolve_placeholder(placeholder, environment, redactions)?;
        result.push_str(&resolved);
        cursor = end + 2;
    }

    result.push_str(&value[cursor..]);
    Ok(result)
}

fn resolve_placeholder(
    placeholder: &str,
    environment: &EnvironmentFile,
    redactions: &mut Vec<String>,
) -> Result<String, VariableResolutionError> {
    let normalized = placeholder.to_ascii_lowercase();
    let secret_requested = normalized.strip_prefix("secret.");
    let variable_name = secret_requested.unwrap_or(&normalized);
    let variable = environment
        .variables
        .iter()
        .find(|variable| variable.name.eq_ignore_ascii_case(variable_name));

    if let Some(variable) = variable {
        return resolve_variable(variable, placeholder, redactions);
    }

    if secret_requested.is_some() {
        let value = get_secret(variable_name).map_err(|error| {
            VariableResolutionError::new(
                error.kind,
                format!("Secret {variable_name} indisponível."),
            )
        })?;
        add_redaction(redactions, &value);
        return Ok(value);
    }

    Err(VariableResolutionError::new(
        "missing_variable",
        format!("A variável {variable_name} não existe no environment ativo."),
    ))
}

fn resolve_variable(
    variable: &EnvironmentVariable,
    placeholder: &str,
    redactions: &mut Vec<String>,
) -> Result<String, VariableResolutionError> {
    if let Some(value) = &variable.value {
        return Ok(value.clone());
    }

    let Some(secret_ref) = &variable.secret_ref else {
        return Err(VariableResolutionError::new(
            "invalid_variable",
            format!("A variável {placeholder} não possui valor nem referência secreta."),
        ));
    };

    let value = get_secret(secret_ref).map_err(|error| {
        VariableResolutionError::new(error.kind, format!("Secret {secret_ref} indisponível."))
    })?;
    add_redaction(redactions, &value);
    Ok(value)
}

fn add_redaction(redactions: &mut Vec<String>, value: &str) {
    if !value.is_empty() && !redactions.iter().any(|item| item == value) {
        redactions.push(value.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::environment::{
        EnvironmentFile, EnvironmentVariable, CURRENT_ENVIRONMENT_SCHEMA_VERSION,
    };
    use crate::domain::request::{HttpMethod, QueryParam};

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
    fn resolve_variavel_publica_em_url_e_query() {
        let request = RequestDefinition {
            id: "health".to_string(),
            name: "Health".to_string(),
            method: HttpMethod::Get,
            url: "{{baseUrl}}/health".to_string(),
            query: vec![QueryParam {
                name: "env".to_string(),
                value: "{{baseUrl}}".to_string(),
                enabled: true,
            }],
            headers: vec![],
            cookies: vec![],
            body: None,
            auth: None,
            assertions: vec![],
        };

        let resolved = resolve_request(request, Some(&environment())).unwrap();

        assert_eq!(resolved.request.url, "http://localhost:3000/health");
        assert_eq!(resolved.request.query[0].value, "http://localhost:3000");
        assert!(resolved.redactions.is_empty());
    }

    #[test]
    fn rejeita_placeholder_sem_variavel() {
        let request = RequestDefinition {
            id: "health".to_string(),
            name: "Health".to_string(),
            method: HttpMethod::Get,
            url: "{{missing}}/health".to_string(),
            query: vec![],
            headers: vec![],
            cookies: vec![],
            body: None,
            auth: None,
            assertions: vec![],
        };

        let error = resolve_request(request, Some(&environment())).unwrap_err();

        assert_eq!(error.kind, "missing_variable");
    }

    #[test]
    fn resolve_variavel_em_bearer_auth() {
        let mut test_environment = environment();
        test_environment.variables.push(EnvironmentVariable {
            name: "token".to_string(),
            value: Some("local-token".to_string()),
            secret_ref: None,
        });

        let request = RequestDefinition {
            id: "health".to_string(),
            name: "Health".to_string(),
            method: HttpMethod::Get,
            url: "http://localhost:3000/health".to_string(),
            query: vec![],
            headers: vec![],
            cookies: vec![],
            body: None,
            auth: Some(RequestAuth::Bearer {
                token: "{{token}}".to_string(),
            }),
            assertions: vec![],
        };

        let resolved = resolve_request(request, Some(&test_environment)).unwrap();

        match resolved.request.auth {
            Some(RequestAuth::Bearer { token }) => assert_eq!(token, "local-token"),
            _ => panic!("auth não foi preservada como bearer"),
        }
        assert!(resolved.redactions.is_empty());
    }

    #[test]
    fn resolve_variavel_em_assertion() {
        let request = RequestDefinition {
            id: "health".to_string(),
            name: "Health".to_string(),
            method: HttpMethod::Get,
            url: "http://localhost:3000/health".to_string(),
            query: vec![],
            headers: vec![],
            cookies: vec![],
            body: None,
            auth: None,
            assertions: vec![AssertionDefinition::BodyContains {
                value: "{{baseUrl}}".to_string(),
            }],
        };

        let resolved = resolve_request(request, Some(&environment())).unwrap();

        assert_eq!(
            resolved.request.assertions[0],
            AssertionDefinition::BodyContains {
                value: "http://localhost:3000".to_string(),
            }
        );
    }
}
