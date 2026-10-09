use std::path::{Path, PathBuf};
use std::time::Instant;

use prost::Message;
use prost_reflect::{DescriptorPool, DynamicMessage};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tonic::client::Grpc;
use tonic::codec::{BufferSettings, Codec, DecodeBuf, Decoder, EncodeBuf, Encoder};
use tonic::codegen::http::uri::PathAndQuery;
use tonic::metadata::{AsciiMetadataKey, AsciiMetadataValue, KeyAndValueRef, MetadataMap};
use tonic::transport::Endpoint;
use tonic::Request;
use tonic::Status;

const MAX_PROTO_BYTES: u64 = 5 * 1024 * 1024;
const MAX_METADATA: usize = 128;

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

    Ok(GrpcSchema {
        source_path: canonical.display().to_string(),
        services,
    })
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

pub async fn execute_unary(request: GrpcUnaryRequest) -> Result<GrpcUnaryResponse, GrpcError> {
    let (_, pool) = load_pool(&request.proto_path)?;
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

    let endpoint_url = request.url.trim();
    if !(endpoint_url.starts_with("http://") || endpoint_url.starts_with("https://")) {
        return Err(GrpcError::new(
            "grpc_url",
            "Informe uma URL gRPC http:// ou https:// válida.",
            format!("Esquema não suportado: {endpoint_url}"),
        ));
    }
    let endpoint = Endpoint::from_shared(endpoint_url.to_string()).map_err(|error| {
        GrpcError::new(
            "grpc_url",
            "Informe uma URL gRPC http:// ou https:// válida.",
            error.to_string(),
        )
    })?;
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
    let path = format!("/{}/{}", service.full_name(), method.name());
    let path = PathAndQuery::from_maybe_shared(path).map_err(|error| {
        GrpcError::new(
            "grpc_path",
            "Não foi possível montar o caminho do método gRPC.",
            error.to_string(),
        )
    })?;

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
        })
        .await
        .unwrap_err();

        assert_eq!(error.kind, "streaming_not_supported");
    }
}
