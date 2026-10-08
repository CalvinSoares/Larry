use std::time::{Duration, Instant};

use reqwest::{header, Client, Method, Url};
use serde::{Deserialize, Serialize};
use tokio::net::lookup_host;
use tokio::time::timeout;

use crate::domain::request::{HttpMethod, RequestBody, RequestDefinition};

const MAX_RESPONSE_BYTES: usize = 10 * 1024 * 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<ResponseHeader>,
    pub body: String,
    pub body_size: usize,
    pub duration_ms: u64,
    pub trace: TraceInfo,
    pub diagnostics: Vec<DiagnosticEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceInfo {
    pub phases: Vec<TracePhase>,
    pub resolved_addresses: Vec<String>,
    pub http_version: Option<String>,
    pub connection_reuse: TraceAvailability,
    pub tls: TraceAvailability,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TracePhase {
    pub name: String,
    pub duration_ms: Option<u64>,
    pub provenance: String,
    pub detail: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceAvailability {
    pub value: Option<String>,
    pub provenance: String,
    pub detail: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticEntry {
    pub layer: String,
    pub kind: String,
    pub summary: String,
    pub technical: String,
    pub provenance: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionError {
    pub kind: String,
    pub message: String,
    pub diagnostic: Option<DiagnosticEntry>,
}

impl ExecutionError {
    fn new(kind: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            message: message.into(),
            diagnostic: None,
        }
    }

    fn with_diagnostic(mut self, diagnostic: DiagnosticEntry) -> Self {
        self.diagnostic = Some(diagnostic);
        self
    }
}

#[allow(dead_code)]
pub async fn execute(request: RequestDefinition) -> Result<HttpResponse, ExecutionError> {
    execute_with_redactions(request, &[]).await
}

pub async fn execute_with_redactions(
    request: RequestDefinition,
    redactions: &[String],
) -> Result<HttpResponse, ExecutionError> {
    let parsed_url =
        validate_request(&request).map_err(|error| redact_execution_error(error, redactions))?;
    let trace_started = Instant::now();
    let dns = resolve_dns(&parsed_url, redactions).await?;
    let method = map_method(&request.method);
    let client = Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|error| ExecutionError::new("client", error.to_string()))?;

    let mut builder = client.request(method, parsed_url);

    let query_params: Vec<(&str, &str)> = request
        .query
        .iter()
        .filter(|param| param.enabled && !param.name.trim().is_empty())
        .map(|param| (param.name.as_str(), param.value.as_str()))
        .collect();

    if !query_params.is_empty() {
        builder = builder.query(&query_params);
    }

    for header_entry in request
        .headers
        .iter()
        .filter(|entry| entry.enabled && !entry.name.trim().is_empty())
    {
        let name = header::HeaderName::from_bytes(header_entry.name.as_bytes())
            .map_err(|error| ExecutionError::new("invalid_header_name", error.to_string()))?;
        let value = header::HeaderValue::from_str(&header_entry.value)
            .map_err(|error| ExecutionError::new("invalid_header_value", error.to_string()))?;

        builder = builder.header(name, value);
    }

    builder = match request.body {
        Some(RequestBody::Json(value)) => builder.json(&value),
        Some(RequestBody::Text(value)) => builder.body(value),
        None => builder,
    };

    let request_started = Instant::now();
    let response = builder
        .send()
        .await
        .map_err(|error| classify_request_error(error, redactions))?;
    let headers_received_at = Instant::now();
    let ttfb_ms = request_started.elapsed().as_millis() as u64;
    let status = response.status();
    let status_text = status.canonical_reason().unwrap_or_default().to_string();
    let http_version = format_http_version(response.version());
    let headers = response
        .headers()
        .iter()
        .map(|(name, value)| ResponseHeader {
            name: name.to_string(),
            value: value.to_str().unwrap_or("<non-utf8>").to_string(),
        })
        .collect();

    let body_bytes = read_response_body(response).await?;
    let body_size = body_bytes.len();
    let body = String::from_utf8_lossy(&body_bytes).into_owned();
    let download_ms = headers_received_at.elapsed().as_millis() as u64;
    let total_ms = trace_started.elapsed().as_millis() as u64;

    Ok(HttpResponse {
        status: status.as_u16(),
        status_text: status_text.clone(),
        headers,
        body,
        body_size,
        duration_ms: total_ms,
        trace: TraceInfo {
            phases: vec![
                TracePhase {
                    name: "dns".to_string(),
                    duration_ms: Some(dns.duration_ms),
                    provenance: "measured".to_string(),
                    detail: "Consulta DNS de preflight antes do envio pelo reqwest.".to_string(),
                },
                TracePhase {
                    name: "tcp".to_string(),
                    duration_ms: None,
                    provenance: "unavailable".to_string(),
                    detail: "O adapter HTTP atual não expõe o intervalo exato da conexão TCP."
                        .to_string(),
                },
                TracePhase {
                    name: "tls".to_string(),
                    duration_ms: None,
                    provenance: "unavailable".to_string(),
                    detail:
                        "O adapter HTTP atual não expõe handshake, certificado ou cipher suite."
                            .to_string(),
                },
                TracePhase {
                    name: "ttfb".to_string(),
                    duration_ms: Some(ttfb_ms),
                    provenance: "measured".to_string(),
                    detail: "Tempo observado até os headers da resposta ficarem disponíveis."
                        .to_string(),
                },
                TracePhase {
                    name: "download".to_string(),
                    duration_ms: Some(download_ms),
                    provenance: "measured".to_string(),
                    detail: "Tempo observado para ler o body até o fim.".to_string(),
                },
                TracePhase {
                    name: "total".to_string(),
                    duration_ms: Some(total_ms),
                    provenance: "measured".to_string(),
                    detail: "Tempo monotônico desde o preflight DNS até o fim do body.".to_string(),
                },
            ],
            resolved_addresses: dns.addresses,
            http_version: Some(http_version),
            connection_reuse: TraceAvailability {
                value: None,
                provenance: "unavailable".to_string(),
                detail: "Cada execução cria um Client novo; reuso de conexão ainda não é medido."
                    .to_string(),
            },
            tls: TraceAvailability {
                value: None,
                provenance: "unavailable".to_string(),
                detail: "Metadados TLS detalhados serão adicionados com um adapter instrumentado."
                    .to_string(),
            },
        },
        diagnostics: vec![diagnostic_for_status(status.as_u16(), &status_text)],
    })
}

struct DnsResolution {
    duration_ms: u64,
    addresses: Vec<String>,
}

async fn resolve_dns(url: &Url, redactions: &[String]) -> Result<DnsResolution, ExecutionError> {
    let host = url.host_str().ok_or_else(|| {
        ExecutionError::new("dns", "A URL não possui host para resolver.").with_diagnostic(
            diagnostic(
                "dns",
                "missing_host",
                "Não foi possível iniciar a resolução DNS.",
                "A URL validada não contém um host.",
            ),
        )
    })?;
    let port = url.port_or_known_default().unwrap_or(443);
    let started_at = Instant::now();

    let resolution = timeout(REQUEST_TIMEOUT, lookup_host((host, port)))
        .await
        .map_err(|_| {
            ExecutionError::new("dns_timeout", "A resolução DNS excedeu o timeout local.")
                .with_diagnostic(diagnostic(
                    "dns",
                    "timeout",
                    "A resolução DNS excedeu o timeout local.",
                    "lookup_host atingiu o limite de 30 segundos.",
                ))
        })?
        .map_err(|error| {
            let technical = redact(error.to_string(), redactions);
            ExecutionError::new("dns", technical.clone()).with_diagnostic(diagnostic(
                "dns",
                "resolution_failed",
                "O host não pôde ser resolvido localmente.",
                &technical,
            ))
        })?;

    let addresses = resolution
        .map(|address| address.ip().to_string())
        .collect::<Vec<_>>();

    Ok(DnsResolution {
        duration_ms: started_at.elapsed().as_millis() as u64,
        addresses,
    })
}

fn format_http_version(version: reqwest::Version) -> String {
    match version {
        reqwest::Version::HTTP_09 => "HTTP/0.9",
        reqwest::Version::HTTP_10 => "HTTP/1.0",
        reqwest::Version::HTTP_11 => "HTTP/1.1",
        reqwest::Version::HTTP_2 => "HTTP/2",
        reqwest::Version::HTTP_3 => "HTTP/3",
        _ => "Desconhecido",
    }
    .to_string()
}

fn diagnostic_for_status(status: u16, status_text: &str) -> DiagnosticEntry {
    let (kind, summary) = match status {
        401 => ("authentication", "A aplicação recusou a autenticação."),
        403 => ("authorization", "A aplicação recusou a autorização."),
        404 => ("not_found", "A aplicação não encontrou o recurso."),
        500..=599 => ("server_error", "O servidor retornou um erro de aplicação."),
        400..=499 => (
            "client_error",
            "A aplicação retornou um erro de requisição.",
        ),
        _ => (
            "http_status",
            "A aplicação respondeu sem erro HTTP de cliente ou servidor.",
        ),
    };

    diagnostic(
        "application",
        kind,
        summary,
        &format!("HTTP {status} {status_text}"),
    )
}

fn diagnostic(layer: &str, kind: &str, summary: &str, technical: &str) -> DiagnosticEntry {
    DiagnosticEntry {
        layer: layer.to_string(),
        kind: kind.to_string(),
        summary: summary.to_string(),
        technical: technical.to_string(),
        provenance: "observed".to_string(),
    }
}

fn validate_request(request: &RequestDefinition) -> Result<Url, ExecutionError> {
    if request.url.trim().is_empty() {
        return Err(ExecutionError::new(
            "invalid_url",
            "A URL da requisição não pode estar vazia.",
        ));
    }

    let url = Url::parse(&request.url)
        .map_err(|error| ExecutionError::new("invalid_url", error.to_string()))?;

    if !matches!(url.scheme(), "http" | "https") {
        return Err(ExecutionError::new(
            "unsupported_scheme",
            "Apenas URLs HTTP e HTTPS são suportadas nesta etapa.",
        ));
    }

    if url.host_str().is_none() {
        return Err(ExecutionError::new(
            "invalid_url",
            "A URL precisa conter um host.",
        ));
    }

    Ok(url)
}

fn map_method(method: &HttpMethod) -> Method {
    match method {
        HttpMethod::Get => Method::GET,
        HttpMethod::Post => Method::POST,
        HttpMethod::Put => Method::PUT,
        HttpMethod::Patch => Method::PATCH,
        HttpMethod::Delete => Method::DELETE,
    }
}

fn classify_request_error(error: reqwest::Error, redactions: &[String]) -> ExecutionError {
    let kind = if error.is_timeout() {
        "timeout"
    } else if error.is_connect() {
        "connection"
    } else if error.is_request() {
        "request"
    } else {
        "transport"
    };

    let message = redact(error.to_string(), redactions);
    let layer = if kind == "request" {
        "http"
    } else {
        "transport"
    };
    ExecutionError::new(kind, message.clone()).with_diagnostic(diagnostic(
        layer,
        kind,
        "A execução falhou antes de produzir uma resposta HTTP.",
        &message,
    ))
}

fn redact(mut message: String, redactions: &[String]) -> String {
    for secret in redactions {
        if !secret.is_empty() {
            message = message.replace(secret, "<redacted>");
        }
    }

    message
}

fn redact_execution_error(mut error: ExecutionError, redactions: &[String]) -> ExecutionError {
    error.message = redact(error.message, redactions);

    if let Some(diagnostic) = &mut error.diagnostic {
        diagnostic.technical = redact(diagnostic.technical.clone(), redactions);
    }

    error
}

async fn read_response_body(response: reqwest::Response) -> Result<Vec<u8>, ExecutionError> {
    let mut body = Vec::new();
    let mut response = response;

    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| ExecutionError::new("response_body", error.to_string()))?
    {
        if body.len() + chunk.len() > MAX_RESPONSE_BYTES {
            return Err(ExecutionError::new(
                "response_too_large",
                format!(
                    "A resposta excede o limite local de {} MiB.",
                    MAX_RESPONSE_BYTES / 1024 / 1024
                ),
            ));
        }

        body.extend_from_slice(&chunk);
    }

    Ok(body)
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    use super::*;

    fn request(url: &str) -> RequestDefinition {
        RequestDefinition {
            id: "test-request".to_string(),
            name: "Teste".to_string(),
            method: HttpMethod::Get,
            url: url.to_string(),
            query: vec![],
            headers: vec![],
            body: None,
        }
    }

    #[test]
    fn aceita_http_e_https() {
        assert!(validate_request(&request("http://localhost:3000")).is_ok());
        assert!(validate_request(&request("https://example.com")).is_ok());
    }

    #[test]
    fn rejeita_esquema_que_nao_e_http() {
        let error = validate_request(&request("ftp://example.com")).unwrap_err();

        assert_eq!(error.kind, "unsupported_scheme");
    }

    #[test]
    fn rejeita_url_vazia() {
        let error = validate_request(&request(" ")).unwrap_err();

        assert_eq!(error.kind, "invalid_url");
    }

    #[test]
    fn executa_get_contra_fixture_http_local() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request_buffer = [0_u8; 4096];
            let _ = stream.read(&mut request_buffer);

            let response = concat!(
                "HTTP/1.1 200 OK\r\n",
                "Content-Type: text/plain\r\n",
                "Content-Length: 5\r\n",
                "Connection: close\r\n",
                "\r\n",
                "hello"
            );

            stream.write_all(response.as_bytes()).unwrap();
        });

        let result = tauri::async_runtime::block_on(execute(request(&format!(
            "http://127.0.0.1:{port}/health"
        ))))
        .unwrap();

        server.join().unwrap();

        assert_eq!(result.status, 200);
        assert_eq!(result.body, "hello");
        assert_eq!(result.body_size, 5);
        assert_eq!(result.trace.http_version.as_deref(), Some("HTTP/1.1"));
        assert!(result
            .trace
            .resolved_addresses
            .iter()
            .any(|address| address == "127.0.0.1"));
        assert!(result.trace.phases.iter().any(|phase| {
            phase.name == "ttfb" && phase.duration_ms.is_some() && phase.provenance == "measured"
        }));
        assert_eq!(result.diagnostics[0].layer, "application");
    }

    #[test]
    fn classifica_status_http_de_aplicacao() {
        let diagnostic = diagnostic_for_status(401, "Unauthorized");

        assert_eq!(diagnostic.layer, "application");
        assert_eq!(diagnostic.kind, "authentication");
        assert_eq!(diagnostic.provenance, "observed");
    }
}
