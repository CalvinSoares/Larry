import { invoke } from "@tauri-apps/api/core";

import type {
  AppInfo,
  CollectionFile,
  EnvironmentFile,
  GitSnapshot,
  HistoryComparison,
  HistoryEntry,
  HistorySummary,
  HttpResponse,
  PostmanImportPreview,
  RequestDefinition,
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
