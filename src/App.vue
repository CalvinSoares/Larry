<script setup lang="ts">
import { computed, onMounted, ref } from "vue";

import RequestEditor from "./features/request-editor/RequestEditor.vue";
import ResponseViewer from "./features/response-viewer/ResponseViewer.vue";
import AppTitlebar from "./features/shell/AppTitlebar.vue";
import WorkspaceRail from "./features/shell/WorkspaceRail.vue";
import WelcomeModal from "./features/onboarding/WelcomeModal.vue";
import PostmanImportModal from "./features/onboarding/PostmanImportModal.vue";
import {
  executeRequest,
  compareHistoryEntries,
  getHistoryEntry,
  getAppInfo,
  getSampleRequest,
  listHistory,
} from "./services/ipc";
import { formatIpcError } from "./services/errors";
import type {
  AppInfo,
  CollectionFile,
  EnvironmentFile,
  HistoryComparison,
  HistorySummary,
  HttpResponse,
  RequestDefinition,
} from "./types/api";

const appInfo = ref<AppInfo | null>(null);
const requestDraft = ref<RequestDefinition | null>(null);
const sampleRequest = ref<RequestDefinition | null>(null);
const collectionRequests = ref<RequestDefinition[]>([]);
const activeRequestId = ref("");
const response = ref<HttpResponse | null>(null);
const isExecuting = ref(false);
const errorMessage = ref("");
const editorError = ref("");
const resetToken = ref(0);
const isWelcomeOpen = ref(false);
const isPostmanImportOpen = ref(false);
const activeEnvironment = ref<EnvironmentFile | null>(null);
const historyEntries = ref<HistorySummary[]>([]);
const historyComparison = ref<HistoryComparison | null>(null);
const isHistoryLoading = ref(false);

const ONBOARDING_STORAGE_KEY = "larry.onboarding.completed";

function cloneRequest(request: RequestDefinition) {
  return JSON.parse(JSON.stringify(request)) as RequestDefinition;
}

function cloneEnvironment(environment: EnvironmentFile) {
  return JSON.parse(JSON.stringify(environment)) as EnvironmentFile;
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

const requestDraftJson = computed(() => {
  if (!requestDraft.value) {
    return "";
  }

  return JSON.stringify(requestDraft.value, null, 2);
});

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

  const requestIndex = collectionRequests.value.findIndex((item) => item.id === request.id);
  if (requestIndex >= 0) {
    collectionRequests.value[requestIndex] = cloneRequest(request);
  } else {
    collectionRequests.value.push(cloneRequest(request));
  }

  activeRequestId.value = request.id;
  editorError.value = "";
}

function resetRequest() {
  if (!sampleRequest.value) {
    return;
  }

  const restoredRequest = cloneRequest(sampleRequest.value);
  const activeIndex = collectionRequests.value.findIndex(
    (item) => item.id === activeRequestId.value,
  );

  if (activeIndex >= 0) {
    restoredRequest.id = collectionRequests.value[activeIndex].id;
    collectionRequests.value[activeIndex] = cloneRequest(restoredRequest);
  } else {
    collectionRequests.value.push(cloneRequest(restoredRequest));
  }

  requestDraft.value = restoredRequest;
  activeRequestId.value = restoredRequest.id;
  response.value = null;
  editorError.value = "";
  resetToken.value += 1;
}

function selectCollectionRequest(requestId: string) {
  const selectedRequest = collectionRequests.value.find((item) => item.id === requestId);
  if (!selectedRequest) {
    return;
  }

  requestDraft.value = cloneRequest(selectedRequest);
  activeRequestId.value = selectedRequest.id;
  response.value = null;
  editorError.value = "";
  resetToken.value += 1;
}

function loadCollection(collection: CollectionFile) {
  if (collection.requests.length === 0) {
    errorMessage.value = "A collection não possui requests executáveis para abrir.";
    return;
  }

  collectionRequests.value = collection.requests.map(cloneRequest);
  selectCollectionRequest(collectionRequests.value[0].id);
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
    body: null,
  };
}

function addCollectionRequest() {
  const request = createNewRequest();
  collectionRequests.value.push(request);
  selectCollectionRequest(request.id);
}

function renameCollectionRequest(requestId: string, requestedName: string) {
  const name = requestedName.trim();
  const requestIndex = collectionRequests.value.findIndex((item) => item.id === requestId);

  if (!name || requestIndex < 0) {
    return;
  }

  const renamedRequest = {
    ...collectionRequests.value[requestIndex],
    name,
  };

  collectionRequests.value[requestIndex] = cloneRequest(renamedRequest);

  if (requestDraft.value?.id === requestId) {
    requestDraft.value = cloneRequest(renamedRequest);
    resetToken.value += 1;
  }
}

function startFreshRequest(name: string) {
  const request = createNewRequest(name);
  requestDraft.value = cloneRequest(request);
  collectionRequests.value = [cloneRequest(request)];
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
  if (collection.requests.length === 0) {
    errorMessage.value = "A prévia Postman não possui requests executáveis para importar.";
    return;
  }

  loadCollection(collection);
  isPostmanImportOpen.value = false;
  completeOnboarding();
}

function updateEnvironment(environment: EnvironmentFile | null) {
  activeEnvironment.value = environment ? cloneEnvironment(environment) : null;
}

function duplicateCollectionRequest(requestId: string) {
  const requestIndex = collectionRequests.value.findIndex((item) => item.id === requestId);
  if (requestIndex < 0) {
    return;
  }

  const copy = cloneRequest(collectionRequests.value[requestIndex]);
  copy.id = createRequestId();
  copy.name = `${copy.name} (cópia)`;
  collectionRequests.value.splice(requestIndex + 1, 0, copy);
  selectCollectionRequest(copy.id);
}

function removeCollectionRequest(requestId: string) {
  const requestIndex = collectionRequests.value.findIndex((item) => item.id === requestId);
  if (requestIndex < 0 || collectionRequests.value.length <= 1) {
    return;
  }

  collectionRequests.value.splice(requestIndex, 1);

  if (activeRequestId.value === requestId) {
    const nextRequest = collectionRequests.value[Math.max(0, requestIndex - 1)];
    selectCollectionRequest(nextRequest.id);
  }
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
    historyComparison.value = null;
    editorError.value = entry.errorMessage ?? "";
  } catch (error) {
    editorError.value = formatIpcError(error);
  }
}

async function compareHistory(leftId: string, rightId: string) {
  try {
    historyComparison.value = await compareHistoryEntries(leftId, rightId);
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

  <main class="app-shell">
    <AppTitlebar
      :environment-name="activeEnvironment?.name ?? ''"
      :request-name="requestDraft?.name ?? 'Carregando request'"
    />

    <p v-if="errorMessage" class="global-error">{{ errorMessage }}</p>

    <section v-if="requestDraft" class="workspace">
      <WorkspaceRail
        :requests="collectionRequests"
        :active-request-id="activeRequestId"
        :history-entries="historyEntries"
        :history-comparison="historyComparison"
        :is-history-loading="isHistoryLoading"
        @select-request="selectCollectionRequest"
        @add-request="addCollectionRequest"
        @rename-request="renameCollectionRequest"
        @duplicate-request="duplicateCollectionRequest"
        @remove-request="removeCollectionRequest"
        @loaded-collection="loadCollection"
        @environment-changed="updateEnvironment"
        @refresh-history="refreshHistory"
        @replay-history="replayHistory"
        @compare-history="compareHistory"
      />

      <article class="panel request-panel">
        <div class="panel-heading">
          <div>
            <p class="eyebrow">COMPOSER</p>
            <h1>{{ requestDraft.name }}</h1>
          </div>
          <span class="panel-state">HTTP request</span>
        </div>

        <RequestEditor
          :request="requestDraft"
          :is-executing="isExecuting"
          :reset-token="resetToken"
          @update:request="updateRequest"
          @submit="runRequest"
          @reset="resetRequest"
          @validation-error="editorError = $event"
        />

        <details class="request-preview">
          <summary>Ver modelo serializado</summary>
          <pre>{{ requestDraftJson }}</pre>
        </details>
      </article>

      <ResponseViewer
        :response="response"
        :error="editorError"
        :is-loading="isExecuting"
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

.panel-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
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

h1 {
  margin-bottom: 0;
  color: var(--color-text);
  font-size: 18px;
  letter-spacing: -0.015em;
}

h2 {
  margin-bottom: 0;
  color: var(--color-text);
  font-size: 20px;
}

.panel-state {
  color: var(--color-text-muted);
  font-size: 12px;
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

.panel-heading {
  min-height: 42px;
  margin-bottom: 14px;
  border-bottom: 1px solid var(--color-border);
  padding-bottom: 12px;
}

.request-preview {
  margin-top: 20px;
  border-top: 1px solid var(--color-border);
  padding-top: 14px;
  color: var(--color-text-muted);
  font-size: 13px;
}

.request-preview summary {
  cursor: pointer;
  font-weight: 600;
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
  .panel-heading {
    align-items: flex-start;
    flex-direction: column;
    gap: 10px;
  }

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
