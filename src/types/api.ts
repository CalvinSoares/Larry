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
}
