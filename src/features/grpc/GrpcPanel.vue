<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { open as openDialog } from "@tauri-apps/plugin-dialog";

import { formatIpcError } from "../../services/errors";
import { executeGrpcUnary, inspectGrpcProto } from "../../services/ipc";
import CustomSelect from "../../components/ui/CustomSelect.vue";
import type {
  GrpcMetadataEntry,
  GrpcMethod,
  GrpcSchema,
  GrpcService,
  GrpcUnaryResponse,
} from "../../types/api";
import type { CustomSelectOption } from "../../types/ui";

const props = defineProps<{
  open: boolean;
}>();

const protoPath = ref("");
const url = ref("http://localhost:50051");
const schema = ref<GrpcSchema | null>(null);
const selectedServiceName = ref("");
const selectedMethodName = ref("");
const bodyText = ref("{}\n");
const metadata = ref<GrpcMetadataEntry[]>([]);
const response = ref<GrpcUnaryResponse | null>(null);
const error = ref("");
const isInspecting = ref(false);
const isExecuting = ref(false);

const selectedService = computed<GrpcService | null>(() => {
  return schema.value?.services.find((service) => service.fullName === selectedServiceName.value) ?? null;
});

const selectedMethod = computed<GrpcMethod | null>(() => {
  return selectedService.value?.methods.find((method) => method.name === selectedMethodName.value) ?? null;
});

const serviceOptions = computed<CustomSelectOption[]>(() => [
  { value: "", label: "Selecione um serviço", disabled: true },
  ...(schema.value?.services ?? []).map((service) => ({
    value: service.fullName,
    label: service.fullName,
  })),
]);

const methodOptions = computed<CustomSelectOption[]>(() => [
  { value: "", label: "Selecione um método", disabled: true },
  ...(selectedService.value?.methods ?? []).map((method) => ({
    value: method.name,
    label: `${method.name}${method.clientStreaming || method.serverStreaming ? " · streaming" : ""}`,
    tone: method.clientStreaming || method.serverStreaming ? "warning" as const : "default" as const,
  })),
]);

const responseBody = computed(() => {
  if (!response.value) {
    return "";
  }

  return JSON.stringify(response.value.body, null, 2);
});

function resetError() {
  error.value = "";
}

function chooseFirstMethod(service: GrpcService | null) {
  selectedMethodName.value = service?.methods[0]?.name ?? "";
}

async function importProto() {
  resetError();

  const selected = await openDialog({
    title: "Selecionar arquivo .proto",
    multiple: false,
    directory: false,
    filters: [{ name: "Protocol Buffers", extensions: ["proto"] }],
  });

  if (typeof selected !== "string") {
    return;
  }

  isInspecting.value = true;

  try {
    const inspected = await inspectGrpcProto(selected);
    protoPath.value = inspected.sourcePath;
    schema.value = inspected;
    selectedServiceName.value = inspected.services[0]?.fullName ?? "";
    chooseFirstMethod(inspected.services[0] ?? null);
    response.value = null;
  } catch (value) {
    schema.value = null;
    selectedServiceName.value = "";
    selectedMethodName.value = "";
    error.value = formatIpcError(value);
  } finally {
    isInspecting.value = false;
  }
}

function changeService() {
  chooseFirstMethod(selectedService.value);
}

function addMetadata() {
  metadata.value = [...metadata.value, { name: "", value: "", enabled: true }];
}

function removeMetadata(index: number) {
  metadata.value.splice(index, 1);
}

function prettifyBody() {
  resetError();

  try {
    bodyText.value = `${JSON.stringify(JSON.parse(bodyText.value), null, 2)}\n`;
  } catch {
    error.value = "O body precisa ser um JSON válido antes de formatar.";
  }
}

async function execute() {
  resetError();
  response.value = null;

  if (!schema.value || !selectedService.value || !selectedMethod.value) {
    error.value = "Importe um .proto e selecione um serviço e método.";
    return;
  }

  if (selectedMethod.value.clientStreaming || selectedMethod.value.serverStreaming) {
    error.value = "Este lote suporta somente métodos unary. Streaming será adicionado em uma fase posterior.";
    return;
  }

  let body: unknown;
  try {
    body = JSON.parse(bodyText.value);
  } catch {
    error.value = "O body precisa ser um JSON válido.";
    return;
  }

  if (!url.value.trim()) {
    error.value = "Informe o endpoint gRPC.";
    return;
  }

  isExecuting.value = true;

  try {
    response.value = await executeGrpcUnary({
      protoPath: protoPath.value,
      url: url.value.trim(),
      service: selectedService.value.fullName,
      method: selectedMethod.value.name,
      body,
      metadata: metadata.value,
    });
  } catch (value) {
    error.value = formatIpcError(value);
  } finally {
    isExecuting.value = false;
  }
}

watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) {
      resetError();
    }
  },
);
</script>

<template>
  <div class="grpc-panel">
    <section class="grpc-section">
      <div class="section-heading">
        <div>
          <span class="section-label">DESCRIPTOR LOCAL</span>
          <h3>Importar contrato</h3>
        </div>
        <button class="action-button" type="button" :disabled="isInspecting" @click="importProto">
          {{ isInspecting ? "Lendo..." : "Escolher .proto" }}
        </button>
      </div>
      <p class="path-value">{{ protoPath || "Nenhum arquivo selecionado" }}</p>
      <p v-if="schema" class="schema-summary">
        {{ schema.services.length }} serviço(s), {{ schema.services.reduce((count, service) => count + service.methods.length, 0) }} método(s) descoberto(s)
      </p>
    </section>

    <section class="grpc-section">
      <div class="section-heading">
        <div>
          <span class="section-label">EXECUÇÃO UNARY</span>
          <h3>Endpoint e método</h3>
        </div>
        <button class="action-button primary" type="button" :disabled="isExecuting" @click="execute">
          {{ isExecuting ? "Executando..." : "Executar" }}
        </button>
      </div>

      <div class="field-grid endpoint-grid">
        <label>
          Endpoint
          <input v-model="url" type="text" spellcheck="false" placeholder="http://localhost:50051" />
        </label>
        <label>
          Serviço
          <CustomSelect
            v-model="selectedServiceName"
            :options="serviceOptions"
            :disabled="!schema"
            label="Serviço gRPC"
            @change="changeService"
          />
        </label>
        <label>
          Método
          <CustomSelect
            v-model="selectedMethodName"
            :options="methodOptions"
            :disabled="!selectedService"
            label="Método gRPC"
          />
        </label>
      </div>

      <div class="method-note" v-if="selectedMethod">
        <span>{{ selectedMethod.inputType }} → {{ selectedMethod.outputType }}</span>
        <span v-if="selectedMethod.clientStreaming || selectedMethod.serverStreaming" class="warning-text">Streaming não disponível</span>
      </div>
    </section>

    <section class="grpc-section body-section">
      <div class="section-heading compact">
        <div>
          <span class="section-label">PAYLOAD</span>
          <h3>Body JSON</h3>
        </div>
        <button class="text-button" type="button" @click="prettifyBody">Formatar JSON</button>
      </div>
      <textarea v-model="bodyText" class="code-input" spellcheck="false" rows="8" aria-label="Body JSON" />
    </section>

    <section class="grpc-section">
      <div class="section-heading compact">
        <div>
          <span class="section-label">METADATA</span>
          <h3>Metadados ASCII</h3>
        </div>
        <button class="text-button" type="button" @click="addMetadata">Adicionar</button>
      </div>
      <div class="metadata-table">
        <div v-for="(entry, index) in metadata" :key="index" class="metadata-row">
          <input v-model="entry.name" type="text" placeholder="nome" aria-label="Nome do metadado" />
          <input v-model="entry.value" type="text" placeholder="valor" aria-label="Valor do metadado" />
          <button class="icon-button" type="button" aria-label="Remover metadado" @click="removeMetadata(index)">×</button>
        </div>
        <p v-if="metadata.length === 0" class="empty-inline">Nenhum metadado configurado.</p>
      </div>
    </section>

    <p v-if="error" class="panel-error" aria-live="assertive">{{ error }}</p>

    <section class="grpc-response">
      <div class="response-heading">
        <div>
          <span class="section-label">RESULTADO</span>
          <h3>{{ response ? `gRPC ${response.status}` : "Resposta" }}</h3>
        </div>
        <span v-if="response" class="response-duration">{{ response.durationMs }} ms</span>
      </div>
      <pre v-if="response" class="response-code">{{ responseBody }}</pre>
      <p v-else class="empty-state">A resposta da chamada aparecerá aqui.</p>
    </section>
  </div>
</template>

<style scoped>
.grpc-panel {
  display: flex;
  min-height: 480px;
  flex-direction: column;
  gap: 14px;
}

.grpc-section,
.grpc-response {
  border: 1px solid var(--color-border);
  border-radius: 6px;
  padding: 13px;
  background: var(--color-surface-2);
}

.section-heading,
.response-heading,
.method-note {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.section-heading.compact {
  align-items: baseline;
}

.section-heading h3,
.response-heading h3 {
  margin: 3px 0 0;
  color: var(--color-text);
  font-size: 14px;
}

.section-label {
  color: var(--color-text-subtle);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.08em;
}

.action-button,
.text-button,
.icon-button {
  border: 1px solid var(--color-border-strong);
  border-radius: 5px;
  color: var(--color-text);
  background: transparent;
  cursor: pointer;
  font-size: 12px;
  font-weight: 700;
}

.action-button {
  padding: 7px 10px;
}

.action-button:hover:not(:disabled),
.text-button:hover:not(:disabled),
.icon-button:hover:not(:disabled) {
  border-color: var(--color-brand);
  color: var(--color-brand);
}

.action-button.primary {
  border-color: var(--color-brand);
  color: #071113;
  background: var(--color-brand);
}

.action-button:disabled,
.text-button:disabled,
.icon-button:disabled,
input:disabled {
  cursor: not-allowed;
  opacity: 0.55;
}

.text-button {
  border-color: transparent;
  padding: 4px 6px;
  color: var(--color-brand);
  font-weight: 600;
}

.path-value,
.schema-summary,
.empty-inline,
.empty-state {
  margin: 8px 0 0;
  color: var(--color-text-muted);
  font-size: 11px;
}

.path-value,
.response-code,
.code-input {
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
}

.path-value {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.field-grid {
  display: grid;
  gap: 9px;
  margin-top: 12px;
}

.endpoint-grid {
  grid-template-columns: minmax(220px, 1.6fr) minmax(160px, 1fr) minmax(150px, 1fr);
}

label {
  display: flex;
  flex-direction: column;
  gap: 5px;
  color: var(--color-text-muted);
  font-size: 11px;
  font-weight: 700;
}

input,
.code-input {
  min-width: 0;
  border: 1px solid var(--color-border-strong);
  border-radius: 5px;
  padding: 8px 9px;
  color: var(--color-text);
  background: var(--color-code-bg);
  font-size: 12px;
}

.method-note {
  margin-top: 9px;
  color: var(--color-text-subtle);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 10px;
}

.warning-text {
  color: var(--color-warning);
  font-family: inherit;
}

.body-section {
  flex: 1 1 auto;
}

.code-input {
  display: block;
  width: 100%;
  min-height: 150px;
  margin-top: 10px;
  resize: vertical;
  line-height: 1.5;
}

.metadata-table {
  margin-top: 9px;
}

.metadata-row {
  display: grid;
  grid-template-columns: minmax(120px, 1fr) minmax(160px, 2fr) 30px;
  gap: 6px;
  margin-top: 6px;
}

.metadata-row:first-child {
  margin-top: 0;
}

.icon-button {
  width: 30px;
  padding: 0;
  color: var(--color-text-muted);
  font-size: 16px;
}

.panel-error {
  margin: 0;
  color: var(--color-danger);
  font-size: 12px;
  line-height: 1.4;
  white-space: pre-wrap;
}

.grpc-response {
  min-height: 150px;
  background: var(--color-code-bg);
}

.response-duration {
  color: var(--color-text-subtle);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 11px;
}

.response-code {
  max-height: 260px;
  overflow: auto;
  margin: 12px 0 0;
  color: var(--color-text);
  font-size: 12px;
  line-height: 1.5;
  white-space: pre-wrap;
}

.empty-state {
  padding: 24px 0;
  text-align: center;
}

@media (max-width: 720px) {
  .endpoint-grid {
    grid-template-columns: 1fr;
  }
}
</style>
