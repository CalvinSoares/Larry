use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

use crate::domain::collection::{CollectionFile, CURRENT_COLLECTION_SCHEMA_VERSION};
use crate::domain::request::{HeaderEntry, HttpMethod, QueryParam, RequestBody, RequestDefinition};

const MAX_POSTMAN_BYTES: u64 = 10 * 1024 * 1024;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PostmanImportPreview {
    pub source_name: String,
    pub collection: CollectionFile,
    pub request_count: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PostmanImportError {
    pub kind: String,
    pub message: String,
}

impl PostmanImportError {
    fn new(kind: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            message: message.into(),
        }
    }
}

pub fn preview_collection(path: &str) -> Result<PostmanImportPreview, PostmanImportError> {
    let path = validate_path(path)?;
    let metadata = fs::metadata(&path)
        .map_err(|error| PostmanImportError::new("metadata", error.to_string()))?;

    if metadata.len() > MAX_POSTMAN_BYTES {
        return Err(PostmanImportError::new(
            "file_too_large",
            format!(
                "A collection Postman excede o limite local de {} MiB.",
                MAX_POSTMAN_BYTES / 1024 / 1024
            ),
        ));
    }

    let content = fs::read_to_string(&path)
        .map_err(|error| PostmanImportError::new("read", error.to_string()))?;
    let document: Value = serde_json::from_str(&content)
        .map_err(|error| PostmanImportError::new("parse", error.to_string()))?;

    validate_schema(&document)?;

    let source_name = document
        .pointer("/info/name")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| file_stem(&path));

    let mut warnings = Vec::new();
    let mut requests = Vec::new();
    let mut folder_warning_emitted = false;
    let mut variable_warning_emitted = false;

    if document.pointer("/info/description").is_some() {
        warnings.push(
            "A descrição da collection não é suportada pelo modelo atual e não foi importada."
                .to_string(),
        );
    }

    if document
        .get("variable")
        .and_then(Value::as_array)
        .is_some_and(|variables| !variables.is_empty())
    {
        warnings.push("Variáveis da collection foram mantidas como placeholders e ainda precisam de um environment Larry.".to_string());
        variable_warning_emitted = true;
    }

    if document.get("event").is_some() {
        warnings.push(
            "Scripts e eventos da collection não são executados nem importados nesta versão."
                .to_string(),
        );
    }

    collect_items(
        document.get("item").and_then(Value::as_array),
        &[],
        &mut requests,
        &mut warnings,
        &mut folder_warning_emitted,
        &mut variable_warning_emitted,
    );
    deduplicate_request_ids(&mut requests);

    if requests.is_empty() {
        warnings
            .push("Nenhuma request executável foi encontrada na collection Postman.".to_string());
    }

    Ok(PostmanImportPreview {
        source_name: source_name.clone(),
        collection: CollectionFile {
            schema_version: CURRENT_COLLECTION_SCHEMA_VERSION,
            name: source_name,
            requests: requests.clone(),
            folders: vec![],
        },
        request_count: requests.len(),
        warnings,
    })
}

fn validate_path(path: &str) -> Result<PathBuf, PostmanImportError> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(PostmanImportError::new(
            "invalid_path",
            "Informe o caminho de um arquivo JSON do Postman.",
        ));
    }

    let path = PathBuf::from(trimmed);
    if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
        return Err(PostmanImportError::new(
            "invalid_path",
            "A importação Postman exige um arquivo com extensão .json.",
        ));
    }

    let metadata = fs::metadata(&path)
        .map_err(|error| PostmanImportError::new("metadata", error.to_string()))?;
    if !metadata.is_file() {
        return Err(PostmanImportError::new(
            "invalid_path",
            "O caminho informado precisa apontar para um arquivo.",
        ));
    }

    Ok(path)
}

fn validate_schema(document: &Value) -> Result<(), PostmanImportError> {
    let schema = document
        .pointer("/info/schema")
        .and_then(Value::as_str)
        .unwrap_or_default();

    if !schema.contains("collection/v2.1.0/collection.json") {
        return Err(PostmanImportError::new(
            "unsupported_schema",
            "Somente collections Postman v2.1 são suportadas nesta etapa.",
        ));
    }

    Ok(())
}

fn collect_items(
    items: Option<&Vec<Value>>,
    folder_path: &[String],
    requests: &mut Vec<RequestDefinition>,
    warnings: &mut Vec<String>,
    folder_warning_emitted: &mut bool,
    variable_warning_emitted: &mut bool,
) {
    let Some(items) = items else {
        return;
    };

    for (index, item) in items.iter().enumerate() {
        let name = item
            .get("name")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("Request importada");

        if let Some(nested_items) = item.get("item").and_then(Value::as_array) {
            let mut nested_path = folder_path.to_vec();
            nested_path.push(name.to_string());

            if !*folder_warning_emitted {
                warnings.push("Pastas Postman foram achatadas no nome das requests porque o schema Larry ainda não possui grupos.".to_string());
                *folder_warning_emitted = true;
            }

            collect_items(
                Some(nested_items),
                &nested_path,
                requests,
                warnings,
                folder_warning_emitted,
                variable_warning_emitted,
            );
            continue;
        }

        let Some(request_value) = item.get("request") else {
            warnings.push(format!(
                "O item {} não possui uma request executável e foi ignorado.",
                index + 1
            ));
            continue;
        };

        match convert_request(
            name,
            request_value,
            folder_path,
            warnings,
            variable_warning_emitted,
        ) {
            Ok(request) => requests.push(request),
            Err(message) => warnings.push(message),
        }
    }
}

fn convert_request(
    name: &str,
    request_value: &Value,
    folder_path: &[String],
    warnings: &mut Vec<String>,
    variable_warning_emitted: &mut bool,
) -> Result<RequestDefinition, String> {
    let method_text = request_value
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or("GET")
        .to_uppercase();

    let method = match method_text.as_str() {
        "GET" => HttpMethod::Get,
        "POST" => HttpMethod::Post,
        "PUT" => HttpMethod::Put,
        "PATCH" => HttpMethod::Patch,
        "DELETE" => HttpMethod::Delete,
        unsupported => {
            warnings.push(format!(
                "A request \"{name}\" usa o método {unsupported}; ela foi convertida para GET."
            ));
            HttpMethod::Get
        }
    };

    let (url, query, contains_variables) = convert_url(request_value.get("url"));
    if url.trim().is_empty() {
        return Err(format!(
            "A request \"{name}\" não possui URL e foi ignorada."
        ));
    }

    if contains_variables && !*variable_warning_emitted {
        warnings.push("A collection contém placeholders {{variavel}}; eles ainda precisam de um environment Larry antes da execução.".to_string());
        *variable_warning_emitted = true;
    }

    if request_value.get("auth").is_some() {
        warnings.push(format!(
            "A autenticação da request \"{name}\" não foi importada; revise-a antes de executar."
        ));
    }

    if request_value.get("event").is_some() {
        warnings.push(format!(
            "Scripts da request \"{name}\" não foram importados."
        ));
    }

    let headers = request_value
        .get("header")
        .and_then(Value::as_array)
        .map(|entries| convert_headers(entries, name, warnings))
        .unwrap_or_default();

    let body = convert_body(request_value.get("body"), name, warnings);
    let display_name = if folder_path.is_empty() {
        name.to_string()
    } else {
        format!("{} / {name}", folder_path.join(" / "))
    };

    Ok(RequestDefinition {
        id: format!("postman-{}", requestsafe_id(&display_name)),
        name: display_name,
        method,
        url,
        query,
        headers,
        cookies: Vec::new(),
        body,
        auth: None,
        assertions: vec![],
    })
}

fn convert_url(value: Option<&Value>) -> (String, Vec<QueryParam>, bool) {
    let Some(value) = value else {
        return (String::new(), Vec::new(), false);
    };

    let mut raw = value
        .as_str()
        .or_else(|| value.get("raw").and_then(Value::as_str))
        .unwrap_or_default()
        .to_string();

    if value.is_object() && value.get("query").is_some() {
        raw = strip_query_and_fragment(&raw);
    }

    let contains_variables = raw.contains("{{");

    let query = value
        .get("query")
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| {
                    let name = entry.get("key")?.as_str()?.to_string();
                    Some(QueryParam {
                        name,
                        value: entry
                            .get("value")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                        enabled: !entry
                            .get("disabled")
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    (raw, query, contains_variables)
}

fn strip_query_and_fragment(raw: &str) -> String {
    let without_fragment = raw.split_once('#').map(|(value, _)| value).unwrap_or(raw);

    without_fragment
        .split_once('?')
        .map(|(value, _)| value)
        .unwrap_or(without_fragment)
        .to_string()
}

fn deduplicate_request_ids(requests: &mut [RequestDefinition]) {
    let mut occurrences = HashMap::<String, usize>::new();

    for request in requests {
        let count = occurrences.entry(request.id.clone()).or_insert(0);
        *count += 1;

        if *count > 1 {
            request.id = format!("{}-{}", request.id, count);
        }
    }
}

fn convert_headers(
    entries: &[Value],
    request_name: &str,
    warnings: &mut Vec<String>,
) -> Vec<HeaderEntry> {
    entries
        .iter()
        .filter_map(|entry| {
            let name = entry.get("key").and_then(Value::as_str)?.to_string();
            let original_value = entry
                .get("value")
                .and_then(Value::as_str)
                .unwrap_or_default();

            let sensitive = is_sensitive_name(&name);
            let value = if sensitive {
                warnings.push(format!(
                    "O valor do header sensível {name} da request \"{request_name}\" foi substituído por uma referência."
                ));
                format!("{{{{secret.{}}}}}", secret_key(&name))
            } else {
                original_value.to_string()
            };

            Some(HeaderEntry {
                name,
                value,
                enabled: !entry
                    .get("disabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            })
        })
        .collect()
}

fn convert_body(
    value: Option<&Value>,
    request_name: &str,
    warnings: &mut Vec<String>,
) -> Option<RequestBody> {
    let body = value?;
    let mode = body.get("mode").and_then(Value::as_str).unwrap_or_default();

    match mode {
        "raw" => {
            let raw = body.get("raw").and_then(Value::as_str).unwrap_or_default();
            if raw.trim().is_empty() {
                return None;
            }

            if let Ok(json_value) = serde_json::from_str::<Value>(raw) {
                Some(RequestBody::Json(sanitize_json(
                    json_value,
                    warnings,
                    request_name,
                )))
            } else if looks_sensitive(raw) {
                warnings.push(format!(
                    "O body textual da request \"{request_name}\" parece conter um secret e foi substituído por uma referência."
                ));
                Some(RequestBody::Text("{{secret.postman_body}}".to_string()))
            } else {
                Some(RequestBody::Text(raw.to_string()))
            }
        }
        "urlencoded" | "formdata" => {
            warnings.push(format!(
                "O body {mode} da request \"{request_name}\" não foi convertido porque o editor Larry ainda não suporta esse formato."
            ));
            None
        }
        "graphql" => {
            warnings.push(format!(
                "O body GraphQL da request \"{request_name}\" foi importado como texto; variáveis GraphQL precisam de revisão."
            ));
            body.get("graphql")
                .and_then(Value::as_str)
                .map(|value| RequestBody::Text(value.to_string()))
        }
        unsupported if !unsupported.is_empty() => {
            warnings.push(format!(
                "O modo de body {unsupported} da request \"{request_name}\" não é suportado."
            ));
            None
        }
        _ => None,
    }
}

fn sanitize_json(value: Value, warnings: &mut Vec<String>, request_name: &str) -> Value {
    match value {
        Value::Object(mut object) => {
            for (key, child) in object.iter_mut() {
                if is_sensitive_name(key) && child.is_string() {
                    *child = Value::String(format!("{{{{secret.{}}}}}", secret_key(key)));
                    warnings.push(format!(
                        "O campo sensível {key} do body da request \"{request_name}\" foi substituído por uma referência."
                    ));
                } else {
                    let current = std::mem::take(child);
                    *child = sanitize_json(current, warnings, request_name);
                }
            }
            Value::Object(object)
        }
        Value::Array(values) => Value::Array(
            values
                .into_iter()
                .map(|value| sanitize_json(value, warnings, request_name))
                .collect(),
        ),
        other => other,
    }
}

fn is_sensitive_name(name: &str) -> bool {
    let normalized = name.to_ascii_lowercase().replace(['-', '_'], "");
    normalized.contains("authorization")
        || normalized.contains("cookie")
        || normalized.contains("token")
        || normalized.contains("secret")
        || normalized.contains("password")
        || normalized.contains("apikey")
}

fn looks_sensitive(value: &str) -> bool {
    let normalized = value.to_ascii_lowercase();
    normalized.contains("authorization:")
        || normalized.contains("bearer ")
        || normalized.contains("token=")
        || normalized.contains("password=")
}

fn secret_key(name: &str) -> String {
    name.to_ascii_lowercase().replace(['-', '_', ' '], ".")
}

fn requestsafe_id(name: &str) -> String {
    let mut id = String::new();

    for character in name.chars() {
        if character.is_ascii_alphanumeric() {
            id.push(character.to_ascii_lowercase());
        } else if !id.ends_with('-') {
            id.push('-');
        }
    }

    let id = id.trim_matches('-').to_string();
    if id.is_empty() {
        "request".to_string()
    } else {
        id
    }
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("Postman collection")
        .to_string()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::json;

    use super::*;

    #[test]
    fn importa_request_aninhada_e_sanitiza_dados_sensiveis() {
        let path = std::env::temp_dir().join(format!("larry-postman-{}.json", std::process::id()));
        let document = json!({
            "info": {
                "name": "Payments",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "variable": [{"key": "baseUrl", "value": "http://localhost:3000"}],
            "item": [{
                "name": "Payments",
                "item": [{
                    "name": "Create payment",
                    "request": {
                        "method": "POST",
                        "header": [{"key": "Authorization", "value": "Bearer real-token"}],
                        "url": {
                            "raw": "{{baseUrl}}/payments?duplicate=raw",
                            "query": [{"key": "duplicate", "value": "structured"}]
                        },
                        "body": {
                            "mode": "raw",
                            "raw": "{\"token\":\"real-token\",\"amount\":100}"
                        }
                    }
                }]
            }]
        });

        fs::write(&path, serde_json::to_vec(&document).unwrap()).unwrap();

        let preview = preview_collection(&path.to_string_lossy()).unwrap();

        assert_eq!(preview.collection.name, "Payments");
        assert_eq!(preview.request_count, 1);
        assert_eq!(
            preview.collection.requests[0].name,
            "Payments / Create payment"
        );
        assert_eq!(
            preview.collection.requests[0].headers[0].value,
            "{{secret.authorization}}"
        );
        assert_eq!(preview.collection.requests[0].url, "{{baseUrl}}/payments");
        assert_eq!(preview.collection.requests[0].query[0].value, "structured");
        assert!(preview
            .warnings
            .iter()
            .any(|warning| warning.contains("Pastas")));
        assert!(preview
            .warnings
            .iter()
            .any(|warning| warning.contains("Variáveis")));

        let body = preview.collection.requests[0].body.as_ref().unwrap();
        assert!(!serde_json::to_string(body).unwrap().contains("real-token"));

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn rejeita_schema_que_nao_e_v21() {
        let path =
            std::env::temp_dir().join(format!("larry-postman-invalid-{}.json", std::process::id()));
        fs::write(
            &path,
            r#"{"info":{"name":"Legacy","schema":"https://schema.getpostman.com/json/collection/v1.0.0/collection.json"}}"#,
        )
        .unwrap();

        let error = preview_collection(&path.to_string_lossy()).unwrap_err();

        assert_eq!(error.kind, "unsupported_schema");
        fs::remove_file(path).unwrap();
    }
}
