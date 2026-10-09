<script setup lang="ts">
import { computed, ref, watch } from "vue";

import CustomSelect from "../../components/ui/CustomSelect.vue";
import ProfilerPanel from "../performance/ProfilerPanel.vue";
import ProtocolLabPanel from "../protocol-lab/ProtocolLabPanel.vue";
import type {
  EnvironmentFile,
  HistorySummary,
  HttpResponse,
  RequestDefinition,
} from "../../types/api";
import type { CustomSelectOption } from "../../types/ui";

const props = defineProps<{
  request: RequestDefinition;
  environment: EnvironmentFile | null;
  response: HttpResponse | null;
  error: string;
  isLoading: boolean;
  historyEntries: HistorySummary[];
  isHistoryLoading: boolean;
}>();

const emit = defineEmits<{
  (event: "replay-history", id: string): void;
}>();

type ResponseTab =
  | "body"
  | "headers"
  | "trace"
  | "diagnostics"
  | "protocol-lab"
  | "profiler"
  | "assertions";

const activeTab = ref<ResponseTab>("body");
const selectedHistoryId = ref("");

watch(
  () => props.response,
  () => {
    activeTab.value = "body";
  },
);

const formattedBody = computed(() => {
  if (!props.response) {
    return "";
  }

  try {
    return JSON.stringify(JSON.parse(props.response.body), null, 2);
  } catch {
    return props.response.body;
  }
});

const responseSizeKb = computed(() => {
  if (!props.response) {
    return "---";
  }

  return (props.response.bodySize / 1024).toFixed(1);
});

const assertionResults = computed(() => props.response?.assertions ?? []);
const passedAssertionCount = computed(
  () => assertionResults.value.filter((assertion) => assertion.passed).length,
);

function formatHistoryTime(timestamp: number) {
  return new Intl.DateTimeFormat("pt-BR", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  }).format(new Date(timestamp));
}

function historyLabel(entry: HistorySummary) {
  const outcome = entry.status ? `${entry.status}` : "falhou";
  return `${formatHistoryTime(entry.createdAtMs)} · ${entry.method} · ${outcome}`;
}

const historyOptions = computed<CustomSelectOption[]>(() => [
  { value: "", label: "Agora" },
  ...props.historyEntries.map((entry) => ({
    value: entry.id,
    label: historyLabel(entry),
  })),
]);

function replaySelectedHistory() {
  if (selectedHistoryId.value) {
    emit("replay-history", selectedHistoryId.value);
  }
}
</script>

<template>
  <section class="response-viewer" aria-live="polite">
    <nav class="response-tabs" aria-label="Seções do inspector" role="tablist">
        <button
          class="response-tab"
          :class="{ 'response-tab-active': activeTab === 'body' }"
          type="button"
          role="tab"
          :aria-selected="activeTab === 'body'"
          aria-controls="response-body-panel"
          @click="activeTab = 'body'"
        >
          Body
        </button>
        <button
          class="response-tab"
          :class="{ 'response-tab-active': activeTab === 'headers' }"
          type="button"
          role="tab"
          :aria-selected="activeTab === 'headers'"
          aria-controls="response-headers-panel"
          @click="activeTab = 'headers'"
        >
          Headers
        </button>
        <button
          class="response-tab"
          :class="{ 'response-tab-active': activeTab === 'trace' }"
          type="button"
          role="tab"
          :aria-selected="activeTab === 'trace'"
          aria-controls="response-trace-panel"
          @click="activeTab = 'trace'"
        >
          Trace
        </button>
        <button
          class="response-tab"
          :class="{ 'response-tab-active': activeTab === 'diagnostics' }"
          type="button"
          role="tab"
          :aria-selected="activeTab === 'diagnostics'"
          aria-controls="response-diagnostics-panel"
          @click="activeTab = 'diagnostics'"
        >
          Diagnostics
        </button>
        <button
          class="response-tab response-tab-technical"
          :class="{ 'response-tab-active': activeTab === 'protocol-lab' }"
          type="button"
          role="tab"
          :aria-selected="activeTab === 'protocol-lab'"
          aria-controls="response-protocol-lab-panel"
          @click="activeTab = 'protocol-lab'"
        >
          Protocol Lab
        </button>
        <button
          class="response-tab response-tab-technical"
          :class="{ 'response-tab-active': activeTab === 'profiler' }"
          type="button"
          role="tab"
          :aria-selected="activeTab === 'profiler'"
          aria-controls="response-profiler-panel"
          @click="activeTab = 'profiler'"
        >
          Profiler
        </button>
        <button
          class="response-tab"
          :class="{ 'response-tab-active': activeTab === 'assertions' }"
          type="button"
          role="tab"
          :aria-selected="activeTab === 'assertions'"
          aria-controls="response-assertions-panel"
          @click="activeTab = 'assertions'"
        >
          Tests <span v-if="props.response">{{ passedAssertionCount }}/{{ assertionResults.length }}</span>
        </button>
      </nav>

    <div class="response-summary-row">
      <div class="response-status" aria-label="Resumo da execução">
        <span class="status-placeholder" :class="{ 'status-placeholder-active': props.response }">
          Status: {{ props.response ? `${props.response.status} ${props.response.statusText}` : "---" }}
        </span>
        <span class="status-placeholder">
          Tempo: {{ props.response ? props.response.durationMs : "---" }} ms
        </span>
        <span class="status-placeholder">
          Tamanho: {{ responseSizeKb }} KB
        </span>
      </div>

      <label class="history-selector">
        <span>Histórico</span>
        <CustomSelect
          v-model="selectedHistoryId"
          class="history-select-control"
          :options="historyOptions"
          :disabled="props.isHistoryLoading"
          label="Selecionar execução do histórico"
          @change="replaySelectedHistory"
        />
      </label>
    </div>

    <div v-if="activeTab === 'protocol-lab'" id="response-protocol-lab-panel" class="response-feature-panel" role="tabpanel">
      <ProtocolLabPanel
        :request="props.request"
        :response="props.response"
        :environment="props.environment"
      />
    </div>

    <div v-else-if="activeTab === 'profiler'" id="response-profiler-panel" class="response-feature-panel" role="tabpanel">
      <ProfilerPanel
        :request="props.request"
        :environment="props.environment"
        :open="activeTab === 'profiler'"
      />
    </div>

    <div v-else-if="props.isLoading" class="response-code-surface">
      <p class="response-empty">Executando requisição...</p>
    </div>

    <div v-else-if="props.error" class="response-code-surface response-error">
      <strong>Erro técnico:</strong>
      <p>{{ props.error }}</p>
    </div>

    <div v-else-if="props.response" class="response-content">

      <section v-if="activeTab === 'body'" id="response-body-panel" class="tab-panel" role="tabpanel">
        <div class="tab-panel-heading">
          <h3>Body</h3>
          <span class="provenance">Observado</span>
        </div>
        <pre v-if="formattedBody">{{ formattedBody }}</pre>
        <p v-else class="empty-tab-state">A resposta não contém body.</p>
      </section>

      <section v-else-if="activeTab === 'headers'" id="response-headers-panel" class="tab-panel" role="tabpanel">
        <div class="tab-panel-heading">
          <h3>Headers</h3>
          <span class="provenance">Observado</span>
        </div>
        <dl v-if="props.response.headers.length" class="response-headers">
          <template v-for="header in props.response.headers" :key="`${header.name}-${header.value}`">
            <dt>{{ header.name }}</dt>
            <dd>{{ header.value }}</dd>
          </template>
        </dl>
        <p v-else class="empty-tab-state">A resposta não retornou headers.</p>
      </section>

      <section v-else-if="activeTab === 'trace'" id="response-trace-panel" class="tab-panel evidence-panel" role="tabpanel">
        <span class="evidence-label">TRACE HTTP</span>
        <h3>Timeline observada da execução</h3>
        <p>Uma fase indisponível continua visível para não confundir ausência de medição com duração zero.</p>
        <ol class="trace-list">
          <li v-for="phase in props.response.trace.phases" :key="phase.name">
            <div>
              <strong>{{ phase.name.toUpperCase() }}</strong>
              <small>{{ phase.detail }}</small>
            </div>
            <span>
              {{ phase.durationMs === null ? "Indisponível" : `${phase.durationMs} ms` }}
              <small>{{ phase.provenance }}</small>
            </span>
          </li>
        </ol>
        <dl class="availability-list">
          <dt>IPs resolvidos</dt>
          <dd>{{ props.response.trace.resolvedAddresses.join(", ") || "Indisponível" }}</dd>
          <dt>HTTP</dt>
          <dd>{{ props.response.trace.httpVersion || "Indisponível" }}</dd>
          <dt>Reuso</dt>
          <dd>{{ props.response.trace.connectionReuse.value || props.response.trace.connectionReuse.provenance }}</dd>
          <dt>TLS</dt>
          <dd>{{ props.response.trace.tls.value || props.response.trace.tls.provenance }}</dd>
        </dl>
      </section>

      <section v-else-if="activeTab === 'assertions'" id="response-assertions-panel" class="tab-panel evidence-panel" role="tabpanel">
        <span class="evidence-label">VERIFICAÇÕES LOCAIS</span>
        <div class="tab-panel-heading">
          <h3>{{ passedAssertionCount }}/{{ assertionResults.length }} passaram</h3>
          <span class="provenance">Observado</span>
        </div>
        <p v-if="assertionResults.length === 0">Nenhuma assertion foi configurada para esta request.</p>
        <ol v-else class="assertion-results">
          <li v-for="(assertion, index) in assertionResults" :key="`${assertion.assertionType}-${index}`" :class="{ 'assertion-result-passed': assertion.passed }">
            <span class="assertion-result-mark" aria-hidden="true">{{ assertion.passed ? "OK" : "FALHOU" }}</span>
            <div>
              <strong>{{ assertion.summary }}</strong>
              <small>Esperado: {{ assertion.expected }} · Observado: {{ assertion.actual }}</small>
            </div>
          </li>
        </ol>
      </section>

      <section v-else id="response-diagnostics-panel" class="tab-panel evidence-panel" role="tabpanel">
        <span class="evidence-label">DIAGNÓSTICO POR CAMADA</span>
        <h3>{{ props.response.diagnostics[0]?.summary || "Nenhum diagnóstico produzido." }}</h3>
        <p v-if="props.response.diagnostics[0]">
          Camada observada: {{ props.response.diagnostics[0].layer }}.
          {{ props.response.diagnostics[0].technical }}
        </p>
        <ol v-if="props.response.diagnostics.length" class="layer-list">
          <li v-for="diagnostic in props.response.diagnostics" :key="diagnostic.kind">
            <span>{{ diagnostic.layer }}</span>
            <strong>{{ diagnostic.kind }} · {{ diagnostic.provenance }}</strong>
          </li>
        </ol>
        <p v-else>Nenhum diagnóstico foi retornado pelo core.</p>
      </section>
    </div>

    <div v-else class="response-code-surface">
      <p class="response-empty">Aguardando execução da requisição</p>
    </div>
  </section>
</template>

<style scoped>
.response-viewer {
  display: flex;
  flex-direction: column;
  gap: 16px;
  min-width: 0;
  min-height: 0;
  overflow-y: auto;
  padding: 16px 20px 32px;
  border: 0;
  border-radius: 0;
  background: var(--color-surface-1);
  text-align: left;
}

.response-error {
  color: var(--color-danger);
}

.response-error p {
  white-space: pre-wrap;
}

.response-content {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.response-feature-panel {
  min-width: 0;
  min-height: 0;
  overflow: auto;
}

.response-tabs {
  display: flex;
  gap: 4px;
  overflow-x: auto;
  border-bottom: 1px solid var(--color-border);
}

.response-status {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.response-summary-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  min-width: 0;
}

.history-selector {
  position: relative;
  display: inline-flex;
  flex: 0 0 auto;
  align-items: center;
  min-width: 0;
  color: var(--color-text-subtle);
  font-size: 11px;
}

.history-selector span {
  margin-right: 6px;
}

.history-select-control {
  width: 180px;
}

.status-placeholder {
  border: 1px solid var(--color-border);
  border-radius: 4px;
  padding: 4px 7px;
  color: var(--color-text-muted);
  background: var(--color-surface-2);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 11px;
}

.status-placeholder-active {
  color: var(--color-text);
}

.response-code-surface {
  display: flex;
  flex: 1 1 auto;
  min-height: 220px;
  flex-direction: column;
  justify-content: center;
  border: 1px solid var(--color-border);
  border-radius: 6px;
  background: var(--color-code-bg);
}

.response-empty {
  margin: 0;
  color: var(--color-text-subtle);
  text-align: center;
}

.response-tab {
  flex: 0 0 auto;
  min-height: 34px;
  border: 0;
  border-bottom: 2px solid transparent;
  padding: 0 8px;
  color: var(--color-text-muted);
  background: transparent;
  cursor: pointer;
  font-size: 12px;
  font-weight: 700;
}

.response-tab:hover {
  color: var(--color-text);
}

.response-tab-active {
  border-bottom-color: var(--color-brand);
  color: var(--color-text);
}

.response-tab span {
  color: var(--color-text-subtle);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 11px;
}

.tab-panel {
  min-width: 0;
}

.tab-panel-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin: 8px 0 10px;
}

.tab-panel-heading h3,
.evidence-panel h3 {
  margin: 0;
}

.provenance {
  color: var(--color-text-subtle);
  font-size: 11px;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.empty-tab-state {
  margin: 0;
  padding: 16px;
  border: 1px dashed var(--color-border);
  border-radius: 6px;
  color: var(--color-text-muted);
  background: var(--color-surface-2);
}

.evidence-panel {
  padding: 16px;
  border: 1px solid var(--color-border);
  border-radius: 6px;
  background: var(--color-surface-2);
}

.evidence-label {
  display: block;
  margin-bottom: 10px;
  color: var(--color-warning);
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.08em;
}

.evidence-panel p {
  margin: 8px 0 0;
  color: var(--color-text-muted);
  line-height: 1.5;
}

.availability-list,
.layer-list {
  margin: 16px 0 0;
}

.availability-list {
  display: grid;
  grid-template-columns: minmax(80px, 0.4fr) minmax(0, 1fr);
  gap: 6px 12px;
  padding-top: 12px;
  border-top: 1px solid var(--color-border);
}

.availability-list dt {
  color: var(--color-text);
  font-weight: 600;
}

.availability-list dd {
  margin: 0;
  color: var(--color-text-muted);
}

.trace-list {
  display: grid;
  gap: 8px;
  margin: 16px 0 0;
  padding: 0;
  list-style: none;
}

.trace-list li {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  border-bottom: 1px solid var(--color-border);
  padding-bottom: 8px;
}

.trace-list li > div,
.trace-list li > span {
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.trace-list li > span {
  align-items: flex-end;
  color: var(--color-text);
  white-space: nowrap;
}

.trace-list small {
  color: var(--color-text-subtle);
  font-size: 10px;
  font-weight: 400;
  white-space: normal;
}

.layer-list {
  display: grid;
  gap: 8px;
  padding-left: 20px;
  color: var(--color-text-muted);
}

.layer-list li {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  padding-left: 4px;
}

.layer-list strong {
  color: var(--color-text-subtle);
  font-size: 12px;
  font-weight: 500;
}

.assertion-results {
  display: grid;
  gap: 8px;
  margin: 16px 0 0;
  padding: 0;
  list-style: none;
}

.assertion-results li {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  border-bottom: 1px solid var(--color-border);
  padding-bottom: 8px;
}

.assertion-results li > div {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 3px;
}

.assertion-result-mark {
  flex: 0 0 auto;
  min-width: 48px;
  color: var(--color-danger);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 10px;
  font-weight: 700;
}

.assertion-result-passed .assertion-result-mark {
  color: var(--color-success);
}

.assertion-results small {
  color: var(--color-text-subtle);
  overflow-wrap: anywhere;
}

.response-content h3 {
  margin: 8px 0 0;
}

.response-headers {
  display: grid;
  grid-template-columns: minmax(120px, 0.4fr) minmax(0, 1fr);
  gap: 6px 12px;
  margin: 0;
}

.response-headers dt {
  font-weight: 600;
  color: var(--color-text);
  overflow-wrap: anywhere;
}

.response-headers dd {
  margin: 0;
  color: var(--color-text-muted);
  overflow-wrap: anywhere;
}

pre {
  max-height: 420px;
  margin: 0;
  padding: 14px;
  overflow: auto;
  border-radius: 6px;
  background: var(--color-code-bg);
  color: var(--color-text);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 13px;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}

@media (max-width: 520px) {
  .response-viewer {
    padding: 14px 12px 24px;
  }

  .tab-panel-heading {
    align-items: flex-start;
    flex-direction: column;
  }

  .response-summary-row {
    align-items: flex-start;
    flex-direction: column;
  }

  .history-selector {
    width: 100%;
    justify-content: space-between;
  }

  .history-select-control {
    flex: 1 1 auto;
    width: auto;
  }

  .response-headers,
  .availability-list {
    grid-template-columns: 1fr;
    gap: 3px;
  }

  .response-headers dd,
  .availability-list dd {
    margin-bottom: 8px;
  }

  .layer-list li {
    align-items: flex-start;
    flex-direction: column;
    gap: 2px;
  }

  .trace-list li {
    flex-direction: column;
  }

  .trace-list li > span {
    align-items: flex-start;
  }
}
</style>
