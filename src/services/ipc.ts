import { invoke } from "@tauri-apps/api/core";

import type {
  AppInfo,
  CollectionFile,
  CurlImportPreview,
  EnvironmentFile,
  GrpcSchema,
  GrpcUnaryRequest,
  GrpcUnaryResponse,
  GitSnapshot,
  HistoryComparison,
  HistoryEntry,
  HistorySummary,
  HttpProtocolComparison,
  HttpResponse,
  PostmanImportPreview,
  ProfilerConfig,
  ProfilerStart,
  RequestDefinition,
  SseOpenRequest,
  SseSession,
  WebSocketOpenRequest,
  WebSocketSession,
} from "../types/api";

export function getAppInfo() {
  return invoke<AppInfo>("get_app_info");
}

export function getSampleRequest() {
  return invoke<RequestDefinition>("get_sample_request");
}

export function executeRequest(
  request: RequestDefinition,
  environment: EnvironmentFile | null = null,
) {
  return invoke<HttpResponse>("execute_request", { request, environment });
}

export function compareHttpProtocols(
  request: RequestDefinition,
  environment: EnvironmentFile | null,
) {
  return invoke<HttpProtocolComparison>("compare_http_protocols", { request, environment });
}

export function startProfiler(
  request: RequestDefinition,
  environment: EnvironmentFile | null,
  config: ProfilerConfig,
) {
  return invoke<ProfilerStart>("start_profiler", { request, environment, config });
}

export function cancelProfiler(runId: string) {
  return invoke<void>("cancel_profiler", { runId });
}

export function listHistory(limit = 20) {
  return invoke<HistorySummary[]>("list_history_entries", { limit });
}

export function getHistoryEntry(id: string) {
  return invoke<HistoryEntry>("get_history_entry_command", { id });
}

export function compareHistoryEntries(leftId: string, rightId: string) {
  return invoke<HistoryComparison>("compare_history_entry_command", {
    leftId,
    rightId,
  });
}

export function saveCollection(path: string, collection: CollectionFile) {
  return invoke<void>("save_collection_file", { path, collection });
}

export function loadCollection(path: string) {
  return invoke<CollectionFile>("load_collection_file", { path });
}

export function saveEnvironment(path: string, environment: EnvironmentFile) {
  return invoke<void>("save_environment_file", { path, environment });
}

export function loadEnvironment(path: string) {
  return invoke<EnvironmentFile>("load_environment_file", { path });
}

export function setEnvironmentSecret(secretRef: string, value: string) {
  return invoke<void>("set_environment_secret", { secretRef, value });
}

export function deleteEnvironmentSecret(secretRef: string) {
  return invoke<void>("delete_environment_secret", { secretRef });
}

export function getGitSnapshot(path: string) {
  return invoke<GitSnapshot>("get_git_snapshot", { path });
}

export function previewPostmanCollection(path: string) {
  return invoke<PostmanImportPreview>("preview_postman_collection", { path });
}

export function parseCurlRequest(command: string) {
  return invoke<CurlImportPreview>("parse_curl_request", { command });
}

export function inspectGrpcProto(path: string) {
  return invoke<GrpcSchema>("inspect_grpc_proto", { path });
}

export function executeGrpcUnary(request: GrpcUnaryRequest) {
  return invoke<GrpcUnaryResponse>("execute_grpc_unary", { request });
}

export function openWebSocket(request: WebSocketOpenRequest) {
  return invoke<WebSocketSession>("open_websocket", { request });
}

export function sendWebSocketMessage(sessionId: string, message: string) {
  return invoke<void>("send_websocket_message", { sessionId, message });
}

export function closeWebSocket(sessionId: string) {
  return invoke<void>("close_websocket", { sessionId });
}

export function openSse(request: SseOpenRequest) {
  return invoke<SseSession>("open_sse", { request });
}

export function closeSse(sessionId: string) {
  return invoke<void>("close_sse", { sessionId });
}
