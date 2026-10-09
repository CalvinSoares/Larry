<script setup lang="ts">
import { computed, ref, watch } from "vue";

import AppModal from "../../components/ui/AppModal.vue";
import { formatIpcError } from "../../services/errors";
import { parseCurlRequest } from "../../services/ipc";
import type { CurlImportPreview, RequestDefinition } from "../../types/api";

const props = defineProps<{
  open: boolean;
}>();

const emit = defineEmits<{
  (event: "close"): void;
  (event: "imported", request: RequestDefinition): void;
}>();

const command = ref("");
const preview = ref<CurlImportPreview | null>(null);
const isLoading = ref(false);
const error = ref("");

const primaryLabel = computed(() => {
  if (isLoading.value) {
    return "Analisando...";
  }

  return preview.value ? "Abrir request" : "Analisar cURL";
});

const bodyLabel = computed(() => {
  const body = preview.value?.request.body;
  if (!body) {
    return "Sem body";
  }

  if (body.type === "multipart") {
    return "Multipart";
  }

  if (body.type === "form-urlencoded") {
    return "Form URL Encoded";
  }

  return body.type === "json" ? "JSON" : "Texto";
});

function reset() {
  command.value = "";
  preview.value = null;
  isLoading.value = false;
  error.value = "";
}

async function analyzeCommand() {
  error.value = "";

  if (!command.value.trim()) {
    error.value = "Cole um comando cURL para analisar.";
    return;
  }

  isLoading.value = true;

  try {
    preview.value = await parseCurlRequest(command.value);
  } catch (value) {
    error.value = formatIpcError(value);
  } finally {
    isLoading.value = false;
  }
}

async function handlePrimary() {
  if (!preview.value) {
    await analyzeCommand();
    return;
  }

  emit("imported", preview.value.request);
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
    size="wide"
    variant="info"
    kicker="IMPORTAÇÃO LOCAL"
    title="Importar cURL"
    description="Analise o comando local e abra uma request editável antes de executá-la ou salvá-la."
    :primary-label="primaryLabel"
    :primary-disabled="isLoading"
    secondary-label="Cancelar"
    :close-on-backdrop="false"
    @close="emit('close')"
    @confirm="handlePrimary"
  >
    <div class="curl-import-content">
      <label class="command-label">
        <span>Comando cURL</span>
        <textarea
          v-model="command"
          rows="8"
          spellcheck="false"
          placeholder='curl --request POST "https://api.example.com/payments?environment=sandbox" \
  --header "Content-Type: application/json" \
  --data {"amount":100}'
          :disabled="Boolean(preview) || isLoading"
        />
      </label>

      <p v-if="error" class="import-error" aria-live="assertive">{{ error }}</p>

      <div v-if="preview" class="import-preview">
        <div class="preview-summary">
          <div>
            <span class="preview-label">Método</span>
            <strong class="method-value">{{ preview.request.method }}</strong>
          </div>
          <div class="preview-url-block">
            <span class="preview-label">URL</span>
            <strong>{{ preview.request.url }}</strong>
          </div>
          <div>
            <span class="preview-label">Body</span>
            <strong>{{ bodyLabel }}</strong>
          </div>
        </div>

        <div class="preview-counts">
          <span>{{ preview.request.headers.length }} headers</span>
          <span>{{ preview.request.query.length }} parâmetros</span>
          <span>{{ preview.request.cookies.length }} cookies</span>
        </div>

        <div v-if="preview.warnings.length" class="preview-warnings">
          <span class="preview-label">Revisar antes de executar</span>
          <ul>
            <li v-for="warning in preview.warnings" :key="warning">{{ warning }}</li>
          </ul>
        </div>

        <p v-else class="preview-ready">Comando convertido sem avisos. A request ainda não foi executada.</p>
      </div>

      <p v-else class="import-note">
        O comando é analisado localmente. O Larry não executa shell e não envia o texto para um backend.
      </p>
    </div>
  </AppModal>
</template>

<style scoped>
.curl-import-content {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.command-label {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.command-label span,
.preview-label {
  color: var(--color-text);
  font-size: 12px;
  font-weight: 600;
}

textarea {
  width: 100%;
  min-height: 154px;
  border: 1px solid var(--color-border-strong);
  border-radius: 6px;
  padding: 10px 12px;
  resize: vertical;
  color: var(--color-text);
  background: var(--color-code-bg);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 12px;
  line-height: 1.5;
}

textarea::placeholder {
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
  display: grid;
  grid-template-columns: 100px minmax(0, 1fr) 130px;
  gap: 16px;
}

.preview-summary > div {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 3px;
}

.preview-summary strong {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.preview-url-block strong {
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 12px;
}

.method-value {
  color: var(--color-brand);
}

.preview-label {
  color: var(--color-text-subtle);
  font-size: 10px;
  letter-spacing: 0.07em;
  text-transform: uppercase;
}

.preview-counts {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  color: var(--color-text-muted);
  font-size: 11px;
}

.preview-counts span {
  border: 1px solid var(--color-border);
  border-radius: 999px;
  padding: 4px 8px;
  background: var(--color-surface-2);
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

@media (max-width: 620px) {
  .preview-summary {
    grid-template-columns: 1fr 1fr;
  }

  .preview-url-block {
    grid-column: 1 / -1;
    grid-row: 1;
  }
}
</style>
