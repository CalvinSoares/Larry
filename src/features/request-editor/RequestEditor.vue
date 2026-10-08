<script setup lang="ts">
import { computed, ref, watch } from "vue";

import type {
  RequestBody,
  RequestDefinition,
} from "../../types/api";

const httpMethods = [
  "GET",
  "POST",
  "PUT",
  "PATCH",
  "DELETE",
] as const;

const props = defineProps<{
  request: RequestDefinition;
  isExecuting: boolean;
  resetToken: number;
}>();

const emit = defineEmits<{
  (event: "update:request", request: RequestDefinition): void;
  (event: "submit"): void;
  (event: "reset"): void;
  (event: "validation-error", message: string): void;
}>();

const localRequest = ref<RequestDefinition>(cloneRequest(props.request));
const bodyType = ref<"json" | "text">("json");
const bodyText = ref("");
const bodyError = ref("");
type ComposerTab = "params" | "headers" | "body";
const activeTab = ref<ComposerTab>("params");

const enabledParameterCount = computed(
  () => localRequest.value.query.filter((parameter) => parameter.enabled).length,
);
const enabledHeaderCount = computed(
  () => localRequest.value.headers.filter((header) => header.enabled).length,
);

function cloneRequest(request: RequestDefinition) {
  return JSON.parse(JSON.stringify(request)) as RequestDefinition;
}

function loadBodyEditor(body: RequestBody | null) {
  bodyError.value = "";

  if (!body) {
    bodyType.value = "json";
    bodyText.value = "";
    return;
  }

  bodyType.value = body.type;
  bodyText.value = body.type === "json"
    ? JSON.stringify(body.value, null, 2)
    : body.value;
}

function addHeader() {
  localRequest.value.headers.push({
    name: "",
    value: "",
    enabled: true,
  });
}

function removeHeader(index: number) {
  localRequest.value.headers.splice(index, 1);
}

function addQueryParam() {
  localRequest.value.query.push({
    name: "",
    value: "",
    enabled: true,
  });
}

function removeQueryParam(index: number) {
  localRequest.value.query.splice(index, 1);
}

function handleSubmit() {
  if (bodyError.value) {
    activeTab.value = "body";
    emit("validation-error", bodyError.value);
    return;
  }

  emit("submit");
}

watch(
  localRequest,
  (request) => {
    emit("update:request", cloneRequest(request));
  },
  { deep: true },
);

watch([bodyType, bodyText], () => {
  if (bodyType.value === "text") {
    localRequest.value.body = {
      type: "text",
      value: bodyText.value,
    };
    bodyError.value = "";
    return;
  }

  if (!bodyText.value.trim()) {
    localRequest.value.body = null;
    bodyError.value = "";
    return;
  }

  try {
    localRequest.value.body = {
      type: "json",
      value: JSON.parse(bodyText.value),
    };
    bodyError.value = "";
  } catch {
    bodyError.value = "JSON inválido. Corrija o conteúdo antes de executar.";
  }
});

watch(
  () => props.resetToken,
  () => {
    localRequest.value = cloneRequest(props.request);
    loadBodyEditor(localRequest.value.body);
  },
);

loadBodyEditor(localRequest.value.body);
</script>

<template>
  <form class="request-editor" @submit.prevent="handleSubmit">
    <div class="request-line">
      <label class="method-field">
        <span>Método</span>
        <span class="select-control">
          <select v-model="localRequest.method">
            <option v-for="method in httpMethods" :key="method" :value="method">
              {{ method }}
            </option>
          </select>
          <svg class="select-chevron" viewBox="0 0 16 16" aria-hidden="true">
            <path d="m4 6 4 4 4-4" />
          </svg>
        </span>
      </label>

      <label class="url-field">
        <span>URL</span>
        <input v-model="localRequest.url" type="url" required />
      </label>

      <button class="primary-action" type="submit" :disabled="props.isExecuting || Boolean(bodyError)">
        {{ props.isExecuting ? "Enviando..." : "Enviar" }}
      </button>
    </div>

    <nav class="composer-tabs" aria-label="Configurações da request" role="tablist">
      <button
        class="composer-tab"
        :class="{ 'composer-tab-active': activeTab === 'params' }"
        type="button"
        role="tab"
        :aria-selected="activeTab === 'params'"
        aria-controls="request-params-panel"
        @click="activeTab = 'params'"
      >
        Params <span>{{ enabledParameterCount }}</span>
      </button>
      <button
        class="composer-tab"
        :class="{ 'composer-tab-active': activeTab === 'headers' }"
        type="button"
        role="tab"
        :aria-selected="activeTab === 'headers'"
        aria-controls="request-headers-panel"
        @click="activeTab = 'headers'"
      >
        Headers <span>{{ enabledHeaderCount }}</span>
      </button>
      <button
        class="composer-tab"
        :class="{ 'composer-tab-active': activeTab === 'body' }"
        type="button"
        role="tab"
        :aria-selected="activeTab === 'body'"
        aria-controls="request-body-panel"
        @click="activeTab = 'body'"
      >
        Body <span>{{ localRequest.body ? 1 : 0 }}</span>
      </button>
    </nav>

    <section
      v-if="activeTab === 'params'"
      id="request-params-panel"
      class="editor-section"
      role="tabpanel"
    >
      <div class="section-heading">
        <div>
          <h3>Query parameters</h3>
          <p>Parâmetros habilitados serão adicionados à URL.</p>
        </div>

        <button type="button" @click="addQueryParam">Adicionar</button>
      </div>

      <div v-for="(param, index) in localRequest.query" :key="index" class="pair-row">
        <input v-model="param.enabled" type="checkbox" :aria-label="`Habilitar parâmetro ${index + 1}`" />
        <input v-model="param.name" type="text" placeholder="Nome" :aria-label="`Nome do parâmetro ${index + 1}`" />
        <input v-model="param.value" type="text" placeholder="Valor" :aria-label="`Valor do parâmetro ${index + 1}`" />
        <button type="button" @click="removeQueryParam(index)">Remover</button>
      </div>

      <p v-if="localRequest.query.length === 0" class="muted">Nenhum parâmetro configurado.</p>
    </section>

    <section
      v-else-if="activeTab === 'headers'"
      id="request-headers-panel"
      class="editor-section"
      role="tabpanel"
    >
      <div class="section-heading">
        <div>
          <h3>Headers</h3>
          <p>Somente headers habilitados serão enviados.</p>
        </div>

        <button type="button" @click="addHeader">Adicionar</button>
      </div>

      <div v-for="(header, index) in localRequest.headers" :key="index" class="pair-row">
        <input v-model="header.enabled" type="checkbox" :aria-label="`Habilitar header ${index + 1}`" />
        <input v-model="header.name" type="text" placeholder="Nome" :aria-label="`Nome do header ${index + 1}`" />
        <input v-model="header.value" type="text" placeholder="Valor" :aria-label="`Valor do header ${index + 1}`" />
        <button type="button" @click="removeHeader(index)">Remover</button>
      </div>

      <p v-if="localRequest.headers.length === 0" class="muted">Nenhum header configurado.</p>
    </section>

    <section
      v-else
      id="request-body-panel"
      class="editor-section"
      role="tabpanel"
    >
      <div class="section-heading">
        <div>
          <h3>Body</h3>
          <p>O JSON precisa ser válido antes do envio.</p>
        </div>

        <span class="select-control body-type-control">
          <select v-model="bodyType" aria-label="Tipo do body">
            <option value="json">JSON</option>
            <option value="text">Texto</option>
          </select>
          <svg class="select-chevron" viewBox="0 0 16 16" aria-hidden="true">
            <path d="m4 6 4 4 4-4" />
          </svg>
        </span>
      </div>

      <textarea v-model="bodyText" rows="12" placeholder="Digite o conteúdo da requisição"></textarea>

      <p v-if="bodyError" class="field-error">{{ bodyError }}</p>
      <p v-else-if="bodyType === 'json' && bodyText.trim()" class="field-success">JSON válido.</p>
    </section>

    <div class="secondary-actions">
      <button type="button" @click="emit('reset')">Restaurar exemplo</button>
    </div>
  </form>
</template>

<style scoped>
.request-editor {
  display: flex;
  flex-direction: column;
  gap: 18px;
}

.request-line {
  display: grid;
  grid-template-columns: 120px minmax(0, 1fr) auto;
  gap: 12px;
  align-items: end;
}

.method-field,
.url-field {
  min-width: 0;
}

.editor-section {
  display: flex;
  flex-direction: column;
  gap: 10px;
  min-width: 0;
  padding-top: 2px;
}

.composer-tabs {
  display: flex;
  min-width: 0;
  overflow-x: auto;
  border-bottom: 1px solid var(--color-border);
}

.composer-tab {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  min-height: 36px;
  border: 0;
  border-bottom: 2px solid transparent;
  border-radius: 0;
  padding: 0 10px;
  color: var(--color-text-muted);
  background: transparent;
  font-size: 12px;
  font-weight: 700;
}

.composer-tab:hover:not(:disabled) {
  border-color: transparent;
  color: var(--color-text);
  background: transparent;
}

.composer-tab-active {
  border-bottom-color: var(--color-brand);
  color: var(--color-text);
}

.composer-tab span {
  color: var(--color-text-subtle);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 11px;
  font-weight: 600;
}

.section-heading {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}

.section-heading h3 {
  margin: 0;
}

.section-heading p {
  margin: 4px 0 0;
  color: var(--color-text-muted);
  font-size: 13px;
}

label {
  display: flex;
  flex-direction: column;
  gap: 6px;
  text-align: left;
}

label span {
  font-size: 13px;
  font-weight: 600;
}

input,
select,
textarea {
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

.select-control {
  position: relative;
  display: block;
  min-width: 0;
}

.select-control select {
  appearance: none;
  padding-right: 30px;
}

.select-chevron {
  position: absolute;
  top: 50%;
  right: 9px;
  width: 14px;
  height: 14px;
  pointer-events: none;
  fill: none;
  stroke: var(--color-text-muted);
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 1.2;
  transform: translateY(-50%);
}

.body-type-control {
  width: 160px;
  flex: 0 0 160px;
}

input::placeholder,
textarea::placeholder {
  color: var(--color-text-subtle);
}

textarea {
  resize: vertical;
  min-height: 180px;
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 13px;
}

input[type="checkbox"] {
  width: 18px;
  height: 18px;
  min-height: 18px;
  margin: 0 6px 0 0;
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
  white-space: nowrap;
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

.primary-action {
  border-color: var(--color-brand-strong);
  color: #071315;
  background: var(--color-brand);
  font-weight: 700;
}

.primary-action:hover:not(:disabled) {
  border-color: var(--color-brand-strong);
  color: #071315;
  background: var(--color-brand);
}

.pair-row {
  display: grid;
  grid-template-columns: 24px minmax(0, 1fr) minmax(0, 2fr) auto;
  gap: 8px;
  align-items: center;
}

.muted {
  margin: 0;
  color: var(--color-text-muted);
}

.field-error {
  margin: 0;
  color: var(--color-danger);
}

.field-success {
  margin: 0;
  color: var(--color-success);
}

.secondary-actions {
  display: flex;
  justify-content: flex-end;
}

@media (max-width: 700px) {
  .request-line,
  .pair-row {
    grid-template-columns: 1fr;
  }

  input[type="checkbox"] {
    justify-self: start;
  }

  .primary-action {
    width: 100%;
  }
}
</style>
