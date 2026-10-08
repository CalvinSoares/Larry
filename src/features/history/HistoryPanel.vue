<script setup lang="ts">
import { ref } from "vue";

import type { HistoryComparison, HistorySummary } from "../../types/api";

const props = withDefaults(
  defineProps<{
    entries: HistorySummary[];
    comparison: HistoryComparison | null;
    isLoading: boolean;
    embedded?: boolean;
  }>(),
  {
    embedded: false,
  },
);

const emit = defineEmits<{
  (event: "refresh"): void;
  (event: "replay", id: string): void;
  (event: "compare", leftId: string, rightId: string): void;
}>();

const selectedIds = ref<string[]>([]);

function toggleSelection(id: string) {
  if (selectedIds.value.includes(id)) {
    selectedIds.value = selectedIds.value.filter((item) => item !== id);
    return;
  }

  selectedIds.value = [...selectedIds.value.slice(-1), id];
}

function compareSelected() {
  if (selectedIds.value.length !== 2) {
    return;
  }

  emit("compare", selectedIds.value[0], selectedIds.value[1]);
}

function formatTime(timestamp: number) {
  return new Date(timestamp).toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}
</script>

<template>
  <section class="history-panel">
    <div class="history-heading">
      <div v-if="!props.embedded">
        <p class="eyebrow">HISTORY</p>
        <h3>Execuções locais</h3>
      </div>
      <button type="button" :disabled="isLoading" @click="emit('refresh')">
        {{ isLoading ? "Atualizando..." : "Atualizar" }}
      </button>
    </div>

    <p class="history-note">
      Requests e responses são sanitizadas antes de serem salvas no SQLite local.
    </p>

    <div v-if="entries.length" class="history-list">
      <article v-for="entry in props.entries" :key="entry.id" class="history-row">
        <div class="history-row-topline">
          <label class="history-selection">
            <input
              type="checkbox"
              :checked="selectedIds.includes(entry.id)"
              :aria-label="`Selecionar ${entry.requestName} para comparar`"
              @change="toggleSelection(entry.id)"
            />
            <span class="history-method">{{ entry.method }}</span>
            <strong>{{ entry.requestName }}</strong>
          </label>
          <span class="history-outcome" :class="`history-outcome-${entry.outcome}`">
            {{ entry.status ?? entry.errorKind ?? entry.outcome }}
          </span>
        </div>

        <p class="history-url">{{ entry.url }}</p>
        <p class="history-meta">
          {{ formatTime(entry.createdAtMs) }}
          <span v-if="entry.environmentName">· {{ entry.environmentName }}</span>
          <span v-if="entry.durationMs !== null">· {{ entry.durationMs }} ms</span>
          <span v-if="entry.bodyTruncated">· body omitido</span>
        </p>

        <div class="history-actions">
          <button type="button" @click="emit('replay', entry.id)">Repetir</button>
        </div>
      </article>
    </div>

    <p v-else class="history-empty">
      Nenhuma execução persistida ainda. Envie uma request para criar o primeiro registro.
    </p>

    <div class="history-compare-actions">
      <span>{{ selectedIds.length }}/2 selecionadas</span>
      <button type="button" :disabled="selectedIds.length !== 2" @click="compareSelected">
        Comparar
      </button>
    </div>

    <div v-if="comparison" class="comparison-summary">
      <p class="eyebrow">COMPARAÇÃO</p>
      <strong>
        {{ comparison.statusChanged ? "Status diferente" : "Status igual" }}
      </strong>
      <span>{{ comparison.bodyNote }}</span>
      <span>
        Headers {{ comparison.headersChanged ? "diferentes" : "iguais" }}.
        Latência {{ comparison.durationDeltaMs === null ? "indisponível" : `${comparison.durationDeltaMs} ms de diferença` }}.
      </span>
    </div>
  </section>
</template>

<style scoped>
.history-panel {
  display: flex;
  flex-direction: column;
  gap: 10px;
  border-bottom: 1px solid var(--color-border);
  padding: 16px;
  text-align: left;
}

.history-heading,
.history-row-topline,
.history-compare-actions,
.history-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.history-heading h3 {
  margin: 0;
  color: var(--color-text);
}

.eyebrow {
  margin: 0 0 4px;
  color: var(--color-text-subtle);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.08em;
}

.history-note,
.history-empty,
.history-meta,
.history-url,
.comparison-summary {
  margin: 0;
  color: var(--color-text-muted);
  font-size: 11px;
  line-height: 1.4;
}

button {
  min-height: 28px;
  border: 1px solid var(--color-border-strong);
  border-radius: 5px;
  padding: 4px 8px;
  color: var(--color-text);
  background: var(--color-surface-2);
  cursor: pointer;
  font: inherit;
  font-size: 11px;
}

button:hover:not(:disabled) {
  border-color: var(--color-border-strong);
  background: transparent;
  color: var(--color-text-muted);
}

button:disabled {
  cursor: not-allowed;
  opacity: 0.55;
}

.history-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.history-row {
  display: flex;
  flex-direction: column;
  gap: 5px;
  border: 1px solid var(--color-border);
  border-radius: 6px;
  padding: 8px;
  background: var(--color-surface-2);
}

.history-selection {
  display: flex;
  min-width: 0;
  align-items: center;
  gap: 6px;
  color: var(--color-text);
  font-size: 11px;
}

.history-selection input {
  accent-color: var(--color-brand);
}

.history-selection strong {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.history-method {
  color: var(--color-brand);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 10px;
  font-weight: 700;
}

.history-outcome {
  flex: 0 0 auto;
  color: var(--color-text-muted);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 10px;
}

.history-outcome-success {
  color: var(--color-success);
}

.history-outcome-http_error,
.history-outcome-failed {
  color: var(--color-danger);
}

.history-url {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.history-meta {
  color: var(--color-text-subtle);
}

.history-actions {
  justify-content: flex-end;
}

.history-compare-actions {
  border-top: 1px solid var(--color-border);
  padding-top: 10px;
  color: var(--color-text-subtle);
  font-size: 11px;
}

.comparison-summary {
  display: flex;
  flex-direction: column;
  gap: 4px;
  border: 1px solid var(--color-border-strong);
  padding: 9px;
}

.comparison-summary strong {
  color: var(--color-text);
}

@media (max-width: 520px) {
  .history-heading,
  .history-compare-actions {
    align-items: stretch;
    flex-direction: column;
  }

  .history-heading button,
  .history-compare-actions button {
    width: 100%;
  }
}
</style>
