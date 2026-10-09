<script setup lang="ts">
import { computed, ref } from "vue";

import { formatIpcError } from "../../services/errors";
import { compareHttpProtocols } from "../../services/ipc";
import type {
  EnvironmentFile,
  HttpProtocolComparison,
  HttpResponse,
  RequestDefinition,
  TraceAvailability,
  TracePhase,
} from "../../types/api";

const props = defineProps<{
  request: RequestDefinition;
  response: HttpResponse | null;
  environment: EnvironmentFile | null;
}>();

const comparison = ref<HttpProtocolComparison | null>(null);
const comparisonError = ref("");
const isComparing = ref(false);

type Provenance = "measured" | "observed" | "inferred" | "unavailable";

interface LayerFact {
  label: string;
  value: string;
  provenance: Provenance;
}

interface LayerCard {
  key: string;
  number: string;
  title: string;
  description: string;
  state: Provenance;
  facts: LayerFact[];
  note: string;
}

function findPhase(response: HttpResponse | null, name: string): TracePhase | null {
  return response?.trace.phases.find((phase) => phase.name === name) ?? null;
}

function phaseValue(response: HttpResponse | null, name: string) {
  const phase = findPhase(response, name);
  return phase?.durationMs === null || phase?.durationMs === undefined
    ? "Indisponível"
    : `${phase.durationMs} ms`;
}

function phaseProvenance(response: HttpResponse | null, name: string): Provenance {
  return (findPhase(response, name)?.provenance as Provenance | undefined) ?? "unavailable";
}

function availabilityValue(availability: TraceAvailability | undefined) {
  if (!availability) {
    return "Indisponível";
  }

  return availability.value || "Indisponível";
}

function availabilityProvenance(availability: TraceAvailability | undefined): Provenance {
  return (availability?.provenance as Provenance | undefined) ?? "unavailable";
}

function provenanceLabel(provenance: Provenance) {
  const labels: Record<Provenance, string> = {
    measured: "Medido",
    observed: "Observado",
    inferred: "Inferido",
    unavailable: "Indisponível",
  };

  return labels[provenance];
}

function addressLabel(address: string) {
  return address.includes(":") ? `${address} · IPv6` : `${address} · IPv4`;
}

const layerCards = computed<LayerCard[]>(() => {
  const response = props.response;
  const trace = response?.trace;
  const hasResponse = Boolean(response);

  return [
    {
      key: "application",
      number: "01",
      title: "Application",
      description: "A camada onde a request HTTP e a resposta da API são interpretadas.",
      state: hasResponse ? "observed" : "unavailable",
      facts: [
        {
          label: "Método",
          value: hasResponse ? props.request.method : "Indisponível",
          provenance: hasResponse ? "observed" : "unavailable",
        },
        {
          label: "HTTP",
          value: trace?.httpVersion || "Indisponível",
          provenance: trace?.httpVersion ? "observed" : "unavailable",
        },
        {
          label: "Status",
          value: response ? `${response.status} ${response.statusText}` : "Indisponível",
          provenance: response ? "observed" : "unavailable",
        },
        {
          label: "TTFB",
          value: phaseValue(response, "ttfb"),
          provenance: phaseProvenance(response, "ttfb"),
        },
      ],
      note: "Status e versão HTTP vêm da resposta observada pelo executor local. TTFB é o intervalo até os headers ficarem disponíveis.",
    },
    {
      key: "transport",
      number: "02",
      title: "Transport",
      description: "A camada de entrega da mensagem e do canal que a transporta.",
      state: hasResponse ? "observed" : "unavailable",
      facts: [
        {
          label: "TCP",
          value: "Indisponível",
          provenance: "unavailable",
        },
        {
          label: "TLS",
          value: availabilityValue(trace?.tls),
          provenance: availabilityProvenance(trace?.tls),
        },
        {
          label: "Reuso",
          value: availabilityValue(trace?.connectionReuse),
          provenance: availabilityProvenance(trace?.connectionReuse),
        },
        {
          label: "Download",
          value: phaseValue(response, "download"),
          provenance: phaseProvenance(response, "download"),
        },
      ],
      note: "O adapter HTTP atual não expõe o intervalo TCP nem o handshake TLS detalhado. A ausência não representa duração zero.",
    },
    {
      key: "internet",
      number: "03",
      title: "Internet",
      description: "A camada de endereçamento IP usada para alcançar o host.",
      state: trace?.resolvedAddresses.length ? "observed" : "unavailable",
      facts: [
        {
          label: "DNS",
          value: phaseValue(response, "dns"),
          provenance: phaseProvenance(response, "dns"),
        },
        {
          label: "Endereços",
          value: trace?.resolvedAddresses.length
            ? trace.resolvedAddresses.map(addressLabel).join(", ")
            : "Indisponível",
          provenance: trace?.resolvedAddresses.length ? "observed" : "unavailable",
        },
        {
          label: "Rota",
          value: "Indisponível",
          provenance: "unavailable",
        },
      ],
      note: "Os endereços são resultado do DNS preflight local. O cliente não observa a rota completa pelos roteadores.",
    },
    {
      key: "network-access",
      number: "04",
      title: "Network Access",
      description: "O meio físico ou enlace que conecta o dispositivo à rede.",
      state: "unavailable",
      facts: [
        {
          label: "Wi-Fi / Ethernet",
          value: "Indisponível",
          provenance: "unavailable",
        },
        {
          label: "Bytes no fio",
          value: "Indisponível",
          provenance: "unavailable",
        },
        {
          label: "Captura",
          value: "Não habilitada",
          provenance: "unavailable",
        },
      ],
      note: "Ethernet, Wi-Fi e bytes físicos exigem integração de captura opt-in. O trace próprio não precisa de privilégio elevado.",
    },
  ];
});

function protocolLabel(protocol: "http1" | "http2") {
  return protocol === "http1" ? "HTTP/1.1" : "HTTP/2";
}

function formatDelta(value: number | null, unit: string) {
  if (value === null) {
    return "Indisponível";
  }

  const direction = value === 0 ? "sem diferença" : value > 0 ? "HTTP/2 maior" : "HTTP/2 menor";
  return `${value > 0 ? "+" : ""}${value} ${unit} · ${direction}`;
}

async function compareProtocols() {
  comparisonError.value = "";
  comparison.value = null;
  isComparing.value = true;

  try {
    comparison.value = await compareHttpProtocols(props.request, props.environment);
  } catch (value) {
    comparisonError.value = formatIpcError(value);
  } finally {
    isComparing.value = false;
  }
}
</script>

<template>
  <div class="protocol-lab-panel">
    <header class="lab-intro">
      <span class="section-label">EVIDÊNCIA LOCAL</span>
      <h3>Mapa da execução</h3>
      <p>Leia a request real por camadas e veja onde há uma medição disponível ou uma lacuna de observabilidade.</p>
    </header>

    <div class="lab-target">
      <span class="method-label">{{ props.request.method }}</span>
      <code>{{ props.request.url }}</code>
      <span class="target-state">
        {{ props.environment?.name || "Sem environment" }} · {{ props.response ? "Resposta disponível" : "Aguardando request" }}
      </span>
    </div>

    <section class="comparison-launch">
      <div>
        <span class="section-label">COMPARAÇÃO REAL</span>
        <p>Execute esta mesma request uma vez em HTTP/1.1 e outra em HTTP/2. O resultado não entra no histórico.</p>
      </div>
      <button class="compare-button" type="button" :disabled="isComparing" @click="compareProtocols">
        {{ isComparing ? "Comparando..." : "Comparar protocolos" }}
      </button>
    </section>

    <p v-if="comparisonError" class="comparison-error" aria-live="assertive">{{ comparisonError }}</p>

    <section v-if="comparison" class="comparison-result">
      <div class="comparison-heading">
        <div>
          <span class="section-label">RESULTADO DA COMPARAÇÃO</span>
          <h3>Mesma request, dois modos de transporte</h3>
        </div>
        <span>HTTP/2 - HTTP/1.1</span>
      </div>

      <div class="comparison-grid">
        <article v-for="run in comparison.runs" :key="run.protocol" class="comparison-card">
          <div class="comparison-card-heading">
            <h4>{{ protocolLabel(run.protocol) }}</h4>
            <span>{{ run.httpVersion || "Versão indisponível" }}</span>
          </div>
          <dl>
            <div><dt>Status</dt><dd>{{ run.status ? `${run.status} ${run.statusText}` : "Indisponível" }}</dd></div>
            <div><dt>Tempo</dt><dd>{{ run.durationMs === null ? "Indisponível" : `${run.durationMs} ms` }}</dd></div>
            <div><dt>Body</dt><dd>{{ run.bodySize === null ? "Indisponível" : `${run.bodySize} bytes` }}</dd></div>
          </dl>
          <div v-if="run.error" class="comparison-run-error">
            <strong>{{ run.error.message }}</strong>
            <small v-if="run.error.diagnostic">{{ run.error.diagnostic.technical }}</small>
          </div>
        </article>
      </div>

      <dl class="comparison-deltas">
        <div><dt>Diferença de tempo</dt><dd>{{ formatDelta(comparison.durationDeltaMs, "ms") }}</dd></div>
        <div><dt>Diferença de body</dt><dd>{{ formatDelta(comparison.bodySizeDeltaBytes, "bytes") }}</dd></div>
      </dl>

      <p class="comparison-note">A diferença é uma observação desta execução específica. Ela não prova que um protocolo será sempre mais rápido para todos os tamanhos de body, redes ou servidores.</p>
    </section>

    <div class="layer-flow" aria-label="Camadas observadas da execução">
      <section v-for="layer in layerCards" :key="layer.key" class="layer-card">
        <div class="layer-heading">
          <div class="layer-title">
            <span class="layer-number">{{ layer.number }}</span>
            <div>
              <h4>{{ layer.title }}</h4>
              <p>{{ layer.description }}</p>
            </div>
          </div>
          <span class="layer-state">{{ provenanceLabel(layer.state) }}</span>
        </div>

        <dl class="fact-grid">
          <div v-for="fact in layer.facts" :key="`${layer.key}-${fact.label}`" class="fact-entry">
            <dt>{{ fact.label }}</dt>
            <dd>
              <strong>{{ fact.value }}</strong>
              <small>{{ provenanceLabel(fact.provenance) }}</small>
            </dd>
          </div>
        </dl>

        <p class="layer-note">{{ layer.note }}</p>
      </section>
    </div>

    <footer class="lab-footer">
      <span class="section-label">COMO LER</span>
      <p>Medido usa um intervalo observado pelo cliente. Observado vem de headers, status ou DNS. Indisponível significa que este adapter ainda não consegue ver aquela etapa.</p>
    </footer>
  </div>
</template>

<style scoped>
.protocol-lab-panel {
  display: flex;
  min-height: 420px;
  flex-direction: column;
  gap: 14px;
}

.lab-intro {
  display: flex;
  flex-direction: column;
  gap: 5px;
}

.section-label {
  color: var(--color-text-subtle);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.08em;
}

.lab-intro h3 {
  margin: 0;
  color: var(--color-text);
  font-size: 18px;
}

.lab-intro p,
.lab-footer p {
  margin: 0;
  color: var(--color-text-muted);
  font-size: 12px;
  line-height: 1.45;
}

.lab-target {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
  min-height: 36px;
  border: 1px solid var(--color-border-strong);
  border-radius: 6px;
  padding: 0 10px;
  background: var(--color-code-bg);
}

.method-label {
  flex: 0 0 auto;
  color: var(--color-brand);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 11px;
  font-weight: 700;
}

.lab-target code {
  min-width: 0;
  overflow: hidden;
  color: var(--color-text);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 12px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.target-state {
  flex: 0 0 auto;
  margin-left: auto;
  color: var(--color-text-subtle);
  font-size: 11px;
  white-space: nowrap;
}

.comparison-launch,
.comparison-result {
  border: 1px solid var(--color-border);
  border-radius: 6px;
  background: var(--color-surface-2);
}

.comparison-launch {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
  padding: 12px;
}

.comparison-launch p {
  margin: 5px 0 0;
  color: var(--color-text-muted);
  font-size: 11px;
  line-height: 1.4;
}

.compare-button {
  flex: 0 0 auto;
  min-height: 34px;
  border: 1px solid var(--color-brand);
  border-radius: 5px;
  padding: 7px 11px;
  color: #071113;
  background: var(--color-brand);
  cursor: pointer;
  font-size: 12px;
  font-weight: 700;
  white-space: nowrap;
}

.compare-button:hover:not(:disabled) {
  background: var(--color-brand-strong);
}

.compare-button:disabled {
  cursor: wait;
  opacity: 0.6;
}

.comparison-error {
  margin: 0;
  color: var(--color-danger);
  font-size: 12px;
  line-height: 1.45;
  white-space: pre-wrap;
}

.comparison-result {
  padding: 12px;
}

.comparison-heading,
.comparison-card-heading {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.comparison-heading > div {
  display: flex;
  flex-direction: column;
  gap: 5px;
}

.comparison-heading h3 {
  margin: 0;
  color: var(--color-text);
  font-size: 14px;
}

.comparison-heading > span,
.comparison-card-heading > span {
  color: var(--color-text-subtle);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 10px;
  white-space: nowrap;
}

.comparison-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
  margin-top: 12px;
}

.comparison-card {
  min-width: 0;
  border: 1px solid var(--color-border-strong);
  border-radius: 5px;
  padding: 10px;
  background: var(--color-surface-1);
}

.comparison-card-heading h4 {
  margin: 0;
  color: var(--color-text);
  font-size: 13px;
}

.comparison-card dl,
.comparison-deltas {
  display: grid;
  gap: 6px;
  margin: 12px 0 0;
}

.comparison-card dl > div,
.comparison-deltas > div {
  display: flex;
  justify-content: space-between;
  gap: 12px;
}

.comparison-card dt,
.comparison-deltas dt {
  color: var(--color-text-subtle);
  font-size: 11px;
}

.comparison-card dd,
.comparison-deltas dd {
  margin: 0;
  color: var(--color-text);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 11px;
  text-align: right;
  overflow-wrap: anywhere;
}

.comparison-run-error {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-top: 12px;
  color: var(--color-danger);
  font-size: 11px;
  line-height: 1.4;
}

.comparison-run-error small {
  color: var(--color-text-muted);
  overflow-wrap: anywhere;
}

.comparison-deltas {
  padding-top: 10px;
  border-top: 1px solid var(--color-border);
}

.comparison-note {
  margin: 12px 0 0;
  color: var(--color-text-subtle);
  font-size: 11px;
  line-height: 1.45;
}

.layer-flow {
  display: grid;
  gap: 8px;
}

.layer-card {
  border: 1px solid var(--color-border);
  border-radius: 6px;
  background: var(--color-surface-2);
}

.layer-heading {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  padding: 12px;
}

.layer-title {
  display: flex;
  min-width: 0;
  align-items: flex-start;
  gap: 10px;
}

.layer-number {
  flex: 0 0 auto;
  color: var(--color-brand-warm);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 11px;
  font-weight: 700;
}

.layer-title h4 {
  margin: 0;
  color: var(--color-text);
  font-size: 14px;
}

.layer-title p {
  margin: 4px 0 0;
  color: var(--color-text-muted);
  font-size: 11px;
  line-height: 1.4;
}

.layer-state {
  flex: 0 0 auto;
  border: 1px solid var(--color-border-strong);
  border-radius: 999px;
  padding: 4px 7px;
  color: var(--color-text-muted);
  font-size: 10px;
  white-space: nowrap;
}

.fact-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 1px;
  margin: 0;
  border-top: 1px solid var(--color-border);
  border-bottom: 1px solid var(--color-border);
  background: var(--color-border);
}

.fact-entry {
  min-width: 0;
  padding: 9px 10px;
  background: var(--color-surface-1);
}

.fact-entry dt {
  color: var(--color-text-subtle);
  font-size: 10px;
}

.fact-entry dd {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 4px;
  margin: 5px 0 0;
}

.fact-entry strong {
  overflow-wrap: anywhere;
  color: var(--color-text);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 11px;
  font-weight: 600;
}

.fact-entry small {
  color: var(--color-text-subtle);
  font-size: 10px;
}

.layer-note {
  margin: 0;
  padding: 9px 12px 11px;
  color: var(--color-text-subtle);
  font-size: 11px;
  line-height: 1.45;
}

.lab-footer {
  display: flex;
  flex-direction: column;
  gap: 5px;
  padding-top: 2px;
}

@media (max-width: 640px) {
  .target-state {
    display: none;
  }

  .fact-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .comparison-grid {
    grid-template-columns: 1fr;
  }

  .comparison-launch {
    align-items: stretch;
    flex-direction: column;
  }

  .layer-heading {
    flex-direction: column;
  }
}
</style>
