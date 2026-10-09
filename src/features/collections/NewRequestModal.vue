<script setup lang="ts">
import { ref } from "vue";

import AppModal from "../../components/ui/AppModal.vue";
import CustomSelect from "../../components/ui/CustomSelect.vue";
import type { CustomSelectOption, NewRequestDraft, NewRequestProtocol } from "../../types/ui";

const props = defineProps<{
  open: boolean;
}>();

const emit = defineEmits<{
  (event: "close"): void;
  (event: "created", draft: NewRequestDraft): void;
  (event: "import-curl"): void;
}>();

const protocol = ref<NewRequestProtocol>("http");
const name = ref("Nova request");
const method = ref("GET");
const url = ref("https://example.com");
const error = ref("");

const protocolOptions: Array<{
  value: NewRequestProtocol;
  label: string;
  detail: string;
}> = [
  { value: "http", label: "HTTP", detail: "REST, JSON e headers" },
  { value: "websocket", label: "WebSocket", detail: "Canal persistente" },
  { value: "grpc", label: "gRPC", detail: "Serviços .proto" },
  { value: "sse", label: "SSE", detail: "Eventos do servidor" },
  { value: "curl", label: "From cURL", detail: "Importar comando" },
];

const methodOptions: CustomSelectOption[] = [
  { value: "GET", label: "GET", tone: "success" },
  { value: "POST", label: "POST", tone: "warning" },
  { value: "PUT", label: "PUT", tone: "info" },
  { value: "PATCH", label: "PATCH", tone: "info" },
  { value: "DELETE", label: "DELETE", tone: "warning" },
];

function selectProtocol(value: NewRequestProtocol) {
  protocol.value = value;
  error.value = "";

  if (value === "http") {
    url.value = "https://example.com";
  } else if (value === "websocket") {
    url.value = "wss://example.com/socket";
  } else if (value === "sse") {
    url.value = "https://example.com/events";
  } else if (value === "grpc") {
    url.value = "http://localhost:50051";
  }
}

function createRequest() {
  error.value = "";
  const requestName = name.value.trim();

  if (!requestName) {
    error.value = "Informe um nome para a request.";
    return;
  }

  if (protocol.value === "curl") {
    emit("import-curl");
    emit("close");
    return;
  }

  const requestUrl = url.value.trim();
  if (!requestUrl) {
    error.value = "Informe a URL da request.";
    return;
  }

  emit("created", {
    protocol: protocol.value,
    name: requestName,
    method: method.value,
    url: requestUrl,
  });
  emit("close");
}
</script>

<template>
  <AppModal
    :open="props.open"
    title="Nova Request"
    :close-on-backdrop="false"
    :keep-mounted="true"
    size="new-request"
    :show-kicker="false"
    :primary-label="protocol === 'curl' ? 'Importar cURL' : 'Criar Request'"
    secondary-label="Cancelar"
    :primary-disabled="false"
    @close="emit('close')"
    @confirm="createRequest"
  >
    <div class="new-request-form">
      <fieldset class="protocol-fieldset">
        <legend>Tipo de request</legend>
        <div class="protocol-grid" role="radiogroup" aria-label="Tipo de request">
          <button
            v-for="option in protocolOptions"
            :key="option.value"
            class="protocol-option"
            :class="{ 'protocol-option-active': protocol === option.value }"
            type="button"
            role="radio"
            :aria-checked="protocol === option.value"
            @click="selectProtocol(option.value)"
          >
            <span class="protocol-radio" aria-hidden="true">
              <span v-if="protocol === option.value" />
            </span>
            <span class="protocol-copy">
              <strong>{{ option.label }}</strong>
              <small>{{ option.detail }}</small>
            </span>
          </button>
        </div>
      </fieldset>

      <label class="field-label">
        <span>Nome</span>
        <input v-model="name" type="text" autocomplete="off" spellcheck="false" />
      </label>

      <div v-if="protocol !== 'curl'" class="request-destination">
        <label class="field-label method-field">
          <span>Método</span>
          <CustomSelect
            v-model="method"
            :options="methodOptions"
            label="Método HTTP"
            mono
          />
        </label>
        <label class="field-label url-field">
          <span>URL</span>
          <input v-model="url" type="text" autocomplete="off" spellcheck="false" />
        </label>
      </div>

      <p v-else class="curl-note">
        O próximo passo abrirá o importador para você colar e revisar o comando cURL.
      </p>

      <p v-if="error" class="form-error" role="alert">{{ error }}</p>
    </div>
  </AppModal>
</template>

<style scoped>
.new-request-form {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.protocol-fieldset {
  min-width: 0;
  margin: 0;
  border: 0;
  padding: 0;
}

.protocol-fieldset legend,
.field-label span {
  color: var(--color-text-subtle);
  font-size: 11px;
  font-weight: 700;
}

.protocol-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 6px;
  margin-top: 8px;
}

.protocol-option {
  display: flex;
  align-items: flex-start;
  min-width: 0;
  min-height: 52px;
  gap: 8px;
  border: 1px solid var(--color-border);
  border-radius: 6px;
  padding: 8px;
  color: var(--color-text-muted);
  background: var(--color-surface-2);
  cursor: pointer;
  font: inherit;
  text-align: left;
}

.protocol-option:last-child {
  grid-column: 1 / -1;
}

.protocol-option:hover {
  border-color: var(--color-border-strong);
  background: var(--color-surface-3);
}

.protocol-option:focus-visible {
  border-color: var(--color-brand);
  outline: 2px solid var(--color-brand);
  outline-offset: 2px;
}

.protocol-option-active {
  border-color: var(--color-brand);
  color: var(--color-text);
}

.protocol-radio {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: 0 0 auto;
  width: 14px;
  height: 14px;
  margin-top: 1px;
  border: 1px solid var(--color-border-strong);
  border-radius: 50%;
}

.protocol-option-active .protocol-radio {
  border-color: var(--color-brand);
}

.protocol-radio span {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--color-brand);
}

.protocol-copy {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 3px;
}

.protocol-copy strong {
  color: inherit;
  font-size: 12px;
  font-weight: 700;
}

.protocol-copy small {
  overflow: hidden;
  color: var(--color-text-subtle);
  font-size: 10px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.field-label {
  display: flex;
  flex-direction: column;
  gap: 5px;
}

.field-label input {
  width: 100%;
  height: 32px;
  border: 1px solid var(--color-border);
  border-radius: 5px;
  padding: 0 10px;
  color: var(--color-text);
  background: var(--color-surface-2);
  font: inherit;
  font-size: 12px;
}

.field-label input:focus-visible {
  border-color: var(--color-brand);
  outline: 2px solid var(--color-brand);
  outline-offset: 2px;
}

.request-destination {
  display: grid;
  grid-template-columns: 108px minmax(0, 1fr);
  gap: 8px;
}

.url-field input {
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
}

.curl-note,
.form-error {
  margin: 0;
  color: var(--color-text-muted);
  font-size: 12px;
  line-height: 1.45;
}

.form-error {
  color: var(--color-danger);
}

@media (max-width: 520px) {
  .protocol-grid,
  .request-destination {
    grid-template-columns: 1fr;
  }

  .protocol-option:last-child {
    grid-column: auto;
  }
}
</style>
