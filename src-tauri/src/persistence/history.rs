use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::request::{RequestBody, RequestDefinition};
use crate::execution::http::{ExecutionError, HttpResponse, ResponseHeader};

const MAX_HISTORY_BODY_BYTES: usize = 256 * 1024;
const DEFAULT_HISTORY_LIMIT: u32 = 50;
const MAX_HISTORY_LIMIT: u32 = 200;
static NEXT_HISTORY_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryError {
    pub kind: String,
    pub message: String,
}

impl HistoryError {
    fn new(kind: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistorySummary {
    pub id: String,
    pub request_id: String,
    pub request_name: String,
    pub method: String,
    pub url: String,
    pub environment_name: Option<String>,
    pub created_at_ms: i64,
    pub outcome: String,
    pub status: Option<u16>,
    pub status_text: Option<String>,
    pub duration_ms: Option<u64>,
    pub body_size: Option<usize>,
    pub body_truncated: bool,
    pub error_kind: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub summary: HistorySummary,
    pub request: RequestDefinition,
    pub response: Option<HttpResponse>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryComparison {
    pub left_id: String,
    pub right_id: String,
    pub status_changed: bool,
    pub headers_changed: bool,
    pub body_changed: bool,
    pub duration_delta_ms: Option<i64>,
    pub left_status: Option<u16>,
    pub right_status: Option<u16>,
    pub left_duration_ms: Option<u64>,
    pub right_duration_ms: Option<u64>,
    pub body_note: String,
}

pub fn record_execution(
    path: &str,
    request: &RequestDefinition,
    environment_name: Option<&str>,
    result: &Result<HttpResponse, ExecutionError>,
    redactions: &[String],
) -> Result<HistorySummary, HistoryError> {
    let connection = open_database(path)?;
    let created_at_ms = now_millis()?;
    let id = format!(
        "run-{created_at_ms}-{}-{}",
        std::process::id(),
        NEXT_HISTORY_ID.fetch_add(1, Ordering::Relaxed)
    );
    let sanitized_request = sanitize_request(request, redactions);
    let request_id = sanitized_request.id.clone();
    let request_name = sanitized_request.name.clone();
    let method = method_name(&sanitized_request);
    let url = sanitized_request.url.clone();
    let request_json = serialize_json(&sanitized_request)?;

    let (
        outcome,
        status,
        status_text,
        duration_ms,
        body_size,
        body_truncated,
        response_json,
        error_kind,
        error_message,
    ) = match result {
        Ok(response) => {
            let (sanitized_response, body_truncated) = sanitize_response(response, redactions);
            let response_json = serialize_json(&sanitized_response)?;
            (
                if response.status < 400 {
                    "success"
                } else {
                    "http_error"
                },
                Some(response.status as i64),
                Some(response.status_text.clone()),
                Some(response.duration_ms as i64),
                Some(response.body_size as i64),
                body_truncated,
                Some(response_json),
                None,
                None,
            )
        }
        Err(error) => (
            "failed",
            None,
            None,
            None,
            None,
            false,
            None,
            Some(error.kind.clone()),
            Some(error.message.clone()),
        ),
    };

    connection
        .execute(
            "INSERT INTO history_entries
             (id, request_id, request_name, method, url, environment_name, created_at_ms,
              outcome, status, status_text, duration_ms, body_size, body_truncated,
              request_json, response_json, error_kind, error_message)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                id,
                request_id,
                request_name,
                method,
                url,
                environment_name,
                created_at_ms,
                outcome,
                status,
                status_text,
                duration_ms,
                body_size,
                body_truncated as i64,
                request_json,
                response_json,
                error_kind,
                error_message,
            ],
        )
        .map_err(database_error)?;

    trim_history(&connection)?;

    Ok(HistorySummary {
        id,
        request_id: request_id.clone(),
        request_name: request_name.clone(),
        method: method.clone(),
        url: url.clone(),
        environment_name: environment_name.map(ToOwned::to_owned),
        created_at_ms,
        outcome: outcome.to_string(),
        status: status.map(|value| value as u16),
        status_text,
        duration_ms: duration_ms.map(|value| value as u64),
        body_size: body_size.map(|value| value as usize),
        body_truncated,
        error_kind,
    })
}

pub fn list_history(
    path: &str,
    requested_limit: Option<u32>,
) -> Result<Vec<HistorySummary>, HistoryError> {
    let connection = open_database(path)?;
    let limit = requested_limit
        .unwrap_or(DEFAULT_HISTORY_LIMIT)
        .clamp(1, MAX_HISTORY_LIMIT);
    let mut statement = connection
        .prepare(
            "SELECT id, request_id, request_name, method, url, environment_name,
                    created_at_ms, outcome, status, status_text, duration_ms,
                    body_size, body_truncated, error_kind
             FROM history_entries ORDER BY created_at_ms DESC LIMIT ?1",
        )
        .map_err(database_error)?;

    let rows = statement
        .query_map([limit], summary_from_row)
        .map_err(database_error)?;

    rows.map(|row| row.map_err(database_error))
        .collect::<Result<Vec<_>, _>>()
}

pub fn get_history_entry(path: &str, id: &str) -> Result<HistoryEntry, HistoryError> {
    let connection = open_database(path)?;
    let row = connection
        .query_row(
            "SELECT id, request_id, request_name, method, url, environment_name,
                    created_at_ms, outcome, status, status_text, duration_ms,
                    body_size, body_truncated, error_kind, request_json,
                    response_json, error_message
             FROM history_entries WHERE id = ?1",
            [id],
            full_entry_from_row,
        )
        .optional()
        .map_err(database_error)?
        .ok_or_else(|| {
            HistoryError::new("not_found", "A execução não existe no histórico local.")
        })?;

    Ok(row)
}

pub fn compare_history_entries(
    path: &str,
    left_id: &str,
    right_id: &str,
) -> Result<HistoryComparison, HistoryError> {
    let left = get_history_entry(path, left_id)?;
    let right = get_history_entry(path, right_id)?;
    let left_response = left.response.as_ref();
    let right_response = right.response.as_ref();

    let status_changed = left.summary.status != right.summary.status;
    let headers_changed = match (left_response, right_response) {
        (Some(left), Some(right)) => {
            headers_signature(&left.headers) != headers_signature(&right.headers)
        }
        (None, None) => false,
        _ => true,
    };
    let body_changed = match (left_response, right_response) {
        (Some(left), Some(right)) => left.body != right.body,
        (None, None) => false,
        _ => true,
    };
    let duration_delta_ms = match (left.summary.duration_ms, right.summary.duration_ms) {
        (Some(left), Some(right)) => Some(right as i64 - left as i64),
        _ => None,
    };
    let body_note = if left.summary.body_truncated || right.summary.body_truncated {
        "O body de uma ou das duas execuções foi omitido por exceder o limite do histórico."
    } else if body_changed {
        "Os bodies armazenados são diferentes."
    } else {
        "Os bodies armazenados são iguais."
    };

    Ok(HistoryComparison {
        left_id: left.summary.id,
        right_id: right.summary.id,
        status_changed,
        headers_changed,
        body_changed,
        duration_delta_ms,
        left_status: left.summary.status,
        right_status: right.summary.status,
        left_duration_ms: left.summary.duration_ms,
        right_duration_ms: right.summary.duration_ms,
        body_note: body_note.to_string(),
    })
}

fn open_database(path: &str) -> Result<Connection, HistoryError> {
    let connection = Connection::open(path).map_err(database_error)?;
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY
            );
             CREATE TABLE IF NOT EXISTS history_entries (
                id TEXT PRIMARY KEY,
                request_id TEXT NOT NULL,
                request_name TEXT NOT NULL,
                method TEXT NOT NULL,
                url TEXT NOT NULL,
                environment_name TEXT,
                created_at_ms INTEGER NOT NULL,
                outcome TEXT NOT NULL,
                status INTEGER,
                status_text TEXT,
                duration_ms INTEGER,
                body_size INTEGER,
                body_truncated INTEGER NOT NULL,
                request_json TEXT NOT NULL,
                response_json TEXT,
                error_kind TEXT,
                error_message TEXT
             );
             CREATE INDEX IF NOT EXISTS idx_history_created_at
                ON history_entries(created_at_ms DESC);",
        )
        .map_err(database_error)?;
    connection
        .execute(
            "INSERT OR IGNORE INTO schema_migrations(version) VALUES (1)",
            [],
        )
        .map_err(database_error)?;
    Ok(connection)
}

fn trim_history(connection: &Connection) -> Result<(), HistoryError> {
    connection
        .execute(
            "DELETE FROM history_entries WHERE id NOT IN (
                SELECT id FROM history_entries ORDER BY created_at_ms DESC LIMIT 1000
            )",
            [],
        )
        .map_err(database_error)?;
    Ok(())
}

fn summary_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<HistorySummary> {
    Ok(HistorySummary {
        id: row.get(0)?,
        request_id: row.get(1)?,
        request_name: row.get(2)?,
        method: row.get(3)?,
        url: row.get(4)?,
        environment_name: row.get(5)?,
        created_at_ms: row.get(6)?,
        outcome: row.get(7)?,
        status: row.get::<_, Option<i64>>(8)?.map(|value| value as u16),
        status_text: row.get(9)?,
        duration_ms: row.get::<_, Option<i64>>(10)?.map(|value| value as u64),
        body_size: row.get::<_, Option<i64>>(11)?.map(|value| value as usize),
        body_truncated: row.get::<_, i64>(12)? != 0,
        error_kind: row.get(13)?,
    })
}

fn full_entry_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<HistoryEntry> {
    let summary = summary_from_row(row)?;
    let request_json: String = row.get(14)?;
    let response_json: Option<String> = row.get(15)?;
    let error_message: Option<String> = row.get(16)?;

    let request = serde_json::from_str(&request_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(14, rusqlite::types::Type::Text, Box::new(error))
    })?;
    let response = response_json
        .map(|value| {
            serde_json::from_str(&value).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    15,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })
        })
        .transpose()?;

    Ok(HistoryEntry {
        summary,
        request,
        response,
        error_message,
    })
}

fn sanitize_request(request: &RequestDefinition, redactions: &[String]) -> RequestDefinition {
    let mut sanitized = request.clone();
    for header in &mut sanitized.headers {
        if is_sensitive_name(&header.name) {
            header.value = "<redacted>".to_string();
        } else {
            header.value = redact_known(header.value.clone(), redactions);
        }
    }
    for query in &mut sanitized.query {
        if is_sensitive_name(&query.name) {
            query.value = "<redacted>".to_string();
        } else {
            query.value = redact_known(query.value.clone(), redactions);
        }
    }
    if let Some(body) = &mut sanitized.body {
        sanitize_request_body(body, redactions);
    }
    sanitized
}

fn sanitize_request_body(body: &mut RequestBody, redactions: &[String]) {
    match body {
        RequestBody::Json(value) => sanitize_json(value, redactions),
        RequestBody::Text(value) => {
            *value = redact_known(value.clone(), redactions);
        }
    }
}

fn sanitize_json(value: &mut Value, redactions: &[String]) {
    match value {
        Value::Object(entries) => {
            for (key, value) in entries.iter_mut() {
                if is_sensitive_name(key) {
                    *value = Value::String("<redacted>".to_string());
                } else {
                    sanitize_json(value, redactions);
                }
            }
        }
        Value::Array(entries) => {
            for value in entries {
                sanitize_json(value, redactions);
            }
        }
        Value::String(value) => {
            *value = redact_known(value.clone(), redactions);
        }
        _ => {}
    }
}

fn sanitize_response(response: &HttpResponse, redactions: &[String]) -> (HttpResponse, bool) {
    let mut sanitized = response.clone();
    sanitized.body = redact_known(sanitized.body, redactions);
    for header in &mut sanitized.headers {
        if is_sensitive_name(&header.name) {
            header.value = "<redacted>".to_string();
        } else {
            header.value = redact_known(header.value.clone(), redactions);
        }
    }

    let body_truncated = sanitized.body.len() > MAX_HISTORY_BODY_BYTES;
    if body_truncated {
        sanitized.body.clear();
    }

    (sanitized, body_truncated)
}

fn redact_known(mut value: String, redactions: &[String]) -> String {
    for secret in redactions {
        if !secret.is_empty() {
            value = value.replace(secret, "<redacted>");
        }
    }
    value
}

fn is_sensitive_name(name: &str) -> bool {
    let normalized = name.to_ascii_lowercase().replace(['-', '_'], "");
    [
        "authorization",
        "proxyauthorization",
        "cookie",
        "setcookie",
        "apikey",
        "accesstoken",
        "refreshtoken",
        "password",
        "secret",
        "token",
    ]
    .iter()
    .any(|term| normalized == *term || normalized.contains(term))
}

fn headers_signature(headers: &[ResponseHeader]) -> BTreeMap<String, Vec<String>> {
    let mut signature = BTreeMap::new();
    for header in headers {
        signature
            .entry(header.name.to_ascii_lowercase())
            .or_insert_with(Vec::new)
            .push(header.value.clone());
    }
    signature
}

fn method_name(request: &RequestDefinition) -> String {
    serde_json::to_value(&request.method)
        .ok()
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| "UNKNOWN".to_string())
}

fn serialize_json<T: Serialize>(value: &T) -> Result<String, HistoryError> {
    serde_json::to_string(value).map_err(|error| HistoryError::new("serialize", error.to_string()))
}

fn now_millis() -> Result<i64, HistoryError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .map_err(|error| HistoryError::new("clock", error.to_string()))
}

fn database_error(error: rusqlite::Error) -> HistoryError {
    HistoryError::new("history_database", error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::request::{HttpMethod, RequestBody};

    fn request() -> RequestDefinition {
        RequestDefinition {
            id: "health".to_string(),
            name: "Health".to_string(),
            method: HttpMethod::Get,
            url: "http://localhost/health".to_string(),
            query: vec![],
            headers: vec![],
            body: Some(RequestBody::Json(serde_json::json!({
                "accessToken": "do-not-store"
            }))),
        }
    }

    fn response(status: u16, body: &str, duration_ms: u64) -> HttpResponse {
        HttpResponse {
            status,
            status_text: "OK".to_string(),
            headers: vec![ResponseHeader {
                name: "Content-Type".to_string(),
                value: "application/json".to_string(),
            }],
            body: body.to_string(),
            body_size: body.len(),
            duration_ms,
            trace: crate::execution::http::TraceInfo {
                phases: vec![],
                resolved_addresses: vec![],
                http_version: Some("HTTP/1.1".to_string()),
                connection_reuse: crate::execution::http::TraceAvailability {
                    value: None,
                    provenance: "unavailable".to_string(),
                    detail: "fixture".to_string(),
                },
                tls: crate::execution::http::TraceAvailability {
                    value: None,
                    provenance: "unavailable".to_string(),
                    detail: "fixture".to_string(),
                },
            },
            diagnostics: vec![],
        }
    }

    #[test]
    fn salva_e_compara_execucoes_sem_secret_no_historico() {
        let path =
            std::env::temp_dir().join(format!("larry-history-{}.sqlite", std::process::id()));
        let path_string = path.to_string_lossy().to_string();
        let result = Ok(response(200, "{\"ok\":true}", 20));

        let first = record_execution(
            &path_string,
            &request(),
            Some("Local"),
            &result,
            &["do-not-store".to_string()],
        )
        .unwrap();
        let second_result = Ok(response(201, "{\"ok\":false}", 35));
        let second = record_execution(
            &path_string,
            &request(),
            Some("Staging"),
            &second_result,
            &["do-not-store".to_string()],
        )
        .unwrap();

        let entry = get_history_entry(&path_string, &first.id).unwrap();
        let serialized = serde_json::to_string(&entry).unwrap();
        assert!(!serialized.contains("do-not-store"));
        let RequestBody::Json(body) = entry.request.body.as_ref().unwrap() else {
            panic!("o body do fixture deveria ser JSON");
        };
        assert_eq!(body["accessToken"], "<redacted>");

        let comparison = compare_history_entries(&path_string, &first.id, &second.id).unwrap();
        assert!(comparison.status_changed);
        assert!(comparison.headers_changed == false);
        assert!(comparison.body_changed);
        assert_eq!(comparison.duration_delta_ms, Some(15));

        std::fs::remove_file(path).unwrap();
    }
}
