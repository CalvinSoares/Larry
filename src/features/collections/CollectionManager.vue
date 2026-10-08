<script setup lang="ts">
import { ref } from "vue";

import GitPanel from "../git/GitPanel.vue";
import {
  loadCollection,
  saveCollection,
} from "../../services/ipc";
import { formatIpcError } from "../../services/errors";
import type {
  CollectionFile,
  RequestDefinition,
} from "../../types/api";

const CURRENT_SCHEMA_VERSION = 1;

const props = withDefaults(
  defineProps<{
    requests: RequestDefinition[];
    embedded?: boolean;
  }>(),
  {
    embedded: false,
  },
);

const emit = defineEmits<{
  (event: "loaded-collection", collection: CollectionFile): void;
}>();

const path = ref("");
const collectionName = ref("Minha collection");
const isSaving = ref(false);
const isLoading = ref(false);
const message = ref("");
const error = ref("");

function cloneRequest(request: RequestDefinition) {
  return JSON.parse(JSON.stringify(request)) as RequestDefinition;
}

function clearFeedback() {
  message.value = "";
  error.value = "";
}

function validatePath() {
  if (!path.value.trim()) {
    error.value = "Informe o caminho de um arquivo .yaml.";
    return false;
  }

  return true;
}

async function handleSave() {
  clearFeedback();

  if (!validatePath()) {
    return;
  }

  const collection: CollectionFile = {
    schemaVersion: CURRENT_SCHEMA_VERSION,
    name: collectionName.value.trim() || "Minha collection",
    requests: props.requests.map(cloneRequest),
  };

  isSaving.value = true;

  try {
    await saveCollection(path.value, collection);
    message.value = `Collection salva com ${collection.requests.length} request${collection.requests.length === 1 ? "" : "s"}.`;
  } catch (saveError) {
    error.value = formatIpcError(saveError);
  } finally {
    isSaving.value = false;
  }
}

async function handleLoad() {
  clearFeedback();

  if (!validatePath()) {
    return;
  }

  isLoading.value = true;

  try {
    const collection = await loadCollection(path.value);

    if (collection.requests.length === 0) {
      error.value = "A collection não possui requests para carregar. Adicione uma request antes de abrir o arquivo.";
      return;
    }

    collectionName.value = collection.name;
    emit("loaded-collection", collection);
    message.value = `Collection carregada com ${collection.requests.length} request${collection.requests.length === 1 ? "" : "s"}.`;
  } catch (loadError) {
    error.value = formatIpcError(loadError);
  } finally {
    isLoading.value = false;
  }
}

</script>

<template>
  <section class="collection-manager">
    <div v-if="!props.embedded" class="section-heading">
      <div>
        <p class="eyebrow">FILE-FIRST</p>
        <h3>Collection local</h3>
        <p>Salve um arquivo legível e versionável pelo Git.</p>
      </div>
    </div>

    <label>
      <span>Nome</span>
      <input v-model="collectionName" type="text" />
    </label>

    <label>
      <span>Caminho do arquivo</span>
      <input
        v-model="path"
        type="text"
        placeholder="C:\\projetos\\minha-api\\collection.yaml"
      />
    </label>

    <div class="collection-actions">
      <button type="button" :disabled="isSaving" @click="handleSave">
        {{ isSaving ? "Salvando..." : "Salvar collection" }}
      </button>

      <button type="button" :disabled="isLoading" @click="handleLoad">
        {{ isLoading ? "Carregando..." : "Carregar collection" }}
      </button>
    </div>

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
