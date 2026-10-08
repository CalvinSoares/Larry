import { invoke } from "@tauri-apps/api/core";

import type {
  AppInfo,
  CollectionFile,
  GitSnapshot,
  HttpResponse,
  RequestDefinition,
} from "../types/api";

export function getAppInfo() {
  return invoke<AppInfo>("get_app_info");
}

export function getSampleRequest() {
  return invoke<RequestDefinition>("get_sample_request");
}

export function executeRequest(request: RequestDefinition) {
  return invoke<HttpResponse>("execute_request", { request });
}

export function saveCollection(path: string, collection: CollectionFile) {
  return invoke<void>("save_collection_file", { path, collection });
}

export function loadCollection(path: string) {
  return invoke<CollectionFile>("load_collection_file", { path });
}

export function getGitSnapshot(path: string) {
  return invoke<GitSnapshot>("get_git_snapshot", { path });
}
