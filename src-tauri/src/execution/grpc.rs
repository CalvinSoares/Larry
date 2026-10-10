use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use futures_util::stream;
use prost::Message;
use prost_reflect::{DescriptorPool, DynamicMessage};
use prost_types::FileDescriptorSet;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::watch;
use tokio::time::timeout;
use tonic::client::Grpc;
use tonic::codec::{BufferSettings, Codec, DecodeBuf, Decoder, EncodeBuf, Encoder};
use tonic::codegen::http::uri::PathAndQuery;
use tonic::metadata::{AsciiMetadataKey, AsciiMetadataValue, KeyAndValueRef, MetadataMap};
use tonic::transport::Endpoint;
use tonic::Request;
use tonic::Status;
use tonic_reflection::pb::v1::server_reflection_client::ServerReflectionClient;
use tonic_reflection::pb::v1::server_reflection_request::MessageRequest;
use tonic_reflection::pb::v1::server_reflection_response::MessageResponse;
use tonic_reflection::pb::v1::ServerReflectionRequest;

const MAX_PROTO_BYTES: u64 = 5 * 1024 * 1024;
const MAX_METADATA: usize = 128;
const MAX_REFLECTION_SERVICES: usize = 512;
const MAX_REFLECTION_DESCRIPTOR_BYTES: usize = 16 * 1024 * 1024;
const MAX_STREAM_MESSAGES: usize = 512;
const REFLECTION_TIMEOUT: Duration = Duration::from_secs(15);
const STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
struct DynamicGrpcCodec {
    output: prost_reflect::MessageDescriptor,
}

#[derive(Debug, Clone, Default)]
struct DynamicGrpcEncoder;

#[derive(Debug, Clone)]
struct DynamicGrpcDecoder {
    descriptor: prost_reflect::MessageDescriptor,
}

impl Codec for DynamicGrpcCodec {
    type Encode = DynamicMessage;
    type Decode = DynamicMessage;
    type Encoder = DynamicGrpcEncoder;
    type Decoder = DynamicGrpcDecoder;

    fn encoder(&mut self) -> Self::Encoder {
        DynamicGrpcEncoder
    }

    fn decoder(&mut self) -> Self::Decoder {
        DynamicGrpcDecoder {
            descriptor: self.output.clone(),
        }
    }
}

impl Encoder for DynamicGrpcEncoder {
    type Item = DynamicMessage;
    type Error = Status;

    fn encode(&mut self, item: Self::Item, buffer: &mut EncodeBuf<'_>) -> Result<(), Self::Error> {
        item.encode(buffer)
            .map_err(|error| Status::internal(error.to_string()))
    }

    fn buffer_settings(&self) -> BufferSettings {
        BufferSettings::default()
    }
}

impl Decoder for DynamicGrpcDecoder {
    type Item = DynamicMessage;
    type Error = Status;

    fn decode(&mut self, buffer: &mut DecodeBuf<'_>) -> Result<Option<Self::Item>, Self::Error> {
        DynamicMessage::decode(self.descriptor.clone(), buffer)
            .map(Some)
            .map_err(|error| Status::internal(error.to_string()))
    }

    fn buffer_settings(&self) -> BufferSettings {
        BufferSettings::default()
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GrpcSchema {
    pub source_path: String,
    pub services: Vec<GrpcService>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrpcReflectionRequest {
    pub url: String,
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub metadata: Vec<GrpcMetadataEntry>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GrpcService {
    pub name: String,
    pub full_name: String,
    pub methods: Vec<GrpcMethod>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GrpcMethod {
    pub name: String,
    pub full_name: String,
    pub input_type: String,
    pub output_type: String,
    pub client_streaming: bool,
    pub server_streaming: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrpcMetadataEntry {
    pub name: String,
    pub value: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrpcUnaryRequest {
    pub proto_path: String,
    pub url: String,
    pub service: String,
    pub method: String,
    pub body: Value,
    #[serde(default)]
    pub metadata: Vec<GrpcMetadataEntry>,
    #[serde(default)]
    pub reflection: Option<GrpcReflectionRequest>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GrpcUnaryResponse {
    pub status: String,
    pub body: Value,
    pub duration_ms: u64,
    pub response_metadata: Vec<GrpcMetadataEntry>,
    pub trailers: Vec<GrpcMetadataEntry>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrpcStreamingRequest {
    pub proto_path: String,
    pub url: String,
    pub service: String,
    pub method: String,
    pub messages: Vec<Value>,
    #[serde(default)]
    pub metadata: Vec<GrpcMetadataEntry>,
    #[serde(default)]
    pub reflection: Option<GrpcReflectionRequest>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GrpcStreamingResponse {
    pub status: String,
    pub messages: Vec<Value>,
    pub message_count: usize,
    pub duration_ms: u64,
    pub response_metadata: Vec<GrpcMetadataEntry>,
    pub trailers: Vec<GrpcMetadataEntry>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GrpcError {
    pub kind: String,
    pub message: String,
    pub technical: String,
}

impl GrpcError {
    fn new(
        kind: impl Into<String>,
        message: impl Into<String>,
        technical: impl Into<String>,
    ) -> Self {
        Self {
            kind: kind.into(),
            message: message.into(),
            technical: technical.into(),
        }
    }
}

#[derive(Clone, Default)]
pub struct GrpcStreamManager {
    runs: Arc<Mutex<HashMap<String, watch::Sender<bool>>>>,
}

impl GrpcStreamManager {
    pub fn begin(&self, run_id: String) -> Result<watch::Receiver<bool>, GrpcError> {
        let (sender, receiver) = watch::channel(false);
        let mut runs = self.runs.lock().map_err(|_| {
            GrpcError::new(
                "stream_manager",
                "Não foi possível acessar o controle do streaming gRPC.",
                "O mutex de execuções gRPC está indisponível.",
            )
        })?;

        if runs.contains_key(&run_id) {
            return Err(GrpcError::new(
                "stream_already_running",
                "Já existe um streaming gRPC com este identificador em execução.",
                format!("Run ID recebido: {run_id}"),
            ));
        }

        runs.insert(run_id, sender);
        Ok(receiver)
    }

    pub fn cancel(&self, run_id: &str) -> Result<(), GrpcError> {
        let sender = self
            .runs
            .lock()
            .map_err(|_| {
                GrpcError::new(
                    "stream_manager",
                    "Não foi possível acessar o controle do streaming gRPC.",
                    "O mutex de execuções gRPC está indisponível.",
                )
            })?
            .get(run_id)
            .cloned()
            .ok_or_else(|| {
                GrpcError::new(
                    "stream_not_found",
                    "O streaming gRPC não está mais em execução.",
                    format!("Run ID recebido: {run_id}"),
                )
            })?;

        sender.send(true).map_err(|_| {
            GrpcError::new(
                "stream_cancel",
                "Não foi possível sinalizar o cancelamento do streaming gRPC.",
                "O consumidor do sinal de cancelamento foi encerrado.",
            )
        })
    }

    pub fn remove(&self, run_id: &str) {
        if let Ok(mut runs) = self.runs.lock() {
            runs.remove(run_id);
        }
    }
}

fn load_pool(path: &str) -> Result<(PathBuf, DescriptorPool), GrpcError> {
    let source = path.trim();
    if source.is_empty() {
        return Err(GrpcError::new(
            "proto_path",
            "Selecione um arquivo .proto.",
            "O caminho recebido está vazio.",
        ));
    }

    let source_path = Path::new(source);
    if !source_path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("proto"))
    {
        return Err(GrpcError::new(
            "proto_path",
            "O arquivo precisa ter a extensão .proto.",
            format!("Extensão inválida para: {source}"),
        ));
    }

    let canonical = source_path.canonicalize().map_err(|error| {
        GrpcError::new(
            "proto_path",
            "Não foi possível abrir o arquivo .proto.",
            error.to_string(),
        )
    })?;

    let metadata = std::fs::metadata(&canonical).map_err(|error| {
        GrpcError::new(
            "proto_path",
            "Não foi possível ler as informações do arquivo .proto.",
            error.to_string(),
        )
    })?;
    if !metadata.is_file() {
        return Err(GrpcError::new(
            "proto_path",
            "O caminho selecionado não aponta para um arquivo.",
            canonical.display().to_string(),
        ));
    }
    if metadata.len() > MAX_PROTO_BYTES {
        return Err(GrpcError::new(
            "proto_size",
            "O arquivo .proto excede o limite local de 5 MiB.",
            format!("Tamanho recebido: {} bytes.", metadata.len()),
        ));
    }

    let include_root = canonical.parent().unwrap_or_else(|| Path::new("."));
    let descriptor_set =
        protox::compile([canonical.as_path()], [include_root]).map_err(|error| {
            GrpcError::new(
                "proto_compile",
                "Não foi possível interpretar o arquivo .proto.",
                error.to_string(),
            )
        })?;
    let pool = DescriptorPool::from_file_descriptor_set(descriptor_set).map_err(|error| {
        GrpcError::new(
            "proto_descriptor",
            "O arquivo foi compilado, mas seus descriptors são inválidos.",
            error.to_string(),
        )
    })?;

    Ok((canonical, pool))
}

pub fn inspect_proto(path: &str) -> Result<GrpcSchema, GrpcError> {
    let (canonical, pool) = load_pool(path)?;
    Ok(schema_from_pool(canonical.display().to_string(), pool))
}

pub async fn inspect_reflection(request: GrpcReflectionRequest) -> Result<GrpcSchema, GrpcError> {
    let pool = load_reflection_pool(&request).await?;
    Ok(schema_from_pool("gRPC Reflection".to_string(), pool))
}

async fn load_pool_for_source(
    proto_path: &str,
    reflection: Option<&GrpcReflectionRequest>,
) -> Result<DescriptorPool, GrpcError> {
    if proto_path.trim().is_empty() {
        let reflection = reflection.ok_or_else(|| {
            GrpcError::new(
                "grpc_source",
                "Selecione um .proto local ou configure gRPC Reflection.",
                "Nenhuma fonte de descriptors foi enviada.",
            )
        })?;
        load_reflection_pool(reflection).await
    } else {
        load_pool(proto_path).map(|(_, pool)| pool)
    }
}

fn schema_from_pool(source_path: String, pool: DescriptorPool) -> GrpcSchema {
    let mut services = pool
        .services()
        .map(|service| {
            let mut methods = service
                .methods()
                .map(|method| GrpcMethod {
                    name: method.name().to_string(),
                    full_name: method.full_name().to_string(),
                    input_type: method.input().full_name().to_string(),
                    output_type: method.output().full_name().to_string(),
                    client_streaming: method.is_client_streaming(),
                    server_streaming: method.is_server_streaming(),
                })
                .collect::<Vec<_>>();
            methods.sort_by(|left, right| left.name.cmp(&right.name));

            GrpcService {
                name: service.name().to_string(),
                full_name: service.full_name().to_string(),
                methods,
            }
        })
        .collect::<Vec<_>>();
    services.sort_by(|left, right| left.full_name.cmp(&right.full_name));

    GrpcSchema {
        source_path,
        services,
    }
}

fn endpoint_from_url(raw_url: &str) -> Result<Endpoint, GrpcError> {
    let endpoint_url = raw_url.trim();
    if !(endpoint_url.starts_with("http://") || endpoint_url.starts_with("https://")) {
        return Err(GrpcError::new(
            "grpc_url",
            "Informe uma URL gRPC http:// ou https:// válida.",
            format!("Esquema não suportado: {endpoint_url}"),
        ));
    }

    Endpoint::from_shared(endpoint_url.to_string())
        .map(|endpoint| {
            endpoint
                .connect_timeout(REFLECTION_TIMEOUT)
                .timeout(REFLECTION_TIMEOUT)
        })
        .map_err(|error| {
            GrpcError::new(
                "grpc_url",
                "Informe uma URL gRPC http:// ou https:// válida.",
                error.to_string(),
            )
        })
}

async fn reflection_exchange(
    request: &GrpcReflectionRequest,
    message_request: MessageRequest,
) -> Result<Vec<tonic_reflection::pb::v1::ServerReflectionResponse>, GrpcError> {
    let endpoint = endpoint_from_url(&request.url)?;
    let channel = endpoint.connect().await.map_err(|error| {
        GrpcError::new(
            "reflection_connect",
            "Não foi possível conectar ao endpoint gRPC para usar Reflection.",
            error.to_string(),
        )
    })?;
    let mut client = ServerReflectionClient::new(channel);
    let mut tonic_request = Request::new(stream::iter([ServerReflectionRequest {
        host: request.host.trim().to_string(),
        message_request: Some(message_request),
    }]));
    *tonic_request.metadata_mut() = build_metadata(&request.metadata)?;

    let response = timeout(
        REFLECTION_TIMEOUT,
        client.server_reflection_info(tonic_request),
    )
    .await
    .map_err(|_| {
        GrpcError::new(
            "reflection_timeout",
            "A consulta gRPC Reflection excedeu o timeout local.",
            format!("Limite: {} segundos.", REFLECTION_TIMEOUT.as_secs()),
        )
    })?
    .map_err(|error| {
        GrpcError::new(
            "reflection_call",
            "O endpoint gRPC não aceitou a chamada de Reflection.",
            error.to_string(),
        )
    })?;

    let mut stream = response.into_inner();
    let mut responses = Vec::new();
    while let Some(response) = timeout(REFLECTION_TIMEOUT, stream.message())
        .await
        .map_err(|_| {
            GrpcError::new(
                "reflection_timeout",
                "A resposta gRPC Reflection excedeu o timeout local.",
                format!("Limite: {} segundos.", REFLECTION_TIMEOUT.as_secs()),
            )
        })?
        .map_err(|error| {
            GrpcError::new(
                "reflection_receive",
                "A resposta gRPC Reflection falhou durante o recebimento.",
                error.to_string(),
            )
        })?
    {
        responses.push(response);
    }

    Ok(responses)
}

fn reflection_error(code: i32, message: String) -> GrpcError {
    GrpcError::new(
        "reflection_server",
        "O servidor gRPC retornou um erro de Reflection.",
        format!("Código {code}: {message}"),
    )
}

async fn load_reflection_pool(
    request: &GrpcReflectionRequest,
) -> Result<DescriptorPool, GrpcError> {
    let service_responses = reflection_exchange(
        request,
        MessageRequest::ListServices(request.host.trim().to_string()),
    )
    .await?;

    let mut service_names = Vec::new();
    for response in service_responses {
        match response.message_response {
            Some(MessageResponse::ListServicesResponse(list)) => {
                service_names.extend(list.service.into_iter().map(|service| service.name));
            }
            Some(MessageResponse::ErrorResponse(error)) => {
                return Err(reflection_error(error.error_code, error.error_message));
            }
            _ => {}
        }
    }

    service_names.sort();
    service_names.dedup();
    if service_names.is_empty() {
        return Err(GrpcError::new(
            "reflection_empty",
            "O endpoint não publicou services por gRPC Reflection.",
            "A resposta list_services não contém services.",
        ));
    }
    if service_names.len() > MAX_REFLECTION_SERVICES {
        return Err(GrpcError::new(
            "reflection_limit",
            "O endpoint excede o limite local de services descobertos.",
            format!(
                "Quantidade recebida: {}, limite: {}.",
                service_names.len(),
                MAX_REFLECTION_SERVICES
            ),
        ));
    }

    let mut descriptor_payloads = Vec::new();
    let mut descriptor_bytes = 0_usize;
    for service_name in service_names {
        let responses =
            reflection_exchange(request, MessageRequest::FileContainingSymbol(service_name))
                .await?;

        for response in responses {
            match response.message_response {
                Some(MessageResponse::FileDescriptorResponse(files)) => {
                    for bytes in files.file_descriptor_proto {
                        descriptor_bytes = descriptor_bytes.saturating_add(bytes.len());
                        if descriptor_bytes > MAX_REFLECTION_DESCRIPTOR_BYTES {
                            return Err(GrpcError::new(
                                "reflection_size",
                                "Os descriptors gRPC excedem o limite local de 16 MiB.",
                                format!("Tamanho recebido: {descriptor_bytes} bytes."),
                            ));
                        }

                        descriptor_payloads.push(bytes);
                    }
                }
                Some(MessageResponse::ErrorResponse(error)) => {
                    return Err(reflection_error(error.error_code, error.error_message));
                }
                _ => {}
            }
        }
    }

    descriptor_pool_from_reflection(descriptor_payloads)
}

fn descriptor_pool_from_reflection(
    descriptor_payloads: Vec<Vec<u8>>,
) -> Result<DescriptorPool, GrpcError> {
    let mut descriptors = BTreeMap::new();
    for bytes in descriptor_payloads {
        let descriptor =
            prost_types::FileDescriptorProto::decode(bytes.as_slice()).map_err(|error| {
                GrpcError::new(
                    "reflection_descriptor",
                    "O servidor retornou um descriptor gRPC inválido.",
                    error.to_string(),
                )
            })?;
        let name = descriptor.name.clone().ok_or_else(|| {
            GrpcError::new(
                "reflection_descriptor",
                "O servidor retornou um descriptor sem nome de arquivo.",
                "FileDescriptorProto.name está ausente.",
            )
        })?;
        descriptors.insert(name, descriptor);
    }

    if descriptors.is_empty() {
        return Err(GrpcError::new(
            "reflection_empty",
            "O endpoint não retornou descriptors para os services descobertos.",
            "Nenhum FileDescriptorProto foi recebido.",
        ));
    }

    let descriptor_set = FileDescriptorSet {
        file: descriptors.into_values().collect(),
    };
    DescriptorPool::decode(descriptor_set.encode_to_vec().as_slice()).map_err(|error| {
        GrpcError::new(
            "reflection_descriptor",
            "Os descriptors recebidos não formam um contrato gRPC válido.",
            error.to_string(),
        )
    })
}

fn method_path(
    service: &prost_reflect::ServiceDescriptor,
    method: &prost_reflect::MethodDescriptor,
) -> Result<PathAndQuery, GrpcError> {
    let path = format!("/{}/{}", service.full_name(), method.name());
    PathAndQuery::from_maybe_shared(path).map_err(|error| {
        GrpcError::new(
            "grpc_path",
            "Não foi possível montar o caminho do método gRPC.",
            error.to_string(),
        )
    })
}

fn decode_stream_messages(
    method: &prost_reflect::MethodDescriptor,
    messages: Vec<Value>,
) -> Result<Vec<DynamicMessage>, GrpcError> {
    if messages.is_empty() {
        return Err(GrpcError::new(
            "stream_body",
            "Informe pelo menos uma mensagem JSON para o streaming.",
            "A lista messages está vazia.",
        ));
    }
    if messages.len() > MAX_STREAM_MESSAGES {
        return Err(GrpcError::new(
            "stream_limit",
            "O streaming excede o limite local de 512 mensagens.",
            format!("Quantidade recebida: {}.", messages.len()),
        ));
    }

    messages
        .into_iter()
        .enumerate()
        .map(|(index, body)| {
            let body_json = body.to_string();
            let mut deserializer = serde_json::Deserializer::from_str(&body_json);
            let message = DynamicMessage::deserialize(method.input(), &mut deserializer).map_err(
                |error| {
                    GrpcError::new(
                        "stream_body",
                        format!(
                            "A mensagem {} não corresponde ao tipo de entrada do método.",
                            index + 1
                        ),
                        error.to_string(),
                    )
                },
            )?;
            deserializer.end().map_err(|error| {
                GrpcError::new(
                    "stream_body",
                    format!(
                        "A mensagem {} contém dados após o objeto principal.",
                        index + 1
                    ),
                    error.to_string(),
                )
            })?;
            Ok(message)
        })
        .collect()
}

fn dynamic_message_to_value(message: DynamicMessage) -> Result<Value, GrpcError> {
    serde_json::to_value(message).map_err(|error| {
        GrpcError::new(
            "grpc_response",
            "A resposta gRPC não pôde ser convertida para JSON.",
            error.to_string(),
        )
    })
}

async fn collect_stream_messages(
    mut stream: tonic::Streaming<DynamicMessage>,
) -> Result<(Vec<Value>, Vec<GrpcMetadataEntry>), GrpcError> {
    let mut messages = Vec::new();

    while let Some(message) = timeout(STREAM_IDLE_TIMEOUT, stream.message())
        .await
        .map_err(|_| {
            GrpcError::new(
                "stream_timeout",
                "O streaming gRPC ficou sem resposta além do timeout local.",
                format!(
                    "Limite de inatividade: {} segundos.",
                    STREAM_IDLE_TIMEOUT.as_secs()
                ),
            )
        })?
        .map_err(|error| {
            GrpcError::new(
                "stream_receive",
                "O streaming gRPC falhou durante o recebimento.",
                error.to_string(),
            )
        })?
    {
        if messages.len() >= MAX_STREAM_MESSAGES {
            return Err(GrpcError::new(
                "stream_limit",
                "O servidor excedeu o limite local de 512 mensagens recebidas.",
                format!("Limite: {MAX_STREAM_MESSAGES} mensagens."),
            ));
        }
        messages.push(dynamic_message_to_value(message)?);
    }

    let trailers = timeout(STREAM_IDLE_TIMEOUT, stream.trailers())
        .await
        .map_err(|_| {
            GrpcError::new(
                "stream_timeout",
                "Os trailers do streaming gRPC excederam o timeout local.",
                format!(
                    "Limite de inatividade: {} segundos.",
                    STREAM_IDLE_TIMEOUT.as_secs()
                ),
            )
        })?
        .map_err(|error| {
            GrpcError::new(
                "stream_trailers",
                "Os trailers do streaming gRPC não puderam ser lidos.",
                error.to_string(),
            )
        })?
        .map(|metadata| serialize_metadata(&metadata))
        .unwrap_or_default();

    Ok((messages, trailers))
}

fn find_service<'a>(
    pool: &'a DescriptorPool,
    name: &str,
) -> Option<prost_reflect::ServiceDescriptor> {
    let normalized = name.trim().trim_start_matches('.');
    pool.get_service_by_name(normalized)
}

fn find_method(
    service: &prost_reflect::ServiceDescriptor,
    name: &str,
) -> Option<prost_reflect::MethodDescriptor> {
    let normalized = name.trim().trim_start_matches('.');
    service
        .methods()
        .find(|method| method.name() == normalized || method.full_name() == normalized)
}

fn build_metadata(entries: &[GrpcMetadataEntry]) -> Result<MetadataMap, GrpcError> {
    let enabled = entries
        .iter()
        .filter(|entry| entry.enabled && !entry.name.trim().is_empty())
        .collect::<Vec<_>>();
    if enabled.len() > MAX_METADATA {
        return Err(GrpcError::new(
            "metadata_limit",
            "A chamada excede o limite local de 128 metadados.",
            format!("Quantidade recebida: {}.", enabled.len()),
        ));
    }

    let mut metadata = MetadataMap::new();
    for entry in enabled {
        let name = entry.name.trim().to_ascii_lowercase();
        if name.ends_with("-bin") {
            return Err(GrpcError::new(
                "metadata_binary",
                "Metadados binários ainda não são suportados neste lote.",
                format!("Chave recebida: {name}"),
            ));
        }
        let key = AsciiMetadataKey::from_bytes(name.as_bytes()).map_err(|error| {
            GrpcError::new(
                "metadata_name",
                "O nome de um metadado gRPC é inválido.",
                error.to_string(),
            )
        })?;
        let value = AsciiMetadataValue::try_from(entry.value.as_str()).map_err(|error| {
            GrpcError::new(
                "metadata_value",
                "O valor de um metadado gRPC precisa ser ASCII válido.",
                error.to_string(),
            )
        })?;
        metadata.insert(key, value);
    }

    Ok(metadata)
}

fn serialize_metadata(metadata: &MetadataMap) -> Vec<GrpcMetadataEntry> {
    metadata
        .iter()
        .map(|entry| match entry {
            KeyAndValueRef::Ascii(key, value) => GrpcMetadataEntry {
                name: key.as_str().to_string(),
                value: value.to_str().unwrap_or("<valor não textual>").to_string(),
                enabled: true,
            },
            KeyAndValueRef::Binary(key, value) => GrpcMetadataEntry {
                name: key.as_str().to_string(),
                value: format!("<binário: {} bytes>", value.as_encoded_bytes().len()),
                enabled: true,
            },
        })
        .collect()
}

async fn execute_stream_inner(
    request: GrpcStreamingRequest,
) -> Result<GrpcStreamingResponse, GrpcError> {
    let pool = load_pool_for_source(&request.proto_path, request.reflection.as_ref()).await?;
    let service = find_service(&pool, &request.service).ok_or_else(|| {
        GrpcError::new(
            "service_not_found",
            "O serviço selecionado não existe no contrato gRPC.",
            format!("Serviço recebido: {}", request.service),
        )
    })?;
    let method = find_method(&service, &request.method).ok_or_else(|| {
        GrpcError::new(
            "method_not_found",
            "O método selecionado não existe no serviço gRPC.",
            format!("Método recebido: {}", request.method),
        )
    })?;

    if !method.is_client_streaming() && !method.is_server_streaming() {
        return Err(GrpcError::new(
            "stream_method",
            "O método selecionado é unary e deve ser executado no modo normal.",
            format!("Método recebido: {}", method.full_name()),
        ));
    }

    let mut inputs = decode_stream_messages(&method, request.messages)?;
    if method.is_server_streaming() && !method.is_client_streaming() && inputs.len() != 1 {
        return Err(GrpcError::new(
            "stream_body",
            "Server streaming exige exatamente uma mensagem de entrada.",
            format!("Quantidade recebida: {}.", inputs.len()),
        ));
    }

    let endpoint = endpoint_from_url(&request.url)?;
    let channel = endpoint.connect().await.map_err(|error| {
        GrpcError::new(
            "grpc_connect",
            "Não foi possível conectar ao endpoint gRPC.",
            error.to_string(),
        )
    })?;

    let metadata = build_metadata(&request.metadata)?;
    let path = method_path(&service, &method)?;
    let codec = DynamicGrpcCodec {
        output: method.output(),
    };
    let started = Instant::now();
    let mut grpc = Grpc::new(channel);

    let (messages, response_metadata, trailers) =
        match (method.is_client_streaming(), method.is_server_streaming()) {
            (false, true) => {
                let input = inputs.remove(0);
                let mut tonic_request = Request::new(input);
                *tonic_request.metadata_mut() = metadata;
                let response = grpc
                    .server_streaming(tonic_request, path, codec)
                    .await
                    .map_err(|error| {
                        GrpcError::new(
                            "stream_call",
                            "A chamada server streaming gRPC falhou.",
                            error.to_string(),
                        )
                    })?;
                let response_metadata = serialize_metadata(response.metadata());
                let (messages, trailers) = collect_stream_messages(response.into_inner()).await?;
                (messages, response_metadata, trailers)
            }
            (true, false) => {
                let mut tonic_request = Request::new(stream::iter(inputs));
                *tonic_request.metadata_mut() = metadata;
                let response = grpc
                    .client_streaming(tonic_request, path, codec)
                    .await
                    .map_err(|error| {
                        GrpcError::new(
                            "stream_call",
                            "A chamada client streaming gRPC falhou.",
                            error.to_string(),
                        )
                    })?;
                let response_metadata = serialize_metadata(response.metadata());
                let message = dynamic_message_to_value(response.into_inner())?;
                (vec![message], response_metadata, Vec::new())
            }
            (true, true) => {
                let mut tonic_request = Request::new(stream::iter(inputs));
                *tonic_request.metadata_mut() = metadata;
                let response =
                    grpc.streaming(tonic_request, path, codec)
                        .await
                        .map_err(|error| {
                            GrpcError::new(
                                "stream_call",
                                "A chamada bidirectional streaming gRPC falhou.",
                                error.to_string(),
                            )
                        })?;
                let response_metadata = serialize_metadata(response.metadata());
                let (messages, trailers) = collect_stream_messages(response.into_inner()).await?;
                (messages, response_metadata, trailers)
            }
            (false, false) => unreachable!("unary methods are rejected above"),
        };

    Ok(GrpcStreamingResponse {
        status: "OK".to_string(),
        message_count: messages.len(),
        messages,
        duration_ms: started.elapsed().as_millis() as u64,
        response_metadata,
        trailers,
    })
}

async fn wait_for_stream_cancellation(mut cancellation: watch::Receiver<bool>) {
    while !*cancellation.borrow() {
        if cancellation.changed().await.is_err() {
            return;
        }
    }
}

fn stream_cancelled() -> GrpcError {
    GrpcError::new(
        "cancelled",
        "O streaming gRPC foi cancelado localmente.",
        "A execução foi interrompida antes de finalizar o recebimento.",
    )
}

pub async fn execute_stream(
    request: GrpcStreamingRequest,
    cancellation: watch::Receiver<bool>,
) -> Result<GrpcStreamingResponse, GrpcError> {
    tokio::select! {
        result = execute_stream_inner(request) => result,
        _ = wait_for_stream_cancellation(cancellation) => Err(stream_cancelled()),
    }
}

pub async fn execute_unary(request: GrpcUnaryRequest) -> Result<GrpcUnaryResponse, GrpcError> {
    let pool = load_pool_for_source(&request.proto_path, request.reflection.as_ref()).await?;
    let service = find_service(&pool, &request.service).ok_or_else(|| {
        GrpcError::new(
            "service_not_found",
            "O serviço selecionado não existe no .proto.",
            format!("Serviço recebido: {}", request.service),
        )
    })?;
    let method = find_method(&service, &request.method).ok_or_else(|| {
        GrpcError::new(
            "method_not_found",
            "O método selecionado não existe no serviço.",
            format!("Método recebido: {}", request.method),
        )
    })?;

    if method.is_client_streaming() || method.is_server_streaming() {
        return Err(GrpcError::new(
            "streaming_not_supported",
            "Este lote executa apenas métodos gRPC unary.",
            format!("Método recebido: {}", method.full_name()),
        ));
    }

    let endpoint = endpoint_from_url(&request.url)?;
    let channel = endpoint.connect().await.map_err(|error| {
        GrpcError::new(
            "grpc_connect",
            "Não foi possível conectar ao endpoint gRPC.",
            error.to_string(),
        )
    })?;

    let body_json = request.body.to_string();
    let mut deserializer = serde_json::Deserializer::from_str(&body_json);
    let input =
        DynamicMessage::deserialize(method.input(), &mut deserializer).map_err(|error| {
            GrpcError::new(
                "grpc_body",
                "O body não corresponde ao tipo de entrada do método.",
                error.to_string(),
            )
        })?;
    deserializer.end().map_err(|error| {
        GrpcError::new(
            "grpc_body",
            "O body JSON contém dados após o objeto principal.",
            error.to_string(),
        )
    })?;

    let metadata = build_metadata(&request.metadata)?;
    let mut tonic_request = Request::new(input);
    *tonic_request.metadata_mut() = metadata;
    let path = method_path(&service, &method)?;

    let started = Instant::now();
    let mut grpc = Grpc::new(channel);
    let response = grpc
        .unary::<DynamicMessage, DynamicMessage, _>(
            tonic_request,
            path,
            DynamicGrpcCodec {
                output: method.output(),
            },
        )
        .await
        .map_err(|error| {
            GrpcError::new("grpc_call", "A chamada gRPC falhou.", error.to_string())
        })?;
    let duration_ms = started.elapsed().as_millis() as u64;
    let response_metadata = serialize_metadata(response.metadata());
    let body = serde_json::to_value(response.into_inner()).map_err(|error| {
        GrpcError::new(
            "grpc_response",
            "A resposta gRPC não pôde ser convertida para JSON.",
            error.to_string(),
        )
    })?;

    Ok(GrpcUnaryResponse {
        status: "OK".to_string(),
        body,
        duration_ms,
        response_metadata,
        trailers: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fixture_path(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!("larry-grpc-{name}"));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("payments.proto");
        fs::write(
            &path,
            r#"syntax = "proto3";
package payments;

message CreatePaymentRequest { string id = 1; int32 amount = 2; }
message CreatePaymentResponse { string status = 1; }

service PaymentService {
  rpc CreatePayment(CreatePaymentRequest) returns (CreatePaymentResponse);
  rpc StreamPayments(CreatePaymentRequest) returns (stream CreatePaymentResponse);
}
"#,
        )
        .unwrap();
        path
    }

    #[test]
    fn descobre_services_metodos_e_streaming() {
        let path = fixture_path("inspect");
        let schema = inspect_proto(path.to_str().unwrap()).unwrap();

        assert_eq!(schema.services.len(), 1);
        assert_eq!(schema.services[0].full_name, "payments.PaymentService");
        assert_eq!(schema.services[0].methods.len(), 2);
        assert_eq!(schema.services[0].methods[0].name, "CreatePayment");
        assert_eq!(
            schema.services[0].methods[0].input_type,
            "payments.CreatePaymentRequest"
        );
        assert!(!schema.services[0].methods[0].server_streaming);
        assert!(schema.services[0].methods[1].server_streaming);
    }

    #[test]
    fn rejeita_arquivo_que_nao_e_proto() {
        let path = std::env::temp_dir().join("larry-not-proto.txt");
        fs::write(&path, "not a proto").unwrap();
        let error = inspect_proto(path.to_str().unwrap()).unwrap_err();

        assert_eq!(error.kind, "proto_path");
        assert!(error.message.contains("extensão"));
    }

    #[test]
    fn rejeita_metadado_binario_neste_lote() {
        let error = build_metadata(&[GrpcMetadataEntry {
            name: "trace-bin".to_string(),
            value: "AQI=".to_string(),
            enabled: true,
        }])
        .unwrap_err();

        assert_eq!(error.kind, "metadata_binary");
    }

    #[test]
    fn monta_pool_a_partir_de_descriptors_reflection() {
        let path = fixture_path("reflection-pool");
        let descriptor_set = protox::compile([path.as_path()], [path.parent().unwrap()]).unwrap();
        let payloads = descriptor_set
            .file
            .into_iter()
            .map(|file| file.encode_to_vec())
            .collect();

        let pool = descriptor_pool_from_reflection(payloads).unwrap();
        let schema = schema_from_pool("fixture".to_string(), pool);

        assert_eq!(schema.services[0].full_name, "payments.PaymentService");
        assert_eq!(schema.services[0].methods.len(), 2);
    }

    #[test]
    fn rejeita_descriptor_reflection_invalido() {
        let error = descriptor_pool_from_reflection(vec![vec![0xff]]).unwrap_err();

        assert_eq!(error.kind, "reflection_descriptor");
    }

    #[test]
    fn decodifica_lista_de_mensagens_streaming() {
        let path = fixture_path("stream-body");
        let (_, pool) = load_pool(path.to_str().unwrap()).unwrap();
        let service = find_service(&pool, "payments.PaymentService").unwrap();
        let method = find_method(&service, "StreamPayments").unwrap();

        let messages = decode_stream_messages(
            &method,
            vec![serde_json::json!({ "id": "first", "amount": 10 })],
        )
        .unwrap();

        assert_eq!(messages.len(), 1);
        assert_eq!(
            messages[0].get_field_by_name("id").unwrap().as_str(),
            Some("first")
        );
    }

    #[test]
    fn rejeita_lista_streaming_vazia() {
        let path = fixture_path("empty-stream-body");
        let (_, pool) = load_pool(path.to_str().unwrap()).unwrap();
        let service = find_service(&pool, "payments.PaymentService").unwrap();
        let method = find_method(&service, "StreamPayments").unwrap();

        let error = decode_stream_messages(&method, Vec::new()).unwrap_err();

        assert_eq!(error.kind, "stream_body");
    }

    #[test]
    fn cancela_streaming_por_run_id() {
        let manager = GrpcStreamManager::default();
        let mut cancellation = manager.begin("stream-test".to_string()).unwrap();

        manager.cancel("stream-test").unwrap();

        assert!(*cancellation.borrow_and_update());
        manager.remove("stream-test");
        assert!(manager.cancel("stream-test").is_err());
    }

    #[tokio::test]
    async fn rejeita_streaming_antes_de_abrir_conexao() {
        let path = fixture_path("streaming");
        let error = execute_unary(GrpcUnaryRequest {
            proto_path: path.to_string_lossy().to_string(),
            url: "http://127.0.0.1:1".to_string(),
            service: "payments.PaymentService".to_string(),
            method: "StreamPayments".to_string(),
            body: serde_json::json!({}),
            metadata: vec![],
            reflection: None,
        })
        .await
        .unwrap_err();

        assert_eq!(error.kind, "streaming_not_supported");
    }
}
