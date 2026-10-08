<script setup lang="ts">
import { computed, ref, watch } from "vue";

import AppModal from "../../components/ui/AppModal.vue";
import { formatIpcError } from "../../services/errors";
import { previewPostmanCollection } from "../../services/ipc";
import type { CollectionFile, PostmanImportPreview } from "../../types/api";

const props = defineProps<{
  open: boolean;
}>();

const emit = defineEmits<{
  (event: "close"): void;
  (event: "imported", collection: CollectionFile): void;
}>();

const path = ref("");
const preview = ref<PostmanImportPreview | null>(null);
const isLoading = ref(false);
const error = ref("");

const primaryLabel = computed(() => {
  if (isLoading.value) {
    return "Lendo arquivo...";
  }

  return preview.value ? "Importar collection" : "Ler arquivo";
});

function reset() {
  path.value = "";
  preview.value = null;
  isLoading.value = false;
  error.value = "";
}

async function readPreview() {
  error.value = "";

  if (!path.value.trim()) {
    error.value = "Informe o caminho de um arquivo JSON exportado pelo Postman.";
    return;
  }

  isLoading.value = true;

  try {
    preview.value = await previewPostmanCollection(path.value);
  } catch (value) {
    error.value = formatIpcError(value);
  } finally {
    isLoading.value = false;
  }
}

async function handlePrimary() {
  if (!preview.value) {
    await readPreview();
    return;
  }

  if (preview.value.requestCount === 0) {
    error.value = "A prévia não possui requests executáveis para importar.";
    return;
  }

  emit("imported", preview.value.collection);
}

watch(
  () => props.open,
  (open) => {
    if (open) {
      reset();
    }
  },
);
</script>

<template>
  <AppModal
    :open="props.open"
    variant="info"
    kicker="IMPORTAÇÃO POSTMAN"
    title="Revisar collection"
    description="Leia o arquivo local e confira o que será convertido antes de colocar as requests no workspace."
    :primary-label="primaryLabel"
    :primary-disabled="isLoading"
    secondary-label="Cancelar"
    :close-on-backdrop="false"
    @close="emit('close')"
    @confirm="handlePrimary"
  >
    <div class="import-content">
      <label>
        <span>Caminho do arquivo JSON</span>
        <input
          v-model="path"
          type="text"
          placeholder="C:\\projetos\\minha-api\\postman-collection.json"
          :disabled="Boolean(preview) || isLoading"
        />
      </label>

      <p v-if="error" class="import-error" aria-live="assertive">{{ error }}</p>

      <div v-if="preview" class="import-preview">
        <div class="preview-summary">
          <div>
            <span class="preview-label">Collection</span>
            <strong>{{ preview.sourceName }}</strong>
          </div>
          <div>
            <span class="preview-label">Requests</span>
            <strong>{{ preview.requestCount }}</strong>
          </div>
        </div>

        <div v-if="preview.warnings.length" class="preview-warnings">
          <span class="preview-label">Revisar antes de salvar</span>
          <ul>
            <li v-for="warning in preview.warnings" :key="warning">{{ warning }}</li>
          </ul>
        </div>

        <p v-else class="preview-ready">Nenhum aviso de conversão foi encontrado.</p>
      </div>

      <p v-else class="import-note">
        O arquivo não será salvo automaticamente. A importação cria uma prévia em memória para você revisar.
      </p>
    </div>
  </AppModal>
</template>

<style scoped>
.import-content {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

label {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

label span,
.preview-label {
  color: var(--color-text);
  font-size: 12px;
  font-weight: 600;
}

input {
  width: 100%;
  min-height: 34px;
  border: 1px solid var(--color-border-strong);
  border-radius: 6px;
  padding: 7px 10px;
  color: var(--color-text);
  background: var(--color-surface-2);
}

input::placeholder {
  color: var(--color-text-subtle);
}

.import-note,
.preview-ready {
  margin: 0;
  color: var(--color-text-muted);
  font-size: 12px;
  line-height: 1.45;
}

.import-error {
  margin: 0;
  color: var(--color-danger);
  font-size: 12px;
  white-space: pre-wrap;
}

.import-preview {
  display: flex;
  flex-direction: column;
  gap: 12px;
  border-top: 1px solid var(--color-border);
  padding-top: 12px;
}

.preview-summary {
  display: flex;
  justify-content: space-between;
  gap: 16px;
}

.preview-summary > div {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 3px;
}

.preview-summary strong {
  overflow-wrap: anywhere;
}

.preview-label {
  color: var(--color-text-subtle);
  font-size: 10px;
  letter-spacing: 0.07em;
  text-transform: uppercase;
}

.preview-warnings {
  border: 1px solid #5a4827;
  border-radius: 6px;
  padding: 10px;
  background: #2a2418;
}

.preview-warnings .preview-label {
  color: var(--color-warning);
}

.preview-warnings ul {
  display: grid;
  gap: 6px;
  margin: 8px 0 0;
  padding-left: 18px;
  color: var(--color-text-muted);
  font-size: 12px;
  line-height: 1.4;
}

@media (max-width: 520px) {
  .preview-summary {
    align-items: flex-start;
    flex-direction: column;
    gap: 10px;
  }
}
</style>
