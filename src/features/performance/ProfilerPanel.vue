<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import { formatIpcError } from "../../services/errors";
import { cancelProfiler, startProfiler } from "../../services/ipc";
import type {
  EnvironmentFile,
  ProfilerEvent,
  ProfilerSummary,
  RequestDefinition,
} from "../../types/api";

const props = defineProps<{
  request: RequestDefinition;
  environment: EnvironmentFile | null;
  open: boolean;
}>();

type ProfilerProgress = Extract<ProfilerEvent, { eventType: "progress" }>;

const totalRequests = ref(10);
const concurrency = ref(2);
const runId = ref<string | null>(null);
const progress = ref<ProfilerProgress | null>(null);
const summary = ref<ProfilerSummary | null>(null);
const error = ref("");
const isStarting = ref(false);
const isCancelling = ref(false);
let unlisten: UnlistenFn | null = null;

const isRunning = computed(() => Boolean(runId.value));
const progressCount = computed(() => {
  if (!progress.value) {
    return 0;
  }

  return progress.value.completed + progress.value.cancelledRequests;
});
const progressPercent = computed(() => {
  const requested = progress.value?.requested ?? totalRequests.value;
  return requested > 0 ? Math.min(100, (progressCount.value / requested) * 100) : 0;
});
const currentError = computed(() => progress.value?.lastError?.message ?? "");

function resetError() {
  error.value = "";
}

function handleEvent(event: ProfilerEvent) {
  if (event.eventType === "started") {
    if (runId.value && event.runId !== runId.value) {
      return;
    }

    runId.value = event.runId;
    totalRequests.value = event.requested;
    concurrency.value = event.concurrency;
    return;
  }

  if (event.eventType === "progress") {
    if (runId.value !== event.runId) {
      return;
    }

    progress.value = event;
    return;
  }

  if (runId.value !== event.summary.runId) {
    return;
  }

  summary.value = event.summary;
  progress.value = null;
  runId.value = null;
  isCancelling.value = false;
}

async function startListening() {
  if (unlisten) {
    return;
  }

  unlisten = await listen<ProfilerEvent>("profiler_event", (event) => {
    handleEvent(event.payload);
  });
}

function validateInputs() {
  const requestCount = Math.trunc(Number(totalRequests.value));
  const concurrencyCount = Math.trunc(Number(concurrency.value));

  if (!Number.isInteger(requestCount) || requestCount < 1 || requestCount > 1_000) {
    error.value = "Informe entre 1 e 1000 requests.";
    return null;
  }

  if (!Number.isInteger(concurrencyCount) || concurrencyCount < 1 || concurrencyCount > 32) {
    error.value = "Informe uma concorrência entre 1 e 32.";
    return null;
  }

  if (concurrencyCount > requestCount) {
    error.value = "A concorrência não pode ser maior que a quantidade de requests.";
    return null;
  }

  totalRequests.value = requestCount;
  concurrency.value = concurrencyCount;
  return { totalRequests: requestCount, concurrency: concurrencyCount };
}

async function start() {
  resetError();
  summary.value = null;
  progress.value = null;

  const config = validateInputs();
  if (!config) {
    return;
  }

  isStarting.value = true;

  try {
    const started = await startProfiler(props.request, props.environment, config);
    runId.value = started.runId;
  } catch (value) {
    error.value = formatIpcError(value);
  } finally {
    isStarting.value = false;
  }
}

async function cancel() {
  if (!runId.value) {
    return;
  }

  resetError();
  isCancelling.value = true;

  try {
    await cancelProfiler(runId.value);
  } catch (value) {
    isCancelling.value = false;
    error.value = formatIpcError(value);
  }
}

function formatNumber(value: number) {
  return new Intl.NumberFormat("pt-BR", { maximumFractionDigits: 2 }).format(value);
}

function formatLatency(value: number | null) {
  return value === null ? "Indisponível" : `${formatNumber(value)} ms`;
}

function formatThroughput(value: number) {
  return `${formatNumber(value)} req/s`;
}

function statusLabel(status: number) {
  return String(status);
}

watch(
  () => props.open,
  (open) => {
    if (open) {
      void startListening();
    }
  },
);

onMounted(() => {
  if (props.open) {
    void startListening();
  }
});

onBeforeUnmount(() => {
  unlisten?.();
  unlisten = null;
});
</script>

<template>
  <div class="profiler-panel">
    <div class="profiler-target">
      <span class="section-label">DESTINO ATUAL</span>
      <strong>{{ props.request.method }} {{ props.request.url }}</strong>
      <small>
        {{ props.environment?.name || "Sem environment" }}. As requests serão enviadas de verdade para este endpoint.
      </small>
    </div>

    <div class="profiler-controls">
      <label>
        <span>Requests</span>
        <input v-model.number="totalRequests" type="number" min="1" max="1000" :disabled="isRunning || isStarting" />
      </label>
      <label>
        <span>Concorrência</span>
        <input v-model.number="concurrency" type="number" min="1" max="32" :disabled="isRunning || isStarting" />
      </label>
      <button class="action-button primary" type="button" :disabled="isStarting || isRunning" @click="start">
        {{ isStarting ? "Iniciando..." : "Executar profiler" }}
      </button>
      <button v-if="isRunning" class="action-button" type="button" :disabled="isCancelling" @click="cancel">
        {{ isCancelling ? "Cancelando..." : "Cancelar" }}
      </button>
    </div>

    <p v-if="error" class="panel-error" aria-live="assertive">{{ error }}</p>

    <section v-if="isRunning || progress" class="profiler-progress" aria-live="polite">
      <div class="progress-heading">
        <div>
          <span class="section-label">EXECUÇÃO EM ANDAMENTO</span>
          <strong>{{ progressCount }} de {{ progress?.requested ?? totalRequests }}</strong>
        </div>
        <span>{{ formatNumber(progressPercent) }}%</span>
      </div>
      <div class="progress-track" aria-hidden="true">
        <span :style="{ width: `${progressPercent}%` }"></span>
      </div>
      <p v-if="currentError" class="progress-error">Última falha: {{ currentError }}</p>
    </section>

    <section v-if="summary" class="profiler-summary">
      <div class="summary-heading">
        <div>
          <span class="section-label">RESULTADO</span>
          <strong>{{ summary.completed }} concluídas, {{ summary.cancelledRequests }} canceladas</strong>
        </div>
        <span>{{ formatNumber(summary.elapsedMs) }} ms total</span>
      </div>

      <div class="summary-grid">
        <div><span>Throughput</span><strong>{{ formatThroughput(summary.throughputRps) }}</strong></div>
        <div><span>Média</span><strong>{{ formatLatency(summary.averageMs) }}</strong></div>
        <div><span>P50</span><strong>{{ formatLatency(summary.p50Ms) }}</strong></div>
        <div><span>P95</span><strong>{{ formatLatency(summary.p95Ms) }}</strong></div>
        <div><span>P99</span><strong>{{ formatLatency(summary.p99Ms) }}</strong></div>
        <div><span>Sucesso / HTTP</span><strong>{{ summary.successful }} / {{ summary.httpErrors }}</strong></div>
        <div><span>Transporte</span><strong>{{ summary.transportErrors }}</strong></div>
      </div>

      <div v-if="summary.statuses.length" class="status-list">
        <span class="section-label">STATUS OBSERVADOS</span>
        <div class="status-badges">
          <span v-for="entry in summary.statuses" :key="entry.status" class="status-badge">
            {{ statusLabel(entry.status) }}: {{ entry.count }}
          </span>
        </div>
      </div>

      <p class="provenance-note">
        Latência: {{ summary.provenance.latency }}. Throughput: {{ summary.provenance.throughput }} a partir das amostras concluídas. Status: {{ summary.provenance.status }}.
      </p>
    </section>

    <p v-if="!isRunning && !summary" class="empty-state">
      Configure o número de requests e a concorrência para medir este endpoint localmente.
    </p>
  </div>
</template>

<style scoped>
.profiler-panel {
  display: flex;
  min-height: 360px;
  flex-direction: column;
  gap: 14px;
}

.profiler-target,
.profiler-progress,
.profiler-summary {
  border: 1px solid var(--color-border);
  border-radius: 6px;
  padding: 12px;
  background: var(--color-surface-2);
}

.profiler-target {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 5px;
}

.profiler-target strong {
  overflow: hidden;
  color: var(--color-text);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 12px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.profiler-target small {
  color: var(--color-text-muted);
  font-size: 11px;
  line-height: 1.4;
}

.section-label {
  color: var(--color-text-subtle);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.08em;
}

.profiler-controls {
  display: flex;
  align-items: flex-end;
  flex-wrap: wrap;
  gap: 8px;
}

.profiler-controls label {
  display: flex;
  min-width: 116px;
  flex-direction: column;
  gap: 5px;
  color: var(--color-text-muted);
  font-size: 11px;
}

.profiler-controls input {
  min-height: 34px;
  width: 100%;
  border: 1px solid var(--color-border-strong);
  border-radius: 5px;
  padding: 7px 9px;
  color: var(--color-text);
  background: var(--color-code-bg);
  font: inherit;
}

.action-button {
  min-height: 34px;
  border: 1px solid var(--color-border-strong);
  border-radius: 5px;
  padding: 7px 11px;
  color: var(--color-text);
  background: var(--color-surface-2);
  cursor: pointer;
  font-size: 12px;
  font-weight: 700;
  white-space: nowrap;
}

.action-button:hover:not(:disabled) {
  border-color: var(--color-brand);
  color: var(--color-brand);
}

.action-button.primary {
  border-color: var(--color-brand);
  color: #071113;
  background: var(--color-brand);
}

.action-button.primary:hover:not(:disabled) {
  color: #071113;
  background: var(--color-brand-strong);
}

.action-button:disabled,
.profiler-controls input:disabled {
  cursor: not-allowed;
  opacity: 0.55;
}

.panel-error,
.progress-error {
  margin: 0;
  color: var(--color-danger);
  font-size: 12px;
  line-height: 1.45;
}

.progress-heading,
.summary-heading {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 12px;
  color: var(--color-text-muted);
  font-size: 11px;
}

.progress-heading > div,
.summary-heading > div {
  display: flex;
  flex-direction: column;
  gap: 5px;
}

.progress-heading strong,
.summary-heading strong {
  color: var(--color-text);
  font-size: 13px;
}

.progress-track {
  height: 6px;
  margin-top: 12px;
  overflow: hidden;
  border-radius: 3px;
  background: var(--color-code-bg);
}

.progress-track span {
  display: block;
  height: 100%;
  background: var(--color-brand);
  transition: width 120ms ease-out;
}

.progress-error {
  margin-top: 10px;
}

.summary-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 1px;
  margin-top: 14px;
  border: 1px solid var(--color-border);
  background: var(--color-border);
}

.summary-grid > div {
  display: flex;
  min-height: 58px;
  flex-direction: column;
  justify-content: center;
  gap: 5px;
  padding: 8px 9px;
  background: var(--color-surface-1);
}

.summary-grid span {
  color: var(--color-text-subtle);
  font-size: 10px;
}

.summary-grid strong {
  color: var(--color-text);
  font-size: 12px;
}

.status-list {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 12px;
}

.status-badges {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
}

.status-badge {
  border: 1px solid var(--color-border-strong);
  border-radius: 4px;
  padding: 4px 7px;
  color: var(--color-text-muted);
  background: var(--color-surface-1);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 11px;
}

.provenance-note {
  margin: 12px 0 0;
  color: var(--color-text-subtle);
  font-size: 11px;
  line-height: 1.45;
}

.empty-state {
  margin: auto 0;
  color: var(--color-text-subtle);
  font-size: 12px;
  text-align: center;
}

@media (max-width: 640px) {
  .summary-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}
</style>
