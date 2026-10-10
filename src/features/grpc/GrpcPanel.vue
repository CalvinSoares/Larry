<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { open as openDialog } from "@tauri-apps/plugin-dialog";

import { formatIpcError } from "../../services/errors";
import {
  cancelGrpcStream,
  executeGrpcStream,
  executeGrpcUnary,
  inspectGrpcProto,
  inspectGrpcReflection,
} from "../../services/ipc";
import CustomSelect from "../../components/ui/CustomSelect.vue";
import type {
  GrpcMetadataEntry,
  GrpcMethod,
  GrpcReflectionRequest,
  GrpcSchema,
  GrpcService,
  GrpcStreamingResponse,
  GrpcUnaryResponse,
} from "../../types/api";
import type { CustomSelectOption } from "../../types/ui";

const props = defineProps<{
  open: boolean;
}>();

const protoPath = ref("");
const url = ref("http://localhost:50051");
const reflectionUrl = ref("http://localhost:50051");
const reflectionHost = ref("");
const reflectionRequest = ref<GrpcReflectionRequest | null>(null);
const schema = ref<GrpcSchema | null>(null);
const selectedServiceName = ref("");
const selectedMethodName = ref("");
const bodyText = ref("{}\n");
const metadata = ref<GrpcMetadataEntry[]>([]);
const response = ref<GrpcUnaryResponse | GrpcStreamingResponse | null>(null);
const runId = ref<string | null>(null);
const error = ref("");
const isInspecting = ref(false);
const isReflecting = ref(false);
const isExecuting = ref(false);
const isCancelling = ref(false);

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

const methodIsStreaming = computed(() => {
  return Boolean(selectedMethod.value?.clientStreaming || selectedMethod.value?.serverStreaming);
});

const streamingModeLabel = computed(() => {
  if (!selectedMethod.value) {
    return "";
  }
  if (selectedMethod.value.clientStreaming && selectedMethod.value.serverStreaming) {
    return "Bidirectional streaming · envie uma lista JSON";
  }
  if (selectedMethod.value.clientStreaming) {
    return "Client streaming · envie uma lista JSON";
  }
  return "Server streaming · uma mensagem JSON, várias respostas";
});

const responseBody = computed(() => {
  if (!response.value) {
    return "";
  }

  return JSON.stringify("messages" in response.value ? response.value.messages : response.value.body, null, 2);
});

const responseMessageCount = computed(() => {
  if (!response.value) {
    return 0;
  }

  return "messages" in response.value ? response.value.messageCount : 1;
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
    reflectionRequest.value = null;
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

async function inspectReflection() {
  resetError();
  const normalizedUrl = reflectionUrl.value.trim();

  if (!normalizedUrl) {
    error.value = "Informe o endpoint gRPC que publica Reflection.";
    return;
  }

  isReflecting.value = true;

  try {
    const request: GrpcReflectionRequest = {
      url: normalizedUrl,
      host: reflectionHost.value.trim(),
      metadata: metadata.value,
    };
    const inspected = await inspectGrpcReflection(request);
    reflectionRequest.value = request;
    protoPath.value = "";
    reflectionUrl.value = normalizedUrl;
    url.value = normalizedUrl;
    schema.value = inspected;
    selectedServiceName.value = inspected.services[0]?.fullName ?? "";
    chooseFirstMethod(inspected.services[0] ?? null);
    response.value = null;
  } catch (value) {
    reflectionRequest.value = null;
    schema.value = null;
    selectedServiceName.value = "";
    selectedMethodName.value = "";
    error.value = formatIpcError(value);
  } finally {
    isReflecting.value = false;
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

  if (
    !schema.value ||
    !selectedService.value ||
    !selectedMethod.value ||
    (!protoPath.value.trim() && !reflectionRequest.value)
  ) {
    error.value = "Importe um .proto ou descubra o contrato por Reflection, depois selecione um serviço e método.";
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
  const streamRunId = methodIsStreaming.value
    ? `grpc-stream-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`
    : null;
  runId.value = streamRunId;

  try {
    const source = {
      protoPath: protoPath.value,
      url: url.value.trim(),
      service: selectedService.value.fullName,
      method: selectedMethod.value.name,
      metadata: metadata.value,
      reflection: reflectionRequest.value
        ? {
            url: url.value.trim(),
            host: reflectionHost.value.trim(),
            metadata: metadata.value,
          }
        : null,
    };

    if (methodIsStreaming.value) {
      const messages = selectedMethod.value.clientStreaming
        ? Array.isArray(body)
          ? body
          : null
        : [body];

      if (!messages) {
        error.value = "Métodos client ou bidirectional streaming exigem um array JSON de mensagens.";
        return;
      }

      response.value = await executeGrpcStream(
        {
          ...source,
          messages,
        },
        streamRunId!,
      );
    } else {
      response.value = await executeGrpcUnary({
        ...source,
        body,
      });
    }
  } catch (value) {
    error.value = formatIpcError(value);
  } finally {
    isExecuting.value = false;
    runId.value = null;
    isCancelling.value = false;
  }
}

async function cancel() {
  if (!runId.value) {
    return;
  }

  resetError();
  isCancelling.value = true;

  try {
    await cancelGrpcStream(runId.value);
  } catch (value) {
    isCancelling.value = false;
    error.value = formatIpcError(value);
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
          <span class="section-label">DESCRIPTOR REMOTO</span>
          <h3>Descobrir por Reflection</h3>
        </div>
        <button class="action-button" type="button" :disabled="isReflecting" @click="inspectReflection">
          {{ isReflecting ? "Consultando..." : "Consultar endpoint" }}
        </button>
      </div>
      <div class="field-grid reflection-grid">
        <label>
          Endpoint Reflection
          <input v-model="reflectionUrl" type="text" spellcheck="false" placeholder="http://localhost:50051" />
        </label>
        <label>
          Host opcional
          <input v-model="reflectionHost" type="text" spellcheck="false" placeholder="service.local" />
        </label>
      </div>
      <p class="path-value">O servidor precisa publicar o serviço padrão de gRPC Reflection. O contrato permanece em memória local.</p>
    </section>

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
          <span class="section-label">EXECUÇÃO</span>
          <h3>Endpoint e método</h3>
        </div>
        <div class="execution-actions">
          <button class="action-button primary" type="button" :disabled="isExecuting" @click="execute">
            {{ isExecuting ? "Executando..." : methodIsStreaming ? "Abrir streaming" : "Executar" }}
          </button>
          <button
            v-if="isExecuting && methodIsStreaming"
            class="text-button"
            type="button"
            :disabled="isCancelling"
            @click="cancel"
          >
            {{ isCancelling ? "Cancelando..." : "Cancelar" }}
          </button>
        </div>
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
        <span v-if="methodIsStreaming" class="warning-text">{{ streamingModeLabel }}</span>
      </div>
    </section>

    <section class="grpc-section body-section">
      <div class="section-heading compact">
        <div>
          <span class="section-label">PAYLOAD</span>
          <h3>{{ methodIsStreaming && selectedMethod?.clientStreaming ? "Mensagens JSON" : "Body JSON" }}</h3>
        </div>
        <button class="text-button" type="button" @click="prettifyBody">Formatar JSON</button>
      </div>
      <textarea
        v-model="bodyText"
        class="code-input"
        spellcheck="false"
        rows="8"
        :aria-label="methodIsStreaming && selectedMethod?.clientStreaming ? 'Mensagens JSON' : 'Body JSON'"
        :placeholder="methodIsStreaming && selectedMethod?.clientStreaming ? '[{}]' : '{}'"
      />
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
        <span v-if="response" class="response-duration">{{ response.durationMs }} ms · {{ responseMessageCount }} mensagem(ns)</span>
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

.execution-actions {
  display: flex;
  align-items: center;
  gap: 8px;
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

.action-button:focus-visible,
.text-button:focus-visible,
.icon-button:focus-visible,
input:focus-visible,
.code-input:focus-visible {
  outline: 2px solid var(--color-brand);
  outline-offset: 1px;
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

.reflection-grid {
  grid-template-columns: minmax(260px, 1.6fr) minmax(160px, 1fr);
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
  text-align: right;
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
  .endpoint-grid,
  .reflection-grid {
    grid-template-columns: 1fr;
  }
}
</style>
