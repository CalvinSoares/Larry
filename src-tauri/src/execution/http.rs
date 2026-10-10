use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use reqwest::{header, Client, Method, Url};
use serde::{Deserialize, Serialize};
use tokio::net::lookup_host;
use tokio::sync::watch;
use tokio::time::timeout;

use crate::domain::request::{
    ApiKeyLocation, AssertionDefinition, CookieEntry, FormField, HttpMethod, MultipartBody,
    RequestAuth, RequestBody, RequestDefinition,
};

const MAX_RESPONSE_BYTES: usize = 10 * 1024 * 1024;
const MAX_UPLOAD_BYTES: u64 = 20 * 1024 * 1024;
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
    #[serde(default)]
    pub assertions: Vec<AssertionResult>,
    pub trace: TraceInfo,
    pub diagnostics: Vec<DiagnosticEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HttpProtocol {
    Auto,
    Http1,
    Http2,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpProtocolSample {
    pub protocol: HttpProtocol,
    pub status: Option<u16>,
    pub status_text: Option<String>,
    pub duration_ms: Option<u64>,
    pub body_size: Option<usize>,
    pub http_version: Option<String>,
    pub error: Option<ExecutionError>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpProtocolComparison {
    pub runs: Vec<HttpProtocolSample>,
    pub duration_delta_ms: Option<i64>,
    pub body_size_delta_bytes: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssertionResult {
    pub assertion_type: String,
    pub passed: bool,
    pub summary: String,
    pub expected: String,
    pub actual: String,
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

    fn cancelled() -> Self {
        Self::new("cancelled", "A requisição foi cancelada localmente.").with_diagnostic(
            diagnostic(
                "execution",
                "cancelled",
                "A execução foi interrompida antes de produzir uma resposta.",
                "O usuário sinalizou o cancelamento da operação local.",
            ),
        )
    }
}

#[derive(Clone, Default)]
pub struct RequestExecutionManager {
    runs: Arc<Mutex<HashMap<String, watch::Sender<bool>>>>,
}

impl RequestExecutionManager {
    pub fn begin(&self, run_id: String) -> Result<watch::Receiver<bool>, ExecutionError> {
        let (sender, receiver) = watch::channel(false);
        let mut runs = self.runs.lock().map_err(|_| {
            ExecutionError::new(
                "request_manager",
                "Não foi possível acessar o controle das requests em execução.",
            )
        })?;

        if runs.contains_key(&run_id) {
            return Err(ExecutionError::new(
                "request_already_running",
                "Já existe uma request local com este identificador em execução.",
            ));
        }

        runs.insert(run_id, sender);

        Ok(receiver)
    }

    pub fn cancel(&self, run_id: &str) -> Result<(), ExecutionError> {
        let sender = self
            .runs
            .lock()
            .map_err(|_| {
                ExecutionError::new(
                    "request_manager",
                    "Não foi possível acessar o controle das requests em execução.",
                )
            })?
            .get(run_id)
            .cloned()
            .ok_or_else(|| {
                ExecutionError::new(
                    "request_not_found",
                    "A request não está mais em execução ou já foi finalizada.",
                )
            })?;

        sender.send(true).map_err(|_| {
            ExecutionError::new(
                "request_cancel",
                "Não foi possível sinalizar o cancelamento da request.",
            )
        })
    }

    pub fn remove(&self, run_id: &str) {
        if let Ok(mut runs) = self.runs.lock() {
            runs.remove(run_id);
        }
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
    execute_with_protocol(request, redactions, HttpProtocol::Auto).await
}

pub async fn execute_with_cancellation(
    request: RequestDefinition,
    redactions: &[String],
    cancellation: watch::Receiver<bool>,
) -> Result<HttpResponse, ExecutionError> {
    tokio::select! {
        result = execute_with_redactions(request, redactions) => result,
        _ = wait_for_cancellation(cancellation) => Err(ExecutionError::cancelled()),
    }
}

async fn wait_for_cancellation(mut receiver: watch::Receiver<bool>) {
    if *receiver.borrow() {
        return;
    }

    let _ = receiver.changed().await;
}

pub async fn execute_with_protocol(
    request: RequestDefinition,
    redactions: &[String],
    protocol: HttpProtocol,
) -> Result<HttpResponse, ExecutionError> {
    execute_with_protocol_timeout(request, redactions, protocol, REQUEST_TIMEOUT).await
}

async fn execute_with_protocol_timeout(
    request: RequestDefinition,
    redactions: &[String],
    protocol: HttpProtocol,
    request_timeout: Duration,
) -> Result<HttpResponse, ExecutionError> {
    let parsed_url =
        validate_request(&request).map_err(|error| redact_execution_error(error, redactions))?;
    let trace_started = Instant::now();
    let dns = resolve_dns(&parsed_url, redactions, request_timeout).await?;
    let method = map_method(&request.method);
    let client_builder = Client::builder().timeout(request_timeout);
    let client_builder = match protocol {
        HttpProtocol::Auto => client_builder,
        HttpProtocol::Http1 => client_builder.http1_only(),
        HttpProtocol::Http2 => client_builder.http2_prior_knowledge(),
    };
    let client = client_builder
        .build()
        .map_err(|error| ExecutionError::new("client", error.to_string()))?;

    let mut builder = client.request(method, parsed_url);

    let mut query_params: Vec<(String, String)> = request
        .query
        .iter()
        .filter(|param| param.enabled && !param.name.trim().is_empty())
        .map(|param| (param.name.clone(), param.value.clone()))
        .collect();

    if let Some(auth) = request.auth.as_ref() {
        append_auth_query(&mut query_params, auth)?;
    }

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

    if let Some(cookie_header) = build_cookie_header(&request.cookies)? {
        builder = builder.header(header::COOKIE, cookie_header);
    }

    builder = apply_auth(builder, request.auth.as_ref())?;

    builder = match request.body {
        Some(RequestBody::Json(value)) => builder.json(&value),
        Some(RequestBody::Text(value)) => builder.body(value),
        Some(RequestBody::FormUrlEncoded(fields)) => builder.form(&build_form_fields(&fields)?),
        Some(RequestBody::Multipart(multipart)) => {
            builder.multipart(build_multipart_form(multipart).await?)
        }
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
    let headers: Vec<ResponseHeader> = response
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
    let assertions = evaluate_assertions(&request.assertions, status.as_u16(), &headers, &body);
    let download_ms = headers_received_at.elapsed().as_millis() as u64;
    let total_ms = trace_started.elapsed().as_millis() as u64;

    Ok(HttpResponse {
        status: status.as_u16(),
        status_text: status_text.clone(),
        headers,
        body,
        body_size,
        duration_ms: total_ms,
        assertions,
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

pub async fn compare_http_protocols(
    request: RequestDefinition,
    redactions: &[String],
) -> HttpProtocolComparison {
    let mut runs = Vec::with_capacity(2);

    for protocol in [HttpProtocol::Http1, HttpProtocol::Http2] {
        let sample_protocol = protocol.clone();
        let sample = match execute_with_protocol(request.clone(), redactions, protocol).await {
            Ok(response) => HttpProtocolSample {
                protocol: sample_protocol,
                status: Some(response.status),
                status_text: Some(response.status_text),
                duration_ms: Some(response.duration_ms),
                body_size: Some(response.body_size),
                http_version: response.trace.http_version,
                error: None,
            },
            Err(error) => HttpProtocolSample {
                protocol: sample_protocol,
                status: None,
                status_text: None,
                duration_ms: None,
                body_size: None,
                http_version: None,
                error: Some(error),
            },
        };

        runs.push(sample);
    }

    let duration_delta_ms = difference_between(
        runs.first().and_then(|run| run.duration_ms),
        runs.get(1).and_then(|run| run.duration_ms),
    );
    let body_size_delta_bytes = difference_between(
        runs.first()
            .and_then(|run| run.body_size.map(|size| size as u64)),
        runs.get(1)
            .and_then(|run| run.body_size.map(|size| size as u64)),
    );

    HttpProtocolComparison {
        runs,
        duration_delta_ms,
        body_size_delta_bytes,
    }
}

fn difference_between(left: Option<u64>, right: Option<u64>) -> Option<i64> {
    Some(i64::try_from(right?).ok()? - i64::try_from(left?).ok()?)
}

fn evaluate_assertions(
    definitions: &[AssertionDefinition],
    status: u16,
    headers: &[ResponseHeader],
    body: &str,
) -> Vec<AssertionResult> {
    definitions
        .iter()
        .map(|definition| match definition {
            AssertionDefinition::StatusEquals { expected } => AssertionResult {
                assertion_type: "statusEquals".to_string(),
                passed: status == *expected,
                summary: "Status HTTP".to_string(),
                expected: expected.to_string(),
                actual: status.to_string(),
            },
            AssertionDefinition::HeaderContains { name, value } => {
                let actual = headers
                    .iter()
                    .find(|header| header.name.eq_ignore_ascii_case(name))
                    .map(|header| header.value.clone());
                let passed = actual
                    .as_ref()
                    .is_some_and(|header_value| header_value.contains(value));

                AssertionResult {
                    assertion_type: "headerContains".to_string(),
                    passed,
                    summary: format!("Header {name}"),
                    expected: format!("contém {value}"),
                    actual: actual.unwrap_or_else(|| "header não encontrado".to_string()),
                }
            }
            AssertionDefinition::BodyContains { value } => AssertionResult {
                assertion_type: "bodyContains".to_string(),
                passed: body.contains(value),
                summary: "Conteúdo do body".to_string(),
                expected: format!("contém {value}"),
                actual: if body.contains(value) {
                    "texto encontrado".to_string()
                } else {
                    "texto não encontrado".to_string()
                },
            },
        })
        .collect()
}

fn apply_auth(
    builder: reqwest::RequestBuilder,
    auth: Option<&RequestAuth>,
) -> Result<reqwest::RequestBuilder, ExecutionError> {
    match auth {
        None => Ok(builder),
        Some(RequestAuth::Bearer { token }) => Ok(builder.bearer_auth(token)),
        Some(RequestAuth::Basic { username, password }) => {
            Ok(builder.basic_auth(username, Some(password)))
        }
        Some(RequestAuth::ApiKey {
            name,
            value,
            location: ApiKeyLocation::Header,
        }) => {
            let header_name = header::HeaderName::from_bytes(name.as_bytes())
                .map_err(|error| ExecutionError::new("invalid_auth_name", error.to_string()))?;
            let header_value = header::HeaderValue::from_str(value)
                .map_err(|error| ExecutionError::new("invalid_auth_value", error.to_string()))?;

            Ok(builder.header(header_name, header_value))
        }
        Some(RequestAuth::ApiKey {
            location: ApiKeyLocation::Query,
            ..
        }) => Ok(builder),
    }
}

fn append_auth_query(
    query_params: &mut Vec<(String, String)>,
    auth: &RequestAuth,
) -> Result<(), ExecutionError> {
    if let RequestAuth::ApiKey {
        name,
        value,
        location: ApiKeyLocation::Query,
    } = auth
    {
        validate_api_key_name(name)?;
        query_params.push((name.clone(), value.clone()));
    }

    Ok(())
}

fn validate_api_key_name(name: &str) -> Result<(), ExecutionError> {
    if name.trim().is_empty() {
        return Err(ExecutionError::new(
            "invalid_auth_name",
            "O nome da API key não pode ficar vazio.",
        ));
    }

    if header::HeaderName::from_bytes(name.as_bytes()).is_err()
        && name.chars().any(|character| character.is_control())
    {
        return Err(ExecutionError::new(
            "invalid_auth_name",
            "O nome da API key contém caracteres inválidos.",
        ));
    }

    Ok(())
}

fn build_cookie_header(
    cookies: &[CookieEntry],
) -> Result<Option<header::HeaderValue>, ExecutionError> {
    let mut pairs = Vec::new();

    for cookie in cookies
        .iter()
        .filter(|cookie| cookie.enabled && !cookie.name.trim().is_empty())
    {
        validate_cookie_name(&cookie.name)?;

        if cookie.value.contains(';') || cookie.value.contains('\r') || cookie.value.contains('\n')
        {
            return Err(ExecutionError::new(
                "invalid_cookie_value",
                "O valor do cookie contém caracteres inválidos.",
            ));
        }

        pairs.push(format!("{}={}", cookie.name.trim(), cookie.value));
    }

    if pairs.is_empty() {
        return Ok(None);
    }

    let value = pairs.join("; ");
    let header_value = header::HeaderValue::from_str(&value)
        .map_err(|error| ExecutionError::new("invalid_cookie_value", error.to_string()))?;

    Ok(Some(header_value))
}

fn validate_cookie_name(name: &str) -> Result<(), ExecutionError> {
    if header::HeaderName::from_bytes(name.trim().as_bytes()).is_err() {
        return Err(ExecutionError::new(
            "invalid_cookie_name",
            "O nome do cookie contém caracteres inválidos.",
        ));
    }

    Ok(())
}

fn build_form_fields(fields: &[FormField]) -> Result<Vec<(String, String)>, ExecutionError> {
    fields
        .iter()
        .filter(|field| field.enabled)
        .map(|field| {
            validate_form_field_name(&field.name)?;
            Ok((field.name.trim().to_string(), field.value.clone()))
        })
        .collect()
}

async fn build_multipart_form(
    body: MultipartBody,
) -> Result<reqwest::multipart::Form, ExecutionError> {
    let mut form = reqwest::multipart::Form::new();

    for (name, value) in build_form_fields(&body.fields)? {
        form = form.text(name, value);
    }

    for file in body.files.iter().filter(|file| file.enabled) {
        validate_form_field_name(&file.name)?;
        let path = file.path.trim();

        if path.is_empty() {
            return Err(ExecutionError::new(
                "invalid_file_path",
                "O caminho do arquivo multipart não pode ficar vazio.",
            ));
        }

        let file_path = Path::new(path);
        let metadata = tokio::fs::metadata(file_path).await.map_err(|error| {
            ExecutionError::new(
                "file_read",
                format!("Não foi possível acessar o arquivo multipart: {error}"),
            )
        })?;

        if !metadata.is_file() {
            return Err(ExecutionError::new(
                "file_read",
                "O caminho multipart selecionado não aponta para um arquivo.",
            ));
        }

        if metadata.len() > MAX_UPLOAD_BYTES {
            return Err(ExecutionError::new(
                "file_too_large",
                "O arquivo multipart excede o limite local de 20 MiB.",
            ));
        }

        let bytes = tokio::fs::read(file_path).await.map_err(|error| {
            ExecutionError::new(
                "file_read",
                format!("Não foi possível ler o arquivo multipart: {error}"),
            )
        })?;
        let file_name = file_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("upload.bin")
            .to_string();
        let part = reqwest::multipart::Part::bytes(bytes).file_name(file_name);

        form = form.part(file.name.trim().to_string(), part);
    }

    Ok(form)
}

fn validate_form_field_name(name: &str) -> Result<(), ExecutionError> {
    if name.trim().is_empty() || name.chars().any(|character| character.is_control()) {
        return Err(ExecutionError::new(
            "invalid_form_field_name",
            "O nome do campo de formulário não pode ficar vazio ou conter caracteres de controle.",
        ));
    }

    Ok(())
}

struct DnsResolution {
    duration_ms: u64,
    addresses: Vec<String>,
}

async fn resolve_dns(
    url: &Url,
    redactions: &[String],
    request_timeout: Duration,
) -> Result<DnsResolution, ExecutionError> {
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

    let resolution = timeout(request_timeout, lookup_host((host, port)))
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
    read_response_body_with_limit(response, MAX_RESPONSE_BYTES).await
}

async fn read_response_body_with_limit(
    response: reqwest::Response,
    max_bytes: usize,
) -> Result<Vec<u8>, ExecutionError> {
    let mut body = Vec::new();
    let mut response = response;

    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| ExecutionError::new("response_body", error.to_string()))?
    {
        if body.len() + chunk.len() > max_bytes {
            let limit = if max_bytes == MAX_RESPONSE_BYTES {
                format!("{} MiB", MAX_RESPONSE_BYTES / 1024 / 1024)
            } else {
                format!("{} bytes", max_bytes)
            };
            return Err(ExecutionError::new(
                "response_too_large",
                format!("A resposta excede o limite local de {limit}."),
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
    use std::sync::mpsc;
    use std::time::Duration;

    use super::*;
    use crate::domain::request::{MultipartFile, RequestAuth};

    fn request(url: &str) -> RequestDefinition {
        RequestDefinition {
            id: "test-request".to_string(),
            name: "Teste".to_string(),
            method: HttpMethod::Get,
            url: url.to_string(),
            query: vec![],
            headers: vec![],
            cookies: vec![],
            body: None,
            auth: None,
            assertions: vec![],
        }
    }

    fn spawn_response_server(
        status: u16,
        body: &'static str,
        delay: Duration,
    ) -> (String, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request_buffer = [0_u8; 4096];
            let _ = stream.read(&mut request_buffer);
            std::thread::sleep(delay);

            let reason = match status {
                200 => "OK",
                401 => "Unauthorized",
                403 => "Forbidden",
                500 => "Internal Server Error",
                _ => "Fixture Response",
            };
            let response = format!(
                "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
        });

        (format!("http://127.0.0.1:{port}/fixture"), server)
    }

    #[test]
    fn serializa_protocolos_de_comparacao() {
        assert_eq!(
            serde_json::to_string(&HttpProtocol::Auto).unwrap(),
            "\"auto\""
        );
        assert_eq!(
            serde_json::to_string(&HttpProtocol::Http1).unwrap(),
            "\"http1\""
        );
        assert_eq!(
            serde_json::to_string(&HttpProtocol::Http2).unwrap(),
            "\"http2\""
        );
    }

    #[test]
    fn bloqueia_identificador_de_request_duplicado() {
        let manager = RequestExecutionManager::default();

        assert!(manager.begin("run-1".to_string()).is_ok());

        let error = manager.begin("run-1".to_string()).unwrap_err();

        assert_eq!(error.kind, "request_already_running");
    }

    #[tokio::test]
    async fn cancela_request_antes_do_inicio_da_execucao() {
        let request = request("http://127.0.0.1:1/health");
        let (_sender, receiver) = tokio::sync::watch::channel(true);

        let error = execute_with_cancellation(request, &[], receiver)
            .await
            .unwrap_err();

        assert_eq!(error.kind, "cancelled");
        assert_eq!(error.diagnostic.unwrap().layer, "execution");
    }

    #[tokio::test]
    async fn cancela_request_durante_o_transporte() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (accepted_sender, accepted_receiver) = mpsc::channel();

        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            accepted_sender.send(()).unwrap();
            std::thread::sleep(Duration::from_millis(200));
            let _ = stream.write_all(
                concat!(
                    "HTTP/1.1 200 OK\r\n",
                    "Content-Length: 2\r\n",
                    "Connection: close\r\n",
                    "\r\n",
                    "ok"
                )
                .as_bytes(),
            );
        });

        let manager = RequestExecutionManager::default();
        let run_id = "run-transport".to_string();
        let cancellation = manager.begin(run_id.clone()).unwrap();
        let execution = tokio::spawn(execute_with_cancellation(
            request(&format!("http://127.0.0.1:{port}/slow")),
            &[],
            cancellation,
        ));

        tokio::task::spawn_blocking(move || {
            accepted_receiver
                .recv_timeout(Duration::from_secs(1))
                .expect("o fixture deve aceitar a conexão");
        })
        .await
        .unwrap();

        manager.cancel(&run_id).unwrap();
        let error = tokio::time::timeout(Duration::from_secs(1), execution)
            .await
            .expect("o cancelamento deve encerrar a execução")
            .unwrap()
            .unwrap_err();

        manager.remove(&run_id);
        server.join().unwrap();

        assert_eq!(error.kind, "cancelled");
    }

    #[tokio::test]
    async fn classifica_respostas_de_aplicacao_com_fixture_local() {
        for (status, expected_kind) in [
            (401, "authentication"),
            (403, "authorization"),
            (500, "server_error"),
        ] {
            let (url, server) = spawn_response_server(status, "failure", Duration::ZERO);
            let response = execute(request(&url)).await.unwrap();

            server.join().unwrap();

            assert_eq!(response.status, status);
            assert_eq!(response.diagnostics[0].layer, "application");
            assert_eq!(response.diagnostics[0].kind, expected_kind);
            assert_eq!(response.diagnostics[0].provenance, "observed");
        }
    }

    #[tokio::test]
    async fn classifica_conexao_recusada() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let error = execute(request(&format!("http://127.0.0.1:{port}/refused")))
            .await
            .unwrap_err();

        assert_eq!(error.kind, "connection");
        assert_eq!(error.diagnostic.unwrap().layer, "transport");
    }

    #[tokio::test]
    async fn classifica_timeout_com_limite_de_fixture() {
        let (url, server) = spawn_response_server(200, "slow", Duration::from_millis(200));
        let error = execute_with_protocol_timeout(
            request(&url),
            &[],
            HttpProtocol::Auto,
            Duration::from_millis(50),
        )
        .await
        .unwrap_err();

        server.join().unwrap();

        assert_eq!(error.kind, "timeout");
        assert_eq!(error.diagnostic.unwrap().layer, "transport");
    }

    #[tokio::test]
    async fn rejeita_response_acima_do_limite_configurado() {
        let (url, server) = spawn_response_server(200, "large", Duration::ZERO);
        let response = reqwest::Client::new().get(url).send().await.unwrap();

        let error = read_response_body_with_limit(response, 4)
            .await
            .unwrap_err();

        server.join().unwrap();

        assert_eq!(error.kind, "response_too_large");
        assert!(error.message.contains("4 bytes"));
    }

    #[tokio::test]
    async fn classifica_falha_de_dns_com_dominio_reservado() {
        let error = execute(request("http://larry-test.invalid/health"))
            .await
            .unwrap_err();

        assert!(matches!(error.kind.as_str(), "dns" | "dns_timeout"));
        assert_eq!(error.diagnostic.unwrap().layer, "dns");
    }

    #[test]
    fn calcula_delta_com_http2_menos_http1() {
        assert_eq!(difference_between(Some(100), Some(120)), Some(20));
        assert_eq!(difference_between(Some(120), Some(100)), Some(-20));
        assert_eq!(difference_between(Some(100), None), None);
    }

    #[test]
    fn avalia_assertions_de_status_header_e_body() {
        let definitions = vec![
            AssertionDefinition::StatusEquals { expected: 200 },
            AssertionDefinition::HeaderContains {
                name: "Content-Type".to_string(),
                value: "application/json".to_string(),
            },
            AssertionDefinition::BodyContains {
                value: "approved".to_string(),
            },
        ];
        let headers = vec![ResponseHeader {
            name: "content-type".to_string(),
            value: "application/json; charset=utf-8".to_string(),
        }];

        let results = evaluate_assertions(&definitions, 200, &headers, "{\"state\":\"approved\"}");

        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|result| result.passed));
        assert_eq!(results[0].actual, "200");
    }

    #[test]
    fn assertion_de_header_inexistente_falha_sem_expor_body() {
        let results = evaluate_assertions(
            &[AssertionDefinition::HeaderContains {
                name: "X-Request-Id".to_string(),
                value: "abc".to_string(),
            }],
            200,
            &[],
            "segredo que não deve aparecer",
        );

        assert!(!results[0].passed);
        assert_eq!(results[0].actual, "header não encontrado");
        assert!(!results[0].actual.contains("segredo"));
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
    fn aplica_bearer_auth_no_header() {
        let client = Client::new();
        let request = apply_auth(
            client.get("http://localhost"),
            Some(&RequestAuth::Bearer {
                token: "local-token".to_string(),
            }),
        )
        .unwrap()
        .build()
        .unwrap();

        assert_eq!(
            request.headers().get("authorization").unwrap(),
            "Bearer local-token"
        );
    }

    #[test]
    fn aplica_basic_auth_no_header() {
        let client = Client::new();
        let request = apply_auth(
            client.get("http://localhost"),
            Some(&RequestAuth::Basic {
                username: "user".to_string(),
                password: "password".to_string(),
            }),
        )
        .unwrap()
        .build()
        .unwrap();

        assert_eq!(
            request.headers().get("authorization").unwrap(),
            "Basic dXNlcjpwYXNzd29yZA=="
        );
    }

    #[test]
    fn aplica_api_key_no_header() {
        let client = Client::new();
        let request = apply_auth(
            client.get("http://localhost"),
            Some(&RequestAuth::ApiKey {
                name: "X-API-Key".to_string(),
                value: "local-key".to_string(),
                location: ApiKeyLocation::Header,
            }),
        )
        .unwrap()
        .build()
        .unwrap();

        assert_eq!(request.headers().get("x-api-key").unwrap(), "local-key");
    }

    #[test]
    fn aplica_api_key_na_query() {
        let mut query = Vec::new();
        append_auth_query(
            &mut query,
            &RequestAuth::ApiKey {
                name: "api_key".to_string(),
                value: "local-key".to_string(),
                location: ApiKeyLocation::Query,
            },
        )
        .unwrap();

        assert_eq!(
            query,
            vec![("api_key".to_string(), "local-key".to_string())]
        );
    }

    #[test]
    fn rejeita_api_key_sem_nome() {
        let error = validate_api_key_name(" ").unwrap_err();

        assert_eq!(error.kind, "invalid_auth_name");
    }

    #[test]
    fn monta_cookie_header_apenas_com_cookies_habilitados() {
        let header = build_cookie_header(&[
            CookieEntry {
                name: "session".to_string(),
                value: "local-session".to_string(),
                enabled: true,
            },
            CookieEntry {
                name: "theme".to_string(),
                value: "dark".to_string(),
                enabled: false,
            },
            CookieEntry {
                name: "locale".to_string(),
                value: "pt-BR".to_string(),
                enabled: true,
            },
        ])
        .unwrap()
        .unwrap();

        assert_eq!(
            header.to_str().unwrap(),
            "session=local-session; locale=pt-BR"
        );
    }

    #[test]
    fn rejeita_cookie_com_nome_invalido() {
        let error = build_cookie_header(&[CookieEntry {
            name: "session token".to_string(),
            value: "local-session".to_string(),
            enabled: true,
        }])
        .unwrap_err();

        assert_eq!(error.kind, "invalid_cookie_name");
    }

    #[test]
    fn rejeita_cookie_com_valor_invalido() {
        let error = build_cookie_header(&[CookieEntry {
            name: "session".to_string(),
            value: "local;session".to_string(),
            enabled: true,
        }])
        .unwrap_err();

        assert_eq!(error.kind, "invalid_cookie_value");
    }

    #[test]
    fn monta_form_urlencoded_apenas_com_campos_habilitados() {
        let fields = build_form_fields(&[
            FormField {
                name: "email".to_string(),
                value: "dev@example.com".to_string(),
                enabled: true,
            },
            FormField {
                name: "ignored".to_string(),
                value: "value".to_string(),
                enabled: false,
            },
        ])
        .unwrap();

        assert_eq!(
            fields,
            vec![("email".to_string(), "dev@example.com".to_string())]
        );
    }

    #[test]
    fn rejeita_form_field_sem_nome() {
        let error = build_form_fields(&[FormField {
            name: " ".to_string(),
            value: "value".to_string(),
            enabled: true,
        }])
        .unwrap_err();

        assert_eq!(error.kind, "invalid_form_field_name");
    }

    #[test]
    fn le_arquivo_multipart_fixture_local() {
        let path = std::env::temp_dir().join(format!(
            "larry-multipart-{}-fixture.txt",
            std::process::id()
        ));
        std::fs::write(&path, b"fixture").unwrap();

        let result = tauri::async_runtime::block_on(build_multipart_form(MultipartBody {
            fields: vec![FormField {
                name: "description".to_string(),
                value: "fixture".to_string(),
                enabled: true,
            }],
            files: vec![MultipartFile {
                name: "attachment".to_string(),
                path: path.to_string_lossy().to_string(),
                enabled: true,
            }],
        }));

        std::fs::remove_file(path).unwrap();
        assert!(result.is_ok());
    }

    #[test]
    fn rejeita_multipart_com_arquivo_inexistente() {
        let result = tauri::async_runtime::block_on(build_multipart_form(MultipartBody {
            fields: vec![],
            files: vec![MultipartFile {
                name: "attachment".to_string(),
                path: "C:\\larry\\missing-file.txt".to_string(),
                enabled: true,
            }],
        }));

        let error = result.unwrap_err();
        assert_eq!(error.kind, "file_read");
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

        let mut fixture_request = request(&format!("http://127.0.0.1:{port}/health"));
        fixture_request
            .assertions
            .push(AssertionDefinition::StatusEquals { expected: 200 });
        fixture_request
            .assertions
            .push(AssertionDefinition::HeaderContains {
                name: "Content-Type".to_string(),
                value: "text/plain".to_string(),
            });
        fixture_request
            .assertions
            .push(AssertionDefinition::BodyContains {
                value: "hello".to_string(),
            });

        let result = tauri::async_runtime::block_on(execute(fixture_request)).unwrap();

        server.join().unwrap();

        assert_eq!(result.status, 200);
        assert_eq!(result.body, "hello");
        assert_eq!(result.body_size, 5);
        assert_eq!(result.assertions.len(), 3);
        assert!(result.assertions.iter().all(|assertion| assertion.passed));
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
