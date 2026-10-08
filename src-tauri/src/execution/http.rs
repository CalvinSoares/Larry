use std::time::{Duration, Instant};

use reqwest::{header, Client, Method, Url};
use serde::Serialize;

use crate::domain::request::{HttpMethod, RequestBody, RequestDefinition};

const MAX_RESPONSE_BYTES: usize = 10 * 1024 * 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<ResponseHeader>,
    pub body: String,
    pub body_size: usize,
    pub duration_ms: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionError {
    pub kind: String,
    pub message: String,
}

impl ExecutionError {
    fn new(kind: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            message: message.into(),
        }
    }
}

pub async fn execute(request: RequestDefinition) -> Result<HttpResponse, ExecutionError> {
    let parsed_url = validate_request(&request)?;
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

    let started_at = Instant::now();
    let response = builder.send().await.map_err(classify_request_error)?;
    let status = response.status();
    let status_text = status.canonical_reason().unwrap_or_default().to_string();
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

    Ok(HttpResponse {
        status: status.as_u16(),
        status_text,
        headers,
        body,
        body_size,
        duration_ms: started_at.elapsed().as_millis() as u64,
    })
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

fn classify_request_error(error: reqwest::Error) -> ExecutionError {
    let kind = if error.is_timeout() {
        "timeout"
    } else if error.is_connect() {
        "connection"
    } else if error.is_request() {
        "request"
    } else {
        "transport"
    };

    ExecutionError::new(kind, error.to_string())
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
    }
}
