use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use futures_util::{SinkExt, StreamExt};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::{HeaderName, HeaderValue};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, WebSocketStream};

use crate::domain::request::HeaderEntry;

const MAX_WEBSOCKET_MESSAGE_BYTES: usize = 4 * 1024 * 1024;
const MAX_WEBSOCKET_HEADERS: usize = 128;
static SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSocketOpenRequest {
    pub url: String,
    #[serde(default)]
    pub headers: Vec<HeaderEntry>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSocketSession {
    pub session_id: String,
    pub url: String,
    pub opened_at_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSocketEvent {
    pub session_id: String,
    pub event_type: String,
    pub direction: Option<String>,
    pub payload: Option<String>,
    pub binary: bool,
    pub size_bytes: usize,
    pub timestamp_ms: u64,
    pub technical: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSocketError {
    pub kind: String,
    pub message: String,
    pub technical: Option<String>,
}

impl WebSocketError {
    fn new(kind: impl Into<String>, message: impl Into<String>, technical: Option<String>) -> Self {
        Self {
            kind: kind.into(),
            message: message.into(),
            technical,
        }
    }
}

enum WebSocketCommand {
    SendText(String),
    Close,
}

#[derive(Clone, Default)]
pub struct WebSocketManager {
    sessions: Arc<Mutex<HashMap<String, mpsc::Sender<WebSocketCommand>>>>,
}

impl WebSocketManager {
    fn insert(&self, session_id: String, sender: mpsc::Sender<WebSocketCommand>) {
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.insert(session_id, sender);
        }
    }

    fn sender(&self, session_id: &str) -> Option<mpsc::Sender<WebSocketCommand>> {
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
pub async fn open_websocket(
    app: AppHandle,
    manager: State<'_, WebSocketManager>,
    request: WebSocketOpenRequest,
) -> Result<WebSocketSession, WebSocketError> {
    let url = validate_url(&request.url)?;
    let client_request = build_client_request(&url, &request.headers)?;
    let (stream, _) = connect_async(client_request).await.map_err(|error| {
        WebSocketError::new(
            "connect",
            "Não foi possível abrir a conexão WebSocket.",
            Some(error.to_string()),
        )
    })?;

    let session_id = next_session_id();
    let opened_at_ms = now_ms();
    let (sender, receiver) = mpsc::channel(64);
    manager.insert(session_id.clone(), sender);

    let manager = manager.inner().clone();
    let task_app = app.clone();
    let task_session_id = session_id.clone();
    tokio::spawn(async move {
        run_session(task_app, manager, task_session_id, stream, receiver).await;
    });

    emit_event(
        &app,
        WebSocketEvent {
            session_id: session_id.clone(),
            event_type: "opened".to_string(),
            direction: None,
            payload: None,
            binary: false,
            size_bytes: 0,
            timestamp_ms: opened_at_ms,
            technical: None,
        },
    );

    Ok(WebSocketSession {
        session_id,
        url,
        opened_at_ms,
    })
}

#[tauri::command]
pub async fn send_websocket_message(
    manager: State<'_, WebSocketManager>,
    session_id: String,
    message: String,
) -> Result<(), WebSocketError> {
    validate_message_size(&message)?;
    let sender = manager.sender(&session_id).ok_or_else(|| {
        WebSocketError::new(
            "session_not_found",
            "A sessão WebSocket não está mais aberta.",
            None,
        )
    })?;

    sender
        .send(WebSocketCommand::SendText(message))
        .await
        .map_err(|_| {
            WebSocketError::new(
                "session_closed",
                "A sessão WebSocket foi fechada antes do envio.",
                None,
            )
        })
}

#[tauri::command]
pub async fn close_websocket(
    manager: State<'_, WebSocketManager>,
    session_id: String,
) -> Result<(), WebSocketError> {
    let sender = manager.sender(&session_id).ok_or_else(|| {
        WebSocketError::new(
            "session_not_found",
            "A sessão WebSocket não está mais aberta.",
            None,
        )
    })?;

    sender.send(WebSocketCommand::Close).await.map_err(|_| {
        WebSocketError::new(
            "session_closed",
            "A sessão WebSocket já estava fechada.",
            None,
        )
    })
}

async fn run_session(
    app: AppHandle,
    manager: WebSocketManager,
    session_id: String,
    mut stream: WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    mut receiver: mpsc::Receiver<WebSocketCommand>,
) {
    loop {
        tokio::select! {
            incoming = stream.next() => {
                let Some(incoming) = incoming else {
                    emit_closed(&app, &session_id, Some("peer_closed_stream".to_string()));
                    break;
                };

                match incoming {
                    Ok(Message::Text(message)) => {
                        let message = message.to_string();
                        emit_event(&app, WebSocketEvent {
                            session_id: session_id.clone(),
                            event_type: "message".to_string(),
                            direction: Some("incoming".to_string()),
                            size_bytes: message.len(),
                            payload: Some(message),
                            binary: false,
                            timestamp_ms: now_ms(),
                            technical: None,
                        });
                    }
                    Ok(Message::Binary(bytes)) => {
                        if bytes.len() > MAX_WEBSOCKET_MESSAGE_BYTES {
                            emit_error(
                                &app,
                                &session_id,
                                "message_too_large",
                                "A mensagem WebSocket recebida excedeu o limite local.",
                                Some(format!("{} bytes", bytes.len())),
                            );
                            break;
                        }

                        emit_event(&app, WebSocketEvent {
                            session_id: session_id.clone(),
                            event_type: "message".to_string(),
                            direction: Some("incoming".to_string()),
                            size_bytes: bytes.len(),
                            payload: None,
                            binary: true,
                            timestamp_ms: now_ms(),
                            technical: None,
                        });
                    }
                    Ok(Message::Ping(payload)) => {
                        if stream.send(Message::Pong(payload)).await.is_err() {
                            emit_error(
                                &app,
                                &session_id,
                                "transport",
                                "A resposta automática ao ping falhou.",
                                None,
                            );
                            break;
                        }
                    }
                    Ok(Message::Pong(_)) => {}
                    Ok(Message::Close(_)) => {
                        emit_closed(&app, &session_id, Some("peer_requested_close".to_string()));
                        break;
                    }
                    Ok(Message::Frame(_)) => {}
                    Err(error) => {
                        emit_error(
                            &app,
                            &session_id,
                            "receive",
                            "A conexão WebSocket falhou durante o recebimento.",
                            Some(error.to_string()),
                        );
                        break;
                    }
                }
            }
            command = receiver.recv() => {
                let Some(command) = command else {
                    let _ = stream.close(None).await;
                    emit_closed(&app, &session_id, Some("local_channel_closed".to_string()));
                    break;
                };

                match command {
                    WebSocketCommand::SendText(message) => {
                        let size_bytes = message.len();
                        if stream.send(Message::Text(message.clone().into())).await.is_err() {
                            emit_error(
                                &app,
                                &session_id,
                                "send",
                                "A mensagem WebSocket não pôde ser enviada.",
                                None,
                            );
                            break;
                        }

                        emit_event(&app, WebSocketEvent {
                            session_id: session_id.clone(),
                            event_type: "message".to_string(),
                            direction: Some("outgoing".to_string()),
                            payload: Some(message),
                            binary: false,
                            size_bytes,
                            timestamp_ms: now_ms(),
                            technical: None,
                        });
                    }
                    WebSocketCommand::Close => {
                        let _ = stream.close(None).await;
                        emit_closed(&app, &session_id, Some("local_requested_close".to_string()));
                        break;
                    }
                }
            }
        }
    }

    manager.remove(&session_id);
}

fn validate_url(raw_url: &str) -> Result<String, WebSocketError> {
    let url = Url::parse(raw_url).map_err(|error| {
        WebSocketError::new(
            "invalid_url",
            "A URL WebSocket não pôde ser interpretada.",
            Some(error.to_string()),
        )
    })?;

    if !matches!(url.scheme(), "ws" | "wss") {
        return Err(WebSocketError::new(
            "unsupported_scheme",
            "WebSocket aceita somente URLs ws:// ou wss://.",
            Some(url.scheme().to_string()),
        ));
    }

    Ok(url.to_string())
}

fn build_client_request(
    url: &str,
    headers: &[HeaderEntry],
) -> Result<tokio_tungstenite::tungstenite::http::Request<()>, WebSocketError> {
    if headers.len() > MAX_WEBSOCKET_HEADERS {
        return Err(WebSocketError::new(
            "too_many_headers",
            "A conexão WebSocket excede o limite local de headers.",
            None,
        ));
    }

    let mut request = url.into_client_request().map_err(|error| {
        WebSocketError::new(
            "invalid_request",
            "A requisição de handshake WebSocket não pôde ser criada.",
            Some(error.to_string()),
        )
    })?;

    for header in headers.iter().filter(|header| header.enabled) {
        let name = HeaderName::from_bytes(header.name.as_bytes()).map_err(|error| {
            WebSocketError::new(
                "invalid_header_name",
                "Um nome de header WebSocket é inválido.",
                Some(error.to_string()),
            )
        })?;
        let value = HeaderValue::from_str(&header.value).map_err(|error| {
            WebSocketError::new(
                "invalid_header_value",
                "Um valor de header WebSocket é inválido.",
                Some(error.to_string()),
            )
        })?;
        request.headers_mut().insert(name, value);
    }

    Ok(request)
}

fn validate_message_size(message: &str) -> Result<(), WebSocketError> {
    if message.len() > MAX_WEBSOCKET_MESSAGE_BYTES {
        return Err(WebSocketError::new(
            "message_too_large",
            "A mensagem WebSocket excede o limite local de 4 MiB.",
            Some(format!("{} bytes", message.len())),
        ));
    }

    Ok(())
}

fn next_session_id() -> String {
    format!(
        "ws-{}-{}",
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

fn emit_event(app: &AppHandle, event: WebSocketEvent) {
    let _ = app.emit("websocket_event", event);
}

fn emit_closed(app: &AppHandle, session_id: &str, technical: Option<String>) {
    emit_event(
        app,
        WebSocketEvent {
            session_id: session_id.to_string(),
            event_type: "closed".to_string(),
            direction: None,
            payload: None,
            binary: false,
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
        WebSocketEvent {
            session_id: session_id.to_string(),
            event_type: "error".to_string(),
            direction: None,
            payload: Some(message.to_string()),
            binary: false,
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

    fn header(name: &str, value: &str) -> HeaderEntry {
        HeaderEntry {
            name: name.to_string(),
            value: value.to_string(),
            enabled: true,
        }
    }

    #[test]
    fn aceita_ws_e_wss() {
        assert_eq!(
            validate_url("ws://localhost:9000/socket").unwrap(),
            "ws://localhost:9000/socket"
        );
        assert_eq!(
            validate_url("wss://example.com/socket").unwrap(),
            "wss://example.com/socket"
        );
    }

    #[test]
    fn rejeita_http_como_websocket() {
        let error = validate_url("https://example.com").unwrap_err();
        assert_eq!(error.kind, "unsupported_scheme");
    }

    #[test]
    fn cria_handshake_com_headers_habilitados() {
        let request = build_client_request(
            "ws://localhost:9000/socket",
            &[header("Authorization", "Bearer local")],
        )
        .unwrap();

        assert_eq!(request.headers()["authorization"], "Bearer local");
    }

    #[test]
    fn ignora_headers_desabilitados() {
        let request = build_client_request(
            "ws://localhost:9000/socket",
            &[HeaderEntry {
                name: "X-Disabled".to_string(),
                value: "hidden".to_string(),
                enabled: false,
            }],
        )
        .unwrap();

        assert!(!request.headers().contains_key("x-disabled"));
    }

    #[test]
    fn rejeita_mensagem_maior_que_limite() {
        let message = "x".repeat(MAX_WEBSOCKET_MESSAGE_BYTES + 1);
        let error = validate_message_size(&message).unwrap_err();
        assert_eq!(error.kind, "message_too_large");
    }
}
