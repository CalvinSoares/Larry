<script setup lang="ts">
import { computed, onMounted, ref } from "vue";

import RequestEditor from "./features/request-editor/RequestEditor.vue";
import ResponseViewer from "./features/response-viewer/ResponseViewer.vue";
import CollectionManager from "./features/collections/CollectionManager.vue";
import CollectionSidebar from "./features/collections/CollectionSidebar.vue";
import AppTitlebar from "./features/shell/AppTitlebar.vue";
import {
  executeRequest,
  getAppInfo,
  getSampleRequest,
} from "./services/ipc";
import { formatIpcError } from "./services/errors";
import type {
  AppInfo,
  CollectionFile,
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

function cloneRequest(request: RequestDefinition) {
  return JSON.parse(JSON.stringify(request)) as RequestDefinition;
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
  } catch (error) {
    errorMessage.value = formatIpcError(error);
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
    response.value = await executeRequest(requestDraft.value);
  } catch (error) {
    editorError.value = formatIpcError(error);
  } finally {
    isExecuting.value = false;
  }
}

onMounted(loadApplication);
</script>

<template>
  <main class="app-shell">
    <AppTitlebar :is-executing="isExecuting" />

    <section class="workspace-heading">
      <div>
        <p class="eyebrow">REQUEST WORKSPACE</p>
        <h1>Teste e investigue APIs localmente</h1>
        <p class="subtitle">Configure a chamada, execute e leia o resultado técnico.</p>
      </div>

      <span class="local-badge">Local first</span>
    </section>

    <section v-if="appInfo" class="diagnostic-strip">
      <span>{{ appInfo.appName }} {{ appInfo.version }}</span>
      <span>{{ appInfo.operatingSystem }} · {{ appInfo.architecture }}</span>
    </section>

    <p v-if="errorMessage" class="global-error">{{ errorMessage }}</p>

    <section v-if="requestDraft" class="workspace">
      <aside class="panel sidebar-panel">
        <CollectionSidebar
          :requests="collectionRequests"
          :active-request-id="activeRequestId"
          @select-request="selectCollectionRequest"
          @add-request="addCollectionRequest"
          @duplicate-request="duplicateCollectionRequest"
          @remove-request="removeCollectionRequest"
        />

        <CollectionManager
          :requests="collectionRequests"
          @loaded-collection="loadCollection"
        />
      </aside>

      <article class="panel request-panel">
        <div class="panel-heading">
          <div>
            <p class="eyebrow">ACTIVE REQUEST</p>
            <h2>Editor de request</h2>
          </div>
          <span class="panel-state">Editável</span>
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

.workspace-heading,
.diagnostic-strip,
.panel-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 20px;
}

.workspace-heading {
  flex: 0 0 auto;
  align-items: flex-end;
  margin: 0;
  padding: 18px 20px 12px;
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
  margin-bottom: 6px;
  color: var(--color-text);
  font-size: clamp(24px, 4vw, 32px);
  letter-spacing: -0.03em;
}

h2 {
  margin-bottom: 0;
  color: var(--color-text);
  font-size: 20px;
}

.subtitle {
  margin-bottom: 0;
  color: var(--color-text-muted);
}

.app-status,
.local-badge {
  border-radius: 999px;
  padding: 5px 10px;
  color: var(--color-brand);
  background: #163238;
  font-size: 12px;
  font-weight: 700;
}

.local-badge {
  color: var(--color-success);
  background: #163329;
}

.app-status-idle {
  color: var(--color-text-muted);
  background: var(--color-surface-2);
}

.panel-state {
  color: var(--color-text-muted);
  font-size: 12px;
}

.diagnostic-strip {
  flex: 0 0 auto;
  flex-wrap: wrap;
  margin: 0 20px 12px;
  padding: 8px 12px;
  border: 1px solid var(--color-border);
  border-radius: 5px;
  color: var(--color-text-muted);
  background: var(--color-surface-1);
  font-size: 12px;
}

.workspace {
  display: grid;
  min-width: 0;
  min-height: 0;
  flex: 1 1 auto;
  grid-template-columns: 260px minmax(420px, 1.15fr) minmax(320px, 0.85fr);
  overflow: hidden;
}

.panel,
.workspace > .response-viewer {
  min-width: 0;
  min-height: 0;
  border-radius: 0;
}

.workspace > .response-viewer {
  border-left: 1px solid var(--color-border);
}

.panel {
  overflow-y: auto;
  border: 0;
  border-right: 1px solid var(--color-border);
  background: var(--color-surface-1);
}

.sidebar-panel {
  background: #12151a;
}

.request-panel {
  overflow-y: auto;
  padding: 16px 20px 32px;
}

.sidebar-panel .collection-manager {
  margin: 0;
  padding: 16px;
}

.panel-heading {
  margin-bottom: 18px;
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

  .sidebar-panel {
    max-height: 320px;
  }

  .request-panel {
    border-right: 0;
  }
}

@media (max-width: 520px) {
  .workspace-heading,
  .panel-heading {
    align-items: flex-start;
    flex-direction: column;
    gap: 10px;
  }

  .workspace-heading {
    padding: 14px 12px 10px;
  }

  .diagnostic-strip {
    margin-right: 12px;
    margin-left: 12px;
  }

  .request-panel {
    padding: 14px 12px 24px;
  }
}
</style>
