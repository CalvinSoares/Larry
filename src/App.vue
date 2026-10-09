<script setup lang="ts">
import { computed, onMounted, ref } from "vue";

import RequestEditor from "./features/request-editor/RequestEditor.vue";
import ResponseViewer from "./features/response-viewer/ResponseViewer.vue";
import AppTitlebar from "./features/shell/AppTitlebar.vue";
import WorkspaceRail from "./features/shell/WorkspaceRail.vue";
import AppModal from "./components/ui/AppModal.vue";
import EnvironmentPanel from "./features/environments/EnvironmentPanel.vue";
import WelcomeModal from "./features/onboarding/WelcomeModal.vue";
import PostmanImportModal from "./features/onboarding/PostmanImportModal.vue";
import CurlImportModal from "./features/onboarding/CurlImportModal.vue";
import {
  executeRequest,
  getHistoryEntry,
  getAppInfo,
  getSampleRequest,
  listHistory,
} from "./services/ipc";
import { formatIpcError } from "./services/errors";
import type {
  AppInfo,
  CollectionFolder,
  CollectionFile,
  EnvironmentFile,
  HistorySummary,
  HttpResponse,
  RequestDefinition,
} from "./types/api";
import type { NewRequestDraft } from "./types/ui";

const appInfo = ref<AppInfo | null>(null);
const requestDraft = ref<RequestDefinition | null>(null);
const sampleRequest = ref<RequestDefinition | null>(null);
const collectionRequests = ref<RequestDefinition[]>([]);
const collectionFolders = ref<CollectionFolder[]>([]);
const collectionName = ref("Minha API");
const collectionPath = ref("");
const isCollectionDirty = ref(true);
const activeRequestId = ref("");
const response = ref<HttpResponse | null>(null);
const isExecuting = ref(false);
const errorMessage = ref("");
const editorError = ref("");
const resetToken = ref(0);
const isWelcomeOpen = ref(false);
const isPostmanImportOpen = ref(false);
const isCurlImportOpen = ref(false);
const isEnvironmentOpen = ref(false);
const activeEnvironment = ref<EnvironmentFile | null>(null);
const historyEntries = ref<HistorySummary[]>([]);
const isHistoryLoading = ref(false);

const collectionDirty = computed(() => isCollectionDirty.value || !collectionPath.value);

const ONBOARDING_STORAGE_KEY = "larry.onboarding.completed";

function cloneRequest(request: RequestDefinition) {
  return JSON.parse(JSON.stringify(request)) as RequestDefinition;
}

function cloneEnvironment(environment: EnvironmentFile) {
  return JSON.parse(JSON.stringify(environment)) as EnvironmentFile;
}

function cloneFolder(folder: CollectionFolder): CollectionFolder {
  return JSON.parse(JSON.stringify(folder)) as CollectionFolder;
}

function markCollectionDirty() {
  isCollectionDirty.value = true;
}

function flattenFolderRequests(folders: CollectionFolder[]): RequestDefinition[] {
  return folders.flatMap((folder) => [
    ...folder.requests.map(cloneRequest),
    ...flattenFolderRequests(folder.folders),
  ]);
}

const allCollectionRequests = computed(() => [
  ...collectionRequests.value.map(cloneRequest),
  ...flattenFolderRequests(collectionFolders.value),
]);

function findRequestList(requestId: string): RequestDefinition[] | null {
  if (collectionRequests.value.some((request) => request.id === requestId)) {
    return collectionRequests.value;
  }

  function search(folders: CollectionFolder[]): RequestDefinition[] | null {
    for (const folder of folders) {
      if (folder.requests.some((request) => request.id === requestId)) {
        return folder.requests;
      }

      const nested = search(folder.folders);
      if (nested) {
        return nested;
      }
    }

    return null;
  }

  return search(collectionFolders.value);
}

function findFolder(folderId: string, folders: CollectionFolder[]): CollectionFolder | null {
  for (const folder of folders) {
    if (folder.id === folderId) {
      return folder;
    }

    const nested = findFolder(folderId, folder.folders);
    if (nested) {
      return nested;
    }
  }

  return null;
}

function countFolderRequests(folder: CollectionFolder): number {
  return folder.requests.length + folder.folders.reduce(
    (total, child) => total + countFolderRequests(child),
    0,
  );
}

function findFolderLocation(
  folderId: string,
  folders: CollectionFolder[] = collectionFolders.value,
): { folders: CollectionFolder[]; index: number; folder: CollectionFolder } | null {
  for (let index = 0; index < folders.length; index += 1) {
    const folder = folders[index];
    if (!folder) {
      continue;
    }

    if (folder.id === folderId) {
      return { folders, index, folder };
    }

    const nested = findFolderLocation(folderId, folder.folders);
    if (nested) {
      return nested;
    }
  }

  return null;
}

function folderContainsRequest(folder: CollectionFolder, requestId: string): boolean {
  return folder.requests.some((request) => request.id === requestId)
    || folder.folders.some((child) => folderContainsRequest(child, requestId));
}

function hasCompletedOnboarding() {
  try {
    return window.localStorage.getItem(ONBOARDING_STORAGE_KEY) === "true";
  } catch {
    return false;
  }
}

function completeOnboarding() {
  try {
    window.localStorage.setItem(ONBOARDING_STORAGE_KEY, "true");
  } catch {
    // A sessão continua funcionando mesmo sem persistência do navegador.
  }

  isWelcomeOpen.value = false;
}

async function loadApplication() {
  try {
    const [loadedAppInfo, loadedRequest] = await Promise.all([
      getAppInfo(),
      getSampleRequest(),
    ]);

    appInfo.value = loadedAppInfo;
    sampleRequest.value = loadedRequest;
    requestDraft.value = cloneRequest(loadedRequest);
    collectionRequests.value = [cloneRequest(loadedRequest)];
    collectionFolders.value = [];
    collectionPath.value = "";
    isCollectionDirty.value = true;
    activeRequestId.value = loadedRequest.id;
    await refreshHistory();
  } catch (error) {
    errorMessage.value = formatIpcError(error);
  }
}

async function refreshHistory() {
  isHistoryLoading.value = true;

  try {
    historyEntries.value = await listHistory(20);
  } catch {
    historyEntries.value = [];
  } finally {
    isHistoryLoading.value = false;
  }
}

function updateRequest(request: RequestDefinition) {
  requestDraft.value = request;

  const requestList = findRequestList(request.id);
  const requestIndex = requestList?.findIndex((item) => item.id === request.id) ?? -1;
  if (requestList && requestIndex >= 0) {
    requestList[requestIndex] = cloneRequest(request);
  } else {
    collectionRequests.value.push(cloneRequest(request));
  }

  activeRequestId.value = request.id;
  editorError.value = "";
  markCollectionDirty();
}

function resetRequest() {
  if (!sampleRequest.value) {
    return;
  }

  const restoredRequest = cloneRequest(sampleRequest.value);
  const requestList = findRequestList(activeRequestId.value);
  const activeIndex = requestList?.findIndex((item) => item.id === activeRequestId.value) ?? -1;

  if (requestList && activeIndex >= 0) {
    restoredRequest.id = requestList[activeIndex].id;
    requestList[activeIndex] = cloneRequest(restoredRequest);
  } else {
    collectionRequests.value.push(cloneRequest(restoredRequest));
  }

  requestDraft.value = restoredRequest;
  activeRequestId.value = restoredRequest.id;
  markCollectionDirty();
  response.value = null;
  editorError.value = "";
  resetToken.value += 1;
}

function selectCollectionRequest(requestId: string) {
  const selectedRequest = allCollectionRequests.value.find((item) => item.id === requestId);
  if (!selectedRequest) {
    return;
  }

  requestDraft.value = cloneRequest(selectedRequest);
  activeRequestId.value = selectedRequest.id;
  response.value = null;
  editorError.value = "";
  resetToken.value += 1;
}

function loadCollection(collection: CollectionFile, path = "") {
  if (collection.requests.length === 0 && collection.folders.length === 0) {
    errorMessage.value = "A collection não possui requests executáveis para abrir.";
    return;
  }

  collectionName.value = collection.name;
  collectionRequests.value = collection.requests.map(cloneRequest);
  collectionFolders.value = collection.folders.map(cloneFolder);
  collectionPath.value = path;
  isCollectionDirty.value = !path;
  const firstRequest = allCollectionRequests.value[0];
  if (firstRequest) {
    selectCollectionRequest(firstRequest.id);
  }
}

function createRequestId() {
  return `request-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

function createNewRequest(name = "Nova request"): RequestDefinition {
  return {
    id: createRequestId(),
    name,
    method: "GET",
    url: "https://example.com",
    query: [],
    headers: [],
    cookies: [],
    body: null,
    auth: null,
    assertions: [],
  };
}

function createCollectionRequest(draft: NewRequestDraft, folderId: string | null = null) {
  if (draft.protocol !== "http") {
    return;
  }

  const request = createNewRequest(draft.name);
  request.method = draft.method;
  request.url = draft.url;

  const targetList = folderId
    ? findFolder(folderId, collectionFolders.value)?.requests ?? null
    : collectionRequests.value;
  if (!targetList) {
    return;
  }

  targetList.push(request);
  markCollectionDirty();
  selectCollectionRequest(request.id);
}

function renameCollectionRequest(requestId: string, requestedName: string) {
  const name = requestedName.trim();
  const requestList = findRequestList(requestId);
  const requestIndex = requestList?.findIndex((item) => item.id === requestId) ?? -1;

  if (!name || !requestList || requestIndex < 0) {
    return;
  }

  const renamedRequest = {
    ...requestList[requestIndex],
    name,
  };

  requestList[requestIndex] = cloneRequest(renamedRequest);
  markCollectionDirty();

  if (requestDraft.value?.id === requestId) {
    requestDraft.value = cloneRequest(renamedRequest);
    resetToken.value += 1;
  }
}

function startFreshRequest(name: string) {
  const request = createNewRequest(name);
  requestDraft.value = cloneRequest(request);
  collectionRequests.value = [cloneRequest(request)];
  collectionFolders.value = [];
  collectionPath.value = "";
  markCollectionDirty();
  activeRequestId.value = request.id;
  response.value = null;
  editorError.value = "";
  resetToken.value += 1;
}

function openPostmanImport() {
  isWelcomeOpen.value = false;
  isPostmanImportOpen.value = true;
}

function closePostmanImport() {
  isPostmanImportOpen.value = false;

  if (!hasCompletedOnboarding()) {
    isWelcomeOpen.value = true;
  }
}

function handlePostmanImported(collection: CollectionFile) {
  if (collection.requests.length === 0 && collection.folders.length === 0) {
    errorMessage.value = "A prévia Postman não possui requests executáveis para importar.";
    return;
  }

  loadCollection(collection);
  isPostmanImportOpen.value = false;
  completeOnboarding();
}

function openCurlImport() {
  isCurlImportOpen.value = true;
}

function handleCurlImported(request: RequestDefinition) {
  const importedRequest = cloneRequest(request);
  importedRequest.id = createRequestId();
  collectionRequests.value.push(importedRequest);
  markCollectionDirty();
  selectCollectionRequest(importedRequest.id);
  isCurlImportOpen.value = false;
}

function updateEnvironment(environment: EnvironmentFile | null) {
  activeEnvironment.value = environment ? cloneEnvironment(environment) : null;
}

function duplicateCollectionRequest(requestId: string) {
  const requestList = findRequestList(requestId);
  const requestIndex = requestList?.findIndex((item) => item.id === requestId) ?? -1;
  if (!requestList || requestIndex < 0) {
    return;
  }

  const copy = cloneRequest(requestList[requestIndex]);
  copy.id = createRequestId();
  copy.name = `${copy.name} (cópia)`;
  requestList.splice(requestIndex + 1, 0, copy);
  markCollectionDirty();
  selectCollectionRequest(copy.id);
}

function removeCollectionRequest(requestId: string) {
  const requestList = findRequestList(requestId);
  const requestIndex = requestList?.findIndex((item) => item.id === requestId) ?? -1;
  if (!requestList || requestIndex < 0 || allCollectionRequests.value.length <= 1) {
    return;
  }

  requestList.splice(requestIndex, 1);
  markCollectionDirty();

  if (activeRequestId.value === requestId) {
    const nextRequest = allCollectionRequests.value[Math.max(0, requestIndex - 1)] ?? allCollectionRequests.value[0];
    if (nextRequest) {
      selectCollectionRequest(nextRequest.id);
    }
  }
}

function createFolder(name: string, parentFolderId: string | null = null) {
  const trimmedName = name.trim();
  if (!trimmedName) {
    return;
  }

  const targetFolders = parentFolderId
    ? findFolder(parentFolderId, collectionFolders.value)?.folders ?? null
    : collectionFolders.value;
  if (!targetFolders) {
    return;
  }

  targetFolders.push({
    id: `folder-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    name: trimmedName,
    requests: [],
    folders: [],
  });
  markCollectionDirty();
}

function renameCollectionFolder(folderId: string, requestedName: string) {
  const name = requestedName.trim();
  const folder = findFolder(folderId, collectionFolders.value);
  if (!folder || !name) {
    return;
  }

  folder.name = name;
  markCollectionDirty();
}

function removeCollectionFolder(folderId: string) {
  const location = findFolderLocation(folderId);
  if (!location) {
    return;
  }

  const remainingRequests = allCollectionRequests.value.length - countFolderRequests(location.folder);
  if (remainingRequests < 1) {
    return;
  }

  const removedActiveRequest = folderContainsRequest(location.folder, activeRequestId.value);
  location.folders.splice(location.index, 1);
  markCollectionDirty();

  if (removedActiveRequest) {
    const nextRequest = allCollectionRequests.value[0];
    if (nextRequest) {
      selectCollectionRequest(nextRequest.id);
    }
  }
}

function moveCollectionRequest(requestId: string, folderId: string | null) {
  const sourceList = findRequestList(requestId);
  const sourceIndex = sourceList?.findIndex((request) => request.id === requestId) ?? -1;
  const targetList = folderId
    ? findFolder(folderId, collectionFolders.value)?.requests ?? null
    : collectionRequests.value;

  if (!sourceList || sourceIndex < 0 || !targetList || sourceList === targetList) {
    return;
  }

  const [request] = sourceList.splice(sourceIndex, 1);
  if (request) {
    targetList.push(request);
    markCollectionDirty();
  }
}

function handleCollectionSaved(path: string) {
  collectionPath.value = path;
  isCollectionDirty.value = false;
}

function handleCollectionNameChange(name: string) {
  collectionName.value = name;
  markCollectionDirty();
}

async function runRequest() {
  if (!requestDraft.value) {
    return;
  }

  isExecuting.value = true;
  editorError.value = "";
  response.value = null;

  try {
    response.value = await executeRequest(requestDraft.value, activeEnvironment.value);
  } catch (error) {
    editorError.value = formatIpcError(error);
  } finally {
    isExecuting.value = false;
    await refreshHistory();
  }
}

async function replayHistory(id: string) {
  try {
    const entry = await getHistoryEntry(id);
    updateRequest(cloneRequest(entry.request));
    response.value = entry.response;
    editorError.value = entry.errorMessage ?? "";
  } catch (error) {
    editorError.value = formatIpcError(error);
  }
}

onMounted(async () => {
  await loadApplication();

  if (requestDraft.value && !hasCompletedOnboarding()) {
    isWelcomeOpen.value = true;
  }
});
</script>

<template>
  <WelcomeModal
    :open="isWelcomeOpen"
    @close="isWelcomeOpen = false"
    @import-postman="openPostmanImport"
    @create-collection="completeOnboarding(); startFreshRequest('Nova collection')"
    @start-request="completeOnboarding(); startFreshRequest('Primeira request')"
    @explore="completeOnboarding"
  />

  <PostmanImportModal
    :open="isPostmanImportOpen"
    @close="closePostmanImport"
    @imported="handlePostmanImported"
  />

  <CurlImportModal
    :open="isCurlImportOpen"
    @close="isCurlImportOpen = false"
    @imported="handleCurlImported"
  />

  <AppModal
    :open="isEnvironmentOpen"
    :keep-mounted="true"
    size="wide"
    kicker="ENVIRONMENT"
    title="Variáveis do workspace"
    description="Variáveis públicas ficam no arquivo local. Secrets continuam separados no armazenamento seguro do sistema."
    primary-label="Fechar"
    secondary-label=""
    @close="isEnvironmentOpen = false"
    @confirm="isEnvironmentOpen = false"
  >
    <EnvironmentPanel
      :embedded="true"
      @environment-changed="updateEnvironment"
    />
  </AppModal>

  <main class="app-shell">
    <AppTitlebar
      :environment-name="activeEnvironment?.name ?? ''"
      :request-name="requestDraft?.name ?? 'Carregando request'"
      @open-environment="isEnvironmentOpen = true"
    />

    <p v-if="errorMessage" class="global-error">{{ errorMessage }}</p>

    <section v-if="requestDraft" class="workspace">
      <WorkspaceRail
        :requests="collectionRequests"
        :folders="collectionFolders"
        :collection-name="collectionName"
        :collection-dirty="collectionDirty"
        :active-request-id="activeRequestId"
        @select-request="selectCollectionRequest"
        @create-request="createCollectionRequest"
        @rename-request="renameCollectionRequest"
        @duplicate-request="duplicateCollectionRequest"
        @remove-request="removeCollectionRequest"
        @loaded-collection="loadCollection"
        @collection-saved="handleCollectionSaved"
        @collection-name-change="handleCollectionNameChange"
        @import-curl="openCurlImport"
        @create-folder="createFolder"
        @rename-folder="renameCollectionFolder"
        @remove-folder="removeCollectionFolder"
        @move-request="moveCollectionRequest"
      />

      <article class="panel request-panel">
        <RequestEditor
          :request="requestDraft"
          :is-executing="isExecuting"
          :reset-token="resetToken"
          @update:request="updateRequest"
          @submit="runRequest"
          @reset="resetRequest"
          @validation-error="editorError = $event"
        />
      </article>

      <ResponseViewer
        :request="requestDraft"
        :environment="activeEnvironment"
        :response="response"
        :error="editorError"
        :is-loading="isExecuting"
        :history-entries="historyEntries"
        :is-history-loading="isHistoryLoading"
        @replay-history="replayHistory"
      />
    </section>

    <p v-else-if="!errorMessage" class="loading-state">Carregando workspace...</p>

    <footer v-if="appInfo" class="app-statusbar">
      <span>Local first</span>
      <span>Core HTTP</span>
      <span>{{ appInfo.appName }} {{ appInfo.version }}</span>
      <span class="statusbar-platform">{{ appInfo.operatingSystem }} · {{ appInfo.architecture }}</span>
    </footer>
  </main>
</template>

<style>
:root {
  font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  color: #e7eaf0;
  background: #0f1115;
  color-scheme: dark;
  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;

  --color-bg: #0f1115;
  --color-surface-1: #15181e;
  --color-surface-2: #1b1f27;
  --color-surface-3: #222832;
  --color-border: #2c333e;
  --color-border-strong: #3b4552;
  --color-text: #e7eaf0;
  --color-text-muted: #9aa4b2;
  --color-text-subtle: #707b8b;
  --color-brand: #62d9dc;
  --color-brand-strong: #3fc0c5;
  --color-brand-warm: #f3ad58;
  --color-success: #4fc58a;
  --color-warning: #e3b76d;
  --color-danger: #ef8585;
  --color-info: #78a8f5;
  --color-code-bg: #0b0d11;
}

* {
  box-sizing: border-box;
}

html,
body,
#app {
  height: 100%;
  overflow: hidden;
}

body {
  margin: 0;
  min-width: 320px;
  background: var(--color-bg);
}

button,
input,
select,
textarea {
  font: inherit;
}

.app-shell {
  display: flex;
  width: 100%;
  min-width: 0;
  height: 100%;
  flex-direction: column;
  margin: 0;
  overflow: hidden;
}

.eyebrow {
  margin: 0 0 6px;
  color: var(--color-text-subtle);
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.08em;
}

h1,
h2,
h3,
p {
  margin-top: 0;
}

h2 {
  margin-bottom: 0;
  color: var(--color-text);
  font-size: 20px;
}

.workspace {
  display: grid;
  min-width: 0;
  min-height: 0;
  flex: 1 1 auto;
  grid-template-columns: 252px minmax(400px, 1.15fr) minmax(340px, 0.85fr);
  overflow: hidden;
}

.panel,
.workspace > .workspace-rail,
.workspace > .response-viewer {
  min-width: 0;
  min-height: 0;
  border-radius: 0;
}

.panel {
  overflow-y: auto;
  border: 0;
  background: var(--color-surface-1);
}

.workspace > .workspace-rail {
  border-right: 1px solid var(--color-border);
}

.workspace > .response-viewer {
  border-left: 1px solid var(--color-border);
}

.request-panel {
  overflow-y: auto;
  padding: 16px 20px 28px;
}

pre {
  max-height: 360px;
  margin: 10px 0 0;
  padding: 14px;
  overflow: auto;
  border-radius: 6px;
  background: var(--color-code-bg);
  color: var(--color-text);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 12px;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}

.global-error,
.loading-state {
  margin: 12px 20px;
  padding: 14px;
  border-radius: 6px;
  background: var(--color-surface-1);
}

.global-error {
  color: var(--color-danger);
}

.loading-state {
  color: var(--color-text-muted);
}

.app-statusbar {
  display: flex;
  flex: 0 0 28px;
  align-items: center;
  gap: 14px;
  min-width: 0;
  padding: 0 16px;
  border-top: 1px solid var(--color-border);
  color: var(--color-text-subtle);
  background: #101318;
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.06em;
  text-transform: uppercase;
}

.statusbar-platform {
  margin-left: auto;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

button,
input,
select,
textarea {
  color: var(--color-text);
}

button:focus-visible,
input:focus-visible,
select:focus-visible,
textarea:focus-visible,
summary:focus-visible {
  outline: 2px solid var(--color-brand);
  outline-offset: 2px;
}

@media (max-width: 1180px) {
  .workspace {
    grid-template-columns: 240px minmax(0, 1fr);
    grid-template-rows: minmax(0, 1fr) minmax(220px, 0.8fr);
  }

  .workspace > .response-viewer {
    grid-column: 2;
    grid-row: 2;
    border-left: 0;
    border-top: 1px solid var(--color-border);
  }

  .workspace > .workspace-rail {
    grid-row: 1 / -1;
  }
}

@media (max-width: 900px) {
  .workspace {
    grid-template-columns: 1fr;
    grid-template-rows: auto minmax(320px, 1fr) minmax(260px, 1fr);
    overflow-y: auto;
  }

  .workspace > .response-viewer {
    grid-column: auto;
    grid-row: 3;
    border-left: 0;
    border-top: 1px solid var(--color-border);
  }

  .workspace > .workspace-rail {
    grid-row: auto;
    max-height: 320px;
  }

  .request-panel {
    border-top: 1px solid var(--color-border);
  }
}

@media (max-width: 520px) {
  .request-panel {
    padding: 14px 12px 24px;
  }

  .app-statusbar {
    gap: 8px;
    padding: 0 10px;
  }

  .app-statusbar span:nth-child(3) {
    display: none;
  }
}
</style>
