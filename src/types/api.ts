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

export type RequestBody =
  | {
      type: "json";
      value: unknown;
    }
  | {
      type: "text";
      value: string;
    };

export interface RequestDefinition {
  id: string;
  name: string;
  method: string;
  url: string;
  query: QueryParam[];
  headers: HeaderEntry[];
  body: RequestBody | null;
}

export interface CollectionFile {
  schemaVersion: number;
  name: string;
  requests: RequestDefinition[];
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

export interface GitSnapshot {
  repositoryRoot: string;
  branch: string;
  status: string;
  diff: string;
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
  trace: TraceInfo;
  diagnostics: DiagnosticEntry[];
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
