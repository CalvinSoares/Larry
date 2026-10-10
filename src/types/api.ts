export interface AppInfo {
  appName: string;
  version: string;
  operatingSystem: string;
  architecture: string;
}

export interface HeaderEntry {
  name: string;
  value: string;
  enabled: boolean;
}

export interface QueryParam {
  name: string;
  value: string;
  enabled: boolean;
}

export interface CookieEntry {
  name: string;
  value: string;
  enabled: boolean;
}

export interface FormField {
  name: string;
  value: string;
  enabled: boolean;
}

export interface MultipartFile {
  name: string;
  path: string;
  enabled: boolean;
}

export type RequestBody =
  | {
      type: "json";
      value: unknown;
    }
  | {
      type: "text";
      value: string;
    }
  | {
      type: "form-urlencoded";
      value: FormField[];
    }
  | {
      type: "multipart";
      value: {
        fields: FormField[];
        files: MultipartFile[];
      };
    };

export type RequestAuth =
  | {
      type: "bearer";
      token: string;
    }
  | {
      type: "basic";
      username: string;
      password: string;
    }
  | {
      type: "apiKey";
      name: string;
      value: string;
      location: "header" | "query";
    };

export type AssertionDefinition =
  | {
      type: "statusEquals";
      expected: number;
    }
  | {
      type: "headerContains";
      name: string;
      value: string;
    }
  | {
      type: "bodyContains";
      value: string;
    };

export interface RequestDefinition {
  id: string;
  name: string;
  method: string;
  url: string;
  query: QueryParam[];
  headers: HeaderEntry[];
  cookies: CookieEntry[];
  body: RequestBody | null;
  auth: RequestAuth | null;
  assertions: AssertionDefinition[];
}

export interface CollectionFolder {
  id: string;
  name: string;
  requests: RequestDefinition[];
  folders: CollectionFolder[];
}

export interface CollectionFile {
  schemaVersion: number;
  name: string;
  requests: RequestDefinition[];
  folders: CollectionFolder[];
}

export interface EnvironmentVariable {
  name: string;
  value: string | null;
  secretRef: string | null;
}

export interface EnvironmentFile {
  schemaVersion: number;
  name: string;
  variables: EnvironmentVariable[];
}

export interface PostmanImportPreview {
  sourceName: string;
  collection: CollectionFile;
  requestCount: number;
  warnings: string[];
}

export interface CurlImportPreview {
  request: RequestDefinition;
  warnings: string[];
}

export interface WebSocketOpenRequest {
  url: string;
  headers: HeaderEntry[];
}

export interface WebSocketSession {
  sessionId: string;
  url: string;
  openedAtMs: number;
}

export interface WebSocketEvent {
  sessionId: string;
  eventType: "opened" | "message" | "closed" | "error";
  direction: "incoming" | "outgoing" | null;
  payload: string | null;
  binary: boolean;
  sizeBytes: number;
  timestampMs: number;
  technical: string | null;
}

export interface SseOpenRequest {
  url: string;
  headers: HeaderEntry[];
}

export interface SseSession {
  sessionId: string;
  url: string;
  openedAtMs: number;
  contentType: string | null;
}

export interface SseEvent {
  sessionId: string;
  eventType: "opened" | "message" | "closed" | "error";
  eventName: string | null;
  data: string | null;
  eventId: string | null;
  retryMs: number | null;
  sizeBytes: number;
  timestampMs: number;
  technical: string | null;
}

export interface GrpcMetadataEntry {
  name: string;
  value: string;
  enabled: boolean;
}

export interface GrpcMethod {
  name: string;
  fullName: string;
  inputType: string;
  outputType: string;
  clientStreaming: boolean;
  serverStreaming: boolean;
}

export interface GrpcService {
  name: string;
  fullName: string;
  methods: GrpcMethod[];
}

export interface GrpcSchema {
  sourcePath: string;
  services: GrpcService[];
}

export interface GrpcReflectionRequest {
  url: string;
  host: string;
  metadata: GrpcMetadataEntry[];
}

export interface GrpcUnaryRequest {
  protoPath: string;
  url: string;
  service: string;
  method: string;
  body: unknown;
  metadata: GrpcMetadataEntry[];
  reflection?: GrpcReflectionRequest | null;
}

export interface GrpcUnaryResponse {
  status: string;
  body: unknown;
  durationMs: number;
  responseMetadata: GrpcMetadataEntry[];
  trailers: GrpcMetadataEntry[];
}

export interface GrpcStreamingRequest {
  protoPath: string;
  url: string;
  service: string;
  method: string;
  messages: unknown[];
  metadata: GrpcMetadataEntry[];
  reflection?: GrpcReflectionRequest | null;
}

export interface GrpcStreamingResponse {
  status: string;
  messages: unknown[];
  messageCount: number;
  durationMs: number;
  responseMetadata: GrpcMetadataEntry[];
  trailers: GrpcMetadataEntry[];
}

export interface GitSemanticChange {
  changeType: "added" | "removed" | "changed";
  target: string;
  field: string;
  before: string | null;
  after: string | null;
}

export interface GitSnapshot {
  repositoryRoot: string;
  branch: string;
  status: string;
  diff: string;
  semanticBase: string;
  semanticChanges: GitSemanticChange[];
}

export interface ResponseHeader {
  name: string;
  value: string;
}

export interface HttpResponse {
  status: number;
  statusText: string;
  headers: ResponseHeader[];
  body: string;
  bodySize: number;
  durationMs: number;
  assertions: AssertionResult[];
  trace: TraceInfo;
  diagnostics: DiagnosticEntry[];
}

export type HttpProtocol = "auto" | "http1" | "http2";

export interface ExecutionError {
  kind: string;
  message: string;
  diagnostic: DiagnosticEntry | null;
}

export interface HttpProtocolSample {
  protocol: Exclude<HttpProtocol, "auto">;
  status: number | null;
  statusText: string | null;
  durationMs: number | null;
  bodySize: number | null;
  httpVersion: string | null;
  error: ExecutionError | null;
}

export interface HttpProtocolComparison {
  runs: HttpProtocolSample[];
  durationDeltaMs: number | null;
  bodySizeDeltaBytes: number | null;
}

export interface ProfilerConfig {
  totalRequests: number;
  concurrency: number;
}

export interface ProfilerStart {
  runId: string;
  totalRequests: number;
  concurrency: number;
}

export interface ProfilerStatusCount {
  status: number;
  count: number;
}

export interface ProfilerProvenance {
  latency: "measured" | "observed" | "inferred" | "unavailable";
  throughput: "measured" | "observed" | "inferred" | "unavailable";
  status: "measured" | "observed" | "inferred" | "unavailable";
}

export interface ProfilerSummary {
  runId: string;
  requested: number;
  completed: number;
  cancelledRequests: number;
  successful: number;
  httpErrors: number;
  transportErrors: number;
  elapsedMs: number;
  throughputRps: number;
  averageMs: number;
  minMs: number | null;
  maxMs: number | null;
  p50Ms: number | null;
  p95Ms: number | null;
  p99Ms: number | null;
  statuses: ProfilerStatusCount[];
  provenance: ProfilerProvenance;
}

export interface ProfilerError {
  kind: string;
  message: string;
  technical: string | null;
}

export type ProfilerEvent =
  | {
      eventType: "started";
      runId: string;
      requested: number;
      concurrency: number;
    }
  | {
      eventType: "progress";
      runId: string;
      completed: number;
      requested: number;
      successful: number;
      httpErrors: number;
      transportErrors: number;
      cancelledRequests: number;
      lastLatencyMs: number | null;
      lastError: ProfilerError | null;
    }
  | {
      eventType: "completed" | "cancelled";
      summary: ProfilerSummary;
    };

export interface AssertionResult {
  assertionType: string;
  passed: boolean;
  summary: string;
  expected: string;
  actual: string;
}

export interface TraceInfo {
  phases: TracePhase[];
  resolvedAddresses: string[];
  httpVersion: string | null;
  connectionReuse: TraceAvailability;
  tls: TraceAvailability;
}

export interface TracePhase {
  name: string;
  durationMs: number | null;
  provenance: "measured" | "observed" | "inferred" | "unavailable";
  detail: string;
}

export interface TraceAvailability {
  value: string | null;
  provenance: "measured" | "observed" | "inferred" | "unavailable";
  detail: string;
}

export interface DiagnosticEntry {
  layer: string;
  kind: string;
  summary: string;
  technical: string;
  provenance: "measured" | "observed" | "inferred" | "unavailable";
}

export interface HistorySummary {
  id: string;
  requestId: string;
  requestName: string;
  method: string;
  url: string;
  environmentName: string | null;
  createdAtMs: number;
  outcome: "success" | "http_error" | "failed";
  status: number | null;
  statusText: string | null;
  durationMs: number | null;
  bodySize: number | null;
  bodyTruncated: boolean;
  errorKind: string | null;
}

export interface HistoryEntry {
  summary: HistorySummary;
  request: RequestDefinition;
  response: HttpResponse | null;
  errorMessage: string | null;
}

export interface HistoryComparison {
  leftId: string;
  rightId: string;
  statusChanged: boolean;
  headersChanged: boolean;
  bodyChanged: boolean;
  durationDeltaMs: number | null;
  leftStatus: number | null;
  rightStatus: number | null;
  leftDurationMs: number | null;
  rightDurationMs: number | null;
  bodyNote: string;
}
