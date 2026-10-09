<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from "vue";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";

import GitPanel from "../git/GitPanel.vue";
import {
  loadCollection,
  saveCollection,
} from "../../services/ipc";
import { formatIpcError } from "../../services/errors";
import type {
  CollectionFolder,
  CollectionFile,
  RequestDefinition,
} from "../../types/api";

const CURRENT_SCHEMA_VERSION = 2;
const AUTO_SAVE_STORAGE_KEY = "larry.collection.auto-save";
const AUTO_SAVE_DELAY_MS = 800;

const props = withDefaults(
  defineProps<{
    requests: RequestDefinition[];
    folders: CollectionFolder[];
    collectionName: string;
    dirty: boolean;
    embedded?: boolean;
  }>(),
  {
    embedded: false,
  },
);

const emit = defineEmits<{
  (event: "loaded-collection", collection: CollectionFile, path: string): void;
  (event: "collection-saved", path: string): void;
  (event: "collection-name-change", name: string): void;
}>();

const path = ref("");
const isSaving = ref(false);
const isLoading = ref(false);
const message = ref("");
const error = ref("");
const autoSaveEnabled = ref(readAutoSavePreference());
const changeRevision = ref(0);
let autoSaveTimer: ReturnType<typeof setTimeout> | null = null;

function readAutoSavePreference() {
  try {
    return window.localStorage.getItem(AUTO_SAVE_STORAGE_KEY) === "true";
  } catch {
    return false;
  }
}

function cloneRequest(request: RequestDefinition) {
  return JSON.parse(JSON.stringify(request)) as RequestDefinition;
}

function cloneFolders(folders: CollectionFolder[]) {
  return JSON.parse(JSON.stringify(folders)) as CollectionFolder[];
}

function countFolderRequests(folders: CollectionFolder[]): number {
  return folders.reduce(
    (total, folder) => total + folder.requests.length + countFolderRequests(folder.folders),
    0,
  );
}

function clearFeedback() {
  message.value = "";
  error.value = "";
}

function validatePath() {
  if (!path.value.trim()) {
    error.value = "Escolha um arquivo .yaml antes de continuar.";
    return false;
  }

  return true;
}

function buildCollection(): CollectionFile {
  return {
    schemaVersion: CURRENT_SCHEMA_VERSION,
    name: props.collectionName.trim() || "Minha collection",
    requests: props.requests.map(cloneRequest),
    folders: cloneFolders(props.folders),
  };
}

async function chooseOpenPath() {
  clearFeedback();

  try {
    const selected = await openDialog({
      title: "Abrir collection Larry",
      multiple: false,
      directory: false,
      filters: [{ name: "Collection Larry", extensions: ["yaml"] }],
    });

    if (typeof selected !== "string") {
      return;
    }

    path.value = selected;
    await handleLoad();
  } catch (dialogError) {
    error.value = formatIpcError(dialogError);
  }
}

async function chooseSavePath() {
  const selected = await saveDialog({
    title: "Salvar collection Larry",
    defaultPath: path.value || "collection.yaml",
    filters: [{ name: "Collection Larry", extensions: ["yaml"] }],
  });

  if (typeof selected === "string") {
    path.value = selected;
    return true;
  }

  return false;
}

async function persistCollection(automatic: boolean) {
  if (!validatePath()) {
    return false;
  }

  const collection = buildCollection();
  const revisionAtStart = changeRevision.value;
  let rescheduleAfterSave = false;

  isSaving.value = true;

  try {
    await saveCollection(path.value, collection);
    const changedDuringSave = changeRevision.value !== revisionAtStart;
    if (!changedDuringSave) {
      emit("collection-saved", path.value);
    } else {
      rescheduleAfterSave = automatic;
    }

    const requestCount = collection.requests.length + countFolderRequests(collection.folders);
    message.value = changedDuringSave
      ? "Collection salva antes da última alteração; o próximo salvamento continuará pendente."
      : automatic
      ? "Alterações salvas automaticamente."
      : `Collection salva com ${requestCount} request${requestCount === 1 ? "" : "s"}.`;
    return !changedDuringSave;
  } catch (saveError) {
    error.value = formatIpcError(saveError);
    return false;
  } finally {
    isSaving.value = false;
    if (rescheduleAfterSave) {
      scheduleAutoSave();
    }
  }
}

function clearAutoSaveTimer() {
  if (autoSaveTimer) {
    clearTimeout(autoSaveTimer);
    autoSaveTimer = null;
  }
}

function scheduleAutoSave() {
  clearAutoSaveTimer();

  if (!autoSaveEnabled.value || !props.dirty || !path.value.trim() || isSaving.value) {
    return;
  }

  autoSaveTimer = setTimeout(() => {
    autoSaveTimer = null;
    void persistCollection(true);
  }, AUTO_SAVE_DELAY_MS);
}

function toggleAutoSave(event: Event) {
  const input = event.target;
  if (!(input instanceof HTMLInputElement)) {
    return;
  }

  autoSaveEnabled.value = input.checked;
  try {
    window.localStorage.setItem(AUTO_SAVE_STORAGE_KEY, String(input.checked));
  } catch {
    // A preferência local indisponível não impede o salvamento manual.
  }

  if (input.checked) {
    scheduleAutoSave();
  } else {
    clearAutoSaveTimer();
  }
}

async function handleSave() {
  clearFeedback();
  clearAutoSaveTimer();

  try {
    if (!path.value.trim() && !(await chooseSavePath())) {
      return;
    }
  } catch (dialogError) {
    error.value = formatIpcError(dialogError);
    return;
  }

  await persistCollection(false);
}

async function handleLoad() {
  clearFeedback();
  clearAutoSaveTimer();

  if (!validatePath()) {
    return;
  }

  isLoading.value = true;

  try {
    const collection = await loadCollection(path.value);

    const requestCount = collection.requests.length + countFolderRequests(collection.folders);
    if (requestCount === 0) {
      error.value = "A collection não possui requests para carregar. Adicione uma request antes de abrir o arquivo.";
      return;
    }

    emit("loaded-collection", collection, path.value);
    message.value = `Collection carregada com ${requestCount} request${requestCount === 1 ? "" : "s"}.`;
  } catch (loadError) {
    error.value = formatIpcError(loadError);
  } finally {
    isLoading.value = false;
  }
}

function updateCollectionName(event: Event) {
  const input = event.target;
  if (input instanceof HTMLInputElement) {
    emit("collection-name-change", input.value);
  }
}

watch(
  () => [props.collectionName, props.requests, props.folders],
  () => {
    changeRevision.value += 1;
    scheduleAutoSave();
  },
  { deep: true },
);

onBeforeUnmount(clearAutoSaveTimer);

</script>

<template>
  <section class="collection-manager">
    <div v-if="!props.embedded" class="section-heading">
      <div>
        <p class="eyebrow">FILE-FIRST</p>
        <h3>Collection local</h3>
        <p>Salve um arquivo legível e versionável pelo Git.</p>
      </div>
      <span class="collection-state" :class="{ 'collection-state-dirty': props.dirty }">
        <span class="collection-state-dot" aria-hidden="true"></span>
        {{ props.dirty ? "Alterações não salvas" : "Salva localmente" }}
      </span>
    </div>

    <label>
      <span>Nome</span>
      <input :value="props.collectionName" type="text" @input="updateCollectionName" />
    </label>

    <label>
      <span>Caminho do arquivo</span>
      <div class="path-picker">
        <input
          :value="path"
          type="text"
          readonly
          placeholder="Escolha um arquivo .yaml"
          aria-label="Caminho da collection"
        />
        <button type="button" :disabled="isLoading || isSaving" @click="chooseOpenPath">
          Abrir arquivo
        </button>
      </div>
    </label>

    <div class="collection-actions">
      <button type="button" :disabled="isSaving" @click="handleSave">
        {{ isSaving ? "Salvando..." : "Salvar collection" }}
      </button>
    </div>

    <label class="auto-save-control">
      <input
        type="checkbox"
        :checked="autoSaveEnabled"
        @change="toggleAutoSave"
      />
      <span>Salvar alterações automaticamente</span>
    </label>
    <p class="auto-save-hint">Disponível somente depois que um arquivo local for escolhido.</p>

    <p v-if="message" class="feedback-success" aria-live="polite">{{ message }}</p>
    <p v-if="error" class="feedback-error" aria-live="assertive">{{ error }}</p>

    <GitPanel :path="path" />
  </section>
</template>

<style scoped>
.collection-manager {
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin-top: 0;
  border-top: 1px solid var(--color-border);
  padding-top: 16px;
  text-align: left;
}

.section-heading h3 {
  margin: 0;
  color: var(--color-text);
}

.section-heading {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}

.section-heading p:not(.eyebrow) {
  margin: 4px 0 0;
  color: var(--color-text-muted);
  font-size: 13px;
}

.eyebrow {
  margin: 0 0 4px;
  color: var(--color-text-subtle);
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.08em;
}

label {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

label span,
.field-label {
  color: var(--color-text);
  font-size: 13px;
  font-weight: 600;
}

input {
  box-sizing: border-box;
  width: 100%;
  min-height: 34px;
  border: 1px solid var(--color-border-strong);
  border-radius: 6px;
  padding: 6px 10px;
  font: inherit;
  background: var(--color-surface-2);
  color: var(--color-text);
}

input::placeholder {
  color: var(--color-text-subtle);
}

.collection-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.auto-save-control {
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: 7px;
  color: var(--color-text-muted);
  font-size: 12px;
  font-weight: 600;
}

.auto-save-control input {
  flex: 0 0 auto;
  width: 14px;
  min-height: 14px;
  margin: 0;
  accent-color: var(--color-brand);
}

.auto-save-hint {
  margin: -7px 0 0 21px;
  color: var(--color-text-subtle);
  font-size: 11px;
}

.path-picker {
  display: flex;
  min-width: 0;
  gap: 8px;
}

.path-picker input {
  min-width: 0;
  flex: 1 1 auto;
}

.path-picker button {
  flex: 0 0 auto;
  white-space: nowrap;
}

.collection-state {
  display: inline-flex;
  align-items: center;
  flex: 0 0 auto;
  gap: 6px;
  color: var(--color-success);
  font-size: 11px;
  font-weight: 700;
}

.collection-state-dirty {
  color: var(--color-warning);
}

.collection-state-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: currentColor;
}

button {
  min-height: 34px;
  border: 1px solid var(--color-border-strong);
  border-radius: 6px;
  padding: 6px 12px;
  font: inherit;
  cursor: pointer;
  background: var(--color-surface-2);
  color: var(--color-text);
}

button:hover:not(:disabled) {
  border-color: var(--color-border-strong);
  background: transparent;
  color: var(--color-text-muted);
}

button:disabled {
  cursor: wait;
  opacity: 0.65;
}

.feedback-success,
.feedback-error {
  margin: 0;
  font-size: 13px;
}

.feedback-success {
  color: var(--color-success);
}

.feedback-error {
  color: var(--color-danger);
  white-space: pre-wrap;
}

@media (max-width: 520px) {
  .section-heading {
    align-items: stretch;
    flex-direction: column;
  }

  .path-picker {
    align-items: stretch;
    flex-direction: column;
  }

  .collection-actions button {
    width: 100%;
  }

  .request-item {
    align-items: stretch;
    flex-direction: column;
  }

  .request-item-actions {
    justify-content: flex-end;
  }
}
</style>
