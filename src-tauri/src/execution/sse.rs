use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use futures_util::Stream;
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, ACCEPT};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::mpsc;

use crate::domain::request::HeaderEntry;

const MAX_SSE_HEADERS: usize = 128;
const MAX_SSE_EVENT_BYTES: usize = 4 * 1024 * 1024;
static SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SseOpenRequest {
    pub url: String,
    #[serde(default)]
    pub headers: Vec<HeaderEntry>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SseSession {
    pub session_id: String,
    pub url: String,
    pub opened_at_ms: u64,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SseEvent {
    pub session_id: String,
    pub event_type: String,
    pub event_name: Option<String>,
    pub data: Option<String>,
    pub event_id: Option<String>,
    pub retry_ms: Option<u64>,
    pub size_bytes: usize,
    pub timestamp_ms: u64,
    pub technical: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SseError {
    pub kind: String,
    pub message: String,
    pub technical: Option<String>,
}

impl SseError {
    fn new(kind: impl Into<String>, message: impl Into<String>, technical: Option<String>) -> Self {
        Self {
            kind: kind.into(),
            message: message.into(),
            technical,
        }
    }
}

enum SseCommand {
    Close,
}

#[derive(Clone, Default)]
pub struct SseManager {
    sessions: Arc<Mutex<HashMap<String, mpsc::Sender<SseCommand>>>>,
}

impl SseManager {
    fn insert(&self, session_id: String, sender: mpsc::Sender<SseCommand>) {
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.insert(session_id, sender);
        }
    }

    fn sender(&self, session_id: &str) -> Option<mpsc::Sender<SseCommand>> {
        self.sessions
            .lock()
            .ok()
            .and_then(|sessions| sessions.get(session_id).cloned())
    }

    fn remove(&self, session_id: &str) {
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.remove(session_id);
        }
    }
}

#[tauri::command]
pub async fn open_sse(
    app: AppHandle,
    manager: State<'_, SseManager>,
    request: SseOpenRequest,
) -> Result<SseSession, SseError> {
    let url = validate_url(&request.url)?;
    let headers = build_headers(&request.headers)?;
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .headers(headers)
        .send()
        .await
        .map_err(|error| {
            SseError::new(
                "connect",
                "Não foi possível abrir a conexão SSE.",
                Some(error.to_string()),
            )
        })?;

    let status = response.status();
    if !status.is_success() {
        return Err(SseError::new(
            "http_status",
            "O endpoint SSE respondeu com falha HTTP.",
            Some(status.to_string()),
        ));
    }

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);
    let session_id = next_session_id();
    let opened_at_ms = now_ms();
    let (sender, receiver) = mpsc::channel(4);
    manager.insert(session_id.clone(), sender);

    let task_app = app.clone();
    let task_manager = manager.inner().clone();
    let task_session_id = session_id.clone();
    tokio::spawn(async move {
        run_session(
            task_app,
            task_manager,
            task_session_id,
            response.bytes_stream(),
            receiver,
        )
        .await;
    });

    emit_event(
        &app,
        SseEvent {
            session_id: session_id.clone(),
            event_type: "opened".to_string(),
            event_name: None,
            data: None,
            event_id: None,
            retry_ms: None,
            size_bytes: 0,
            timestamp_ms: opened_at_ms,
            technical: content_type.clone(),
        },
    );

    Ok(SseSession {
        session_id,
        url,
        opened_at_ms,
        content_type,
    })
}

#[tauri::command]
pub async fn close_sse(manager: State<'_, SseManager>, session_id: String) -> Result<(), SseError> {
    let sender = manager.sender(&session_id).ok_or_else(|| {
        SseError::new(
            "session_not_found",
            "A sessão SSE não está mais aberta.",
            None,
        )
    })?;

    sender
        .send(SseCommand::Close)
        .await
        .map_err(|_| SseError::new("session_closed", "A sessão SSE já estava fechada.", None))
}

async fn run_session<S>(
    app: AppHandle,
    manager: SseManager,
    session_id: String,
    mut stream: S,
    mut receiver: mpsc::Receiver<SseCommand>,
) where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    let mut buffer = String::new();
    let mut frame = SseFrame::default();

    loop {
        tokio::select! {
            chunk = stream.next() => {
                let Some(chunk) = chunk else {
                    dispatch_frame(&app, &session_id, &mut frame);
                    emit_closed(&app, &session_id, Some("server_closed_stream".to_string()));
                    break;
                };

                let chunk = match chunk {
                    Ok(chunk) => chunk,
                    Err(error) => {
                        emit_error(
                            &app,
                            &session_id,
                            "receive",
                            "A conexão SSE falhou durante o recebimento.",
                            Some(error.to_string()),
                        );
                        break;
                    }
                };

                buffer.push_str(&String::from_utf8_lossy(&chunk));
                if buffer.len() > MAX_SSE_EVENT_BYTES {
                    emit_error(
                        &app,
                        &session_id,
                        "event_too_large",
                        "O evento SSE excedeu o limite local de 4 MiB.",
                        Some(format!("{} bytes", buffer.len())),
                    );
                    break;
                }

                let mut oversized = false;
                while let Some(newline) = buffer.find('\n') {
                    let line = buffer[..newline].trim_end_matches('\r').to_string();
                    buffer.drain(..=newline);
                    consume_line(&line, &mut frame, |message| {
                        emit_message(&app, &session_id, message);
                    });
                    if frame.data_bytes > MAX_SSE_EVENT_BYTES {
                        emit_error(
                            &app,
                            &session_id,
                            "event_too_large",
                            "O evento SSE excedeu o limite local de 4 MiB.",
                            Some(format!("{} bytes", frame.data_bytes)),
                        );
                        oversized = true;
                        break;
                    }
                }
                if oversized {
                    break;
                }
            }
            command = receiver.recv() => {
                let Some(SseCommand::Close) = command else {
                    emit_closed(&app, &session_id, Some("local_channel_closed".to_string()));
                    break;
                };

                emit_closed(&app, &session_id, Some("local_requested_close".to_string()));
                break;
            }
        }
    }

    if !buffer.is_empty() {
        let line = buffer.trim_end_matches('\r').to_string();
        consume_line(&line, &mut frame, |message| {
            emit_message(&app, &session_id, message);
        });
    }
    dispatch_frame(&app, &session_id, &mut frame);
    manager.remove(&session_id);
}

#[derive(Default)]
struct SseFrame {
    event_name: Option<String>,
    data: Vec<String>,
    data_bytes: usize,
    event_id: Option<String>,
    retry_ms: Option<u64>,
}

struct SseMessage {
    event_name: Option<String>,
    data: String,
    event_id: Option<String>,
    retry_ms: Option<u64>,
}

fn consume_line<F>(line: &str, frame: &mut SseFrame, mut emit: F)
where
    F: FnMut(SseMessage),
{
    if line.is_empty() {
        if let Some(message) = take_message(frame) {
            emit(message);
        }
        return;
    }

    if line.starts_with(':') {
        return;
    }

    let (field, value) = line.split_once(':').unwrap_or((line, ""));
    let value = value.strip_prefix(' ').unwrap_or(value).to_string();

    match field {
        "event" => frame.event_name = Some(value),
        "data" => {
            frame.data_bytes += value.len();
            frame.data.push(value);
        }
        "id" => frame.event_id = Some(value),
        "retry" => frame.retry_ms = value.parse().ok(),
        _ => {}
    }
}

fn take_message(frame: &mut SseFrame) -> Option<SseMessage> {
    if frame.data.is_empty() {
        return None;
    }

    let message = SseMessage {
        event_name: frame.event_name.take(),
        data: frame.data.drain(..).collect::<Vec<_>>().join("\n"),
        event_id: frame.event_id.take(),
        retry_ms: frame.retry_ms.take(),
    };
    frame.data_bytes = 0;
    Some(message)
}

fn dispatch_frame(app: &AppHandle, session_id: &str, frame: &mut SseFrame) {
    if let Some(message) = take_message(frame) {
        emit_message(app, session_id, message);
    }
}

fn validate_url(raw_url: &str) -> Result<String, SseError> {
    let url = Url::parse(raw_url).map_err(|error| {
        SseError::new(
            "invalid_url",
            "A URL SSE não pôde ser interpretada.",
            Some(error.to_string()),
        )
    })?;

    if !matches!(url.scheme(), "http" | "https") {
        return Err(SseError::new(
            "unsupported_scheme",
            "SSE aceita somente URLs http:// ou https://.",
            Some(url.scheme().to_string()),
        ));
    }

    Ok(url.to_string())
}

fn build_headers(headers: &[HeaderEntry]) -> Result<HeaderMap, SseError> {
    if headers.len() > MAX_SSE_HEADERS {
        return Err(SseError::new(
            "too_many_headers",
            "A conexão SSE excede o limite local de headers.",
            None,
        ));
    }

    let mut result = HeaderMap::new();
    if !headers
        .iter()
        .any(|header| header.enabled && header.name.eq_ignore_ascii_case("accept"))
    {
        result.insert(ACCEPT, HeaderValue::from_static("text/event-stream"));
    }

    for header in headers.iter().filter(|header| header.enabled) {
        let name = HeaderName::from_bytes(header.name.as_bytes()).map_err(|error| {
            SseError::new(
                "invalid_header_name",
                "Um nome de header SSE é inválido.",
                Some(error.to_string()),
            )
        })?;
        let value = HeaderValue::from_str(&header.value).map_err(|error| {
            SseError::new(
                "invalid_header_value",
                "Um valor de header SSE é inválido.",
                Some(error.to_string()),
            )
        })?;
        result.insert(name, value);
    }

    Ok(result)
}

fn emit_message(app: &AppHandle, session_id: &str, message: SseMessage) {
    let size_bytes = message.data.len();
    emit_event(
        app,
        SseEvent {
            session_id: session_id.to_string(),
            event_type: "message".to_string(),
            event_name: message.event_name,
            data: Some(message.data),
            event_id: message.event_id,
            retry_ms: message.retry_ms,
            size_bytes,
            timestamp_ms: now_ms(),
            technical: None,
        },
    );
}

fn next_session_id() -> String {
    format!(
        "sse-{}-{}",
        now_ms(),
        SESSION_COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

fn emit_event(app: &AppHandle, event: SseEvent) {
    let _ = app.emit("sse_event", event);
}

fn emit_closed(app: &AppHandle, session_id: &str, technical: Option<String>) {
    emit_event(
        app,
        SseEvent {
            session_id: session_id.to_string(),
            event_type: "closed".to_string(),
            event_name: None,
            data: None,
            event_id: None,
            retry_ms: None,
            size_bytes: 0,
            timestamp_ms: now_ms(),
            technical,
        },
    );
}

fn emit_error(
    app: &AppHandle,
    session_id: &str,
    kind: &str,
    message: &str,
    technical: Option<String>,
) {
    emit_event(
        app,
        SseEvent {
            session_id: session_id.to_string(),
            event_type: "error".to_string(),
            event_name: None,
            data: Some(message.to_string()),
            event_id: None,
            retry_ms: None,
            size_bytes: 0,
            timestamp_ms: now_ms(),
            technical: Some(match technical {
                Some(value) => format!("{}: {}", kind, value),
                None => kind.to_string(),
            }),
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aceita_http_e_https() {
        assert_eq!(
            validate_url("http://localhost:3000/events").unwrap(),
            "http://localhost:3000/events"
        );
        assert_eq!(
            validate_url("https://example.com/events").unwrap(),
            "https://example.com/events"
        );
    }

    #[test]
    fn rejeita_ws_como_sse() {
        let error = validate_url("ws://localhost:9000/events").unwrap_err();
        assert_eq!(error.kind, "unsupported_scheme");
    }

    #[test]
    fn monta_headers_e_accept_padrao() {
        let headers = build_headers(&[]).unwrap();
        assert_eq!(headers[ACCEPT], "text/event-stream");
    }

    #[test]
    fn ignora_header_desabilitado() {
        let headers = build_headers(&[HeaderEntry {
            name: "Authorization".to_string(),
            value: "hidden".to_string(),
            enabled: false,
        }])
        .unwrap();

        assert!(!headers.contains_key("authorization"));
    }

    #[test]
    fn interpreta_frame_sse_com_multiplas_linhas_de_data() {
        let mut frame = SseFrame::default();
        let mut message = None;
        consume_line("event: payment", &mut frame, |value| message = Some(value));
        consume_line("id: 42", &mut frame, |value| message = Some(value));
        consume_line("retry: 5000", &mut frame, |value| message = Some(value));
        consume_line("data: {\"status\":", &mut frame, |value| {
            message = Some(value)
        });
        consume_line("data: \"paid\"}", &mut frame, |value| message = Some(value));
        consume_line("", &mut frame, |value| message = Some(value));

        let message = message.expect("frame SSE esperado");
        assert_eq!(message.event_name.as_deref(), Some("payment"));
        assert_eq!(message.data, "{\"status\":\n\"paid\"}");
        assert_eq!(message.event_id.as_deref(), Some("42"));
        assert_eq!(message.retry_ms, Some(5000));
    }

    #[test]
    fn ignora_comentarios_keep_alive() {
        let mut frame = SseFrame::default();
        let mut emitted = false;
        consume_line(": keep-alive", &mut frame, |_| emitted = true);
        consume_line("", &mut frame, |_| emitted = true);
        assert!(!emitted);
    }

    #[test]
    fn contabiliza_data_acumulado_do_frame() {
        let mut frame = SseFrame::default();
        consume_line("data: primeiro", &mut frame, |_| {});
        consume_line("data: segundo", &mut frame, |_| {});

        assert_eq!(frame.data_bytes, "primeiro".len() + "segundo".len());
    }
}
