<script setup lang="ts">
import { computed, ref, watch } from "vue";

import type { HttpResponse } from "../../types/api";

const props = defineProps<{
  response: HttpResponse | null;
  error: string;
  isLoading: boolean;
}>();

type ResponseTab = "body" | "headers" | "trace" | "diagnostics";

const activeTab = ref<ResponseTab>("body");

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
</script>

<template>
  <section class="response-viewer" aria-live="polite">
    <div class="viewer-heading">
      <div>
        <p class="viewer-kicker">INSPECTOR</p>
        <h2>Resposta</h2>
        <p>Evidência da última execução.</p>
      </div>

      <span v-if="props.isLoading" class="status status-loading">Executando</span>
      <span v-else-if="props.response" class="status status-success">Concluída</span>
      <span v-else-if="props.error" class="status status-error">Falhou</span>
    </div>

    <div v-if="props.isLoading" class="response-state">
      A requisição está sendo executada...
    </div>

    <div v-else-if="props.error" class="response-state response-error">
      <strong>Erro técnico:</strong>
      <p>{{ props.error }}</p>
    </div>

    <div v-else-if="props.response" class="response-content">
      <div class="response-summary">
        <strong>{{ props.response.status }} {{ props.response.statusText }}</strong>
        <span class="response-metric">
          <small>Tempo medido</small>
          {{ props.response.durationMs }} ms
        </span>
        <span class="response-metric">
          <small>Body medido</small>
          {{ props.response.bodySize }} bytes
        </span>
      </div>

      <nav class="response-tabs" aria-label="Seções da resposta" role="tablist">
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
      </nav>

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

    <div v-else class="response-state">
      Envie uma requisição para visualizar a resposta.
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

.viewer-heading,
.response-summary {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.viewer-heading h2 {
  margin: 0;
}

.viewer-kicker {
  margin: 0 0 5px;
  color: var(--color-text-subtle);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.08em;
}

.viewer-heading p {
  margin: 4px 0 0;
  color: var(--color-text-muted);
  font-size: 13px;
}

.status {
  border-radius: 999px;
  padding: 4px 9px;
  font-size: 12px;
  font-weight: 600;
}

.status-loading {
  color: var(--color-brand);
  background: #163238;
}

.status-success {
  color: var(--color-success);
  background: #163329;
}

.status-error {
  color: var(--color-danger);
  background: #3b2024;
}

.response-state {
  color: var(--color-text-muted);
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

.response-summary {
  justify-content: flex-start;
  flex-wrap: wrap;
  padding-bottom: 12px;
  border-bottom: 1px solid var(--color-border);
}

.response-summary span {
  color: var(--color-text-muted);
}

.response-metric {
  display: inline-flex;
  align-items: baseline;
  gap: 5px;
}

.response-metric small {
  color: var(--color-text-subtle);
  font-size: 10px;
  text-transform: uppercase;
}

.response-tabs {
  display: flex;
  gap: 4px;
  overflow-x: auto;
  border-bottom: 1px solid var(--color-border);
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

  .viewer-heading,
  .tab-panel-heading {
    align-items: flex-start;
    flex-direction: column;
  }

  .response-summary {
    align-items: flex-start;
    flex-direction: column;
    gap: 4px;
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
