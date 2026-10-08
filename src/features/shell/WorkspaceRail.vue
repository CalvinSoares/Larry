<script setup lang="ts">
import { ref } from "vue";

import AppModal from "../../components/ui/AppModal.vue";
import CollectionManager from "../collections/CollectionManager.vue";
import CollectionSidebar from "../collections/CollectionSidebar.vue";
import EnvironmentPanel from "../environments/EnvironmentPanel.vue";
import HistoryPanel from "../history/HistoryPanel.vue";
import type {
  CollectionFile,
  EnvironmentFile,
  HistoryComparison,
  HistorySummary,
  RequestDefinition,
} from "../../types/api";

const props = defineProps<{
  requests: RequestDefinition[];
  activeRequestId: string;
  historyEntries: HistorySummary[];
  historyComparison: HistoryComparison | null;
  isHistoryLoading: boolean;
}>();

const emit = defineEmits<{
  (event: "select-request", requestId: string): void;
  (event: "add-request"): void;
  (event: "rename-request", requestId: string, name: string): void;
  (event: "duplicate-request", requestId: string): void;
  (event: "remove-request", requestId: string): void;
  (event: "loaded-collection", collection: CollectionFile): void;
  (event: "environment-changed", environment: EnvironmentFile | null): void;
  (event: "refresh-history"): void;
  (event: "replay-history", id: string): void;
  (event: "compare-history", leftId: string, rightId: string): void;
}>();

type RailTab = "requests" | "tools";

const activeTab = ref<RailTab>("requests");
type LocalTool = "collection" | "environment" | "history";
const activeTool = ref<LocalTool | null>(null);

function openTool(tool: LocalTool) {
  activeTool.value = tool;
}

function closeTool() {
  activeTool.value = null;
}
</script>

<template>
  <aside class="workspace-rail" aria-label="Navegação do workspace">
    <div class="rail-tabs" role="tablist" aria-label="Áreas do workspace">
      <button
        class="rail-tab"
        :class="{ 'rail-tab-active': activeTab === 'requests' }"
        type="button"
        role="tab"
        :aria-selected="activeTab === 'requests'"
        aria-controls="rail-requests-panel"
        @click="activeTab = 'requests'"
      >
        Requests
      </button>
      <button
        class="rail-tab"
        :class="{ 'rail-tab-active': activeTab === 'tools' }"
        type="button"
        role="tab"
        :aria-selected="activeTab === 'tools'"
        aria-controls="rail-tools-panel"
        @click="activeTab = 'tools'"
      >
        Ferramentas locais
      </button>
    </div>

    <section
      v-if="activeTab === 'requests'"
      id="rail-requests-panel"
      class="rail-content rail-requests"
      role="tabpanel"
    >
      <CollectionSidebar
        :requests="props.requests"
        :active-request-id="props.activeRequestId"
        @select-request="emit('select-request', $event)"
        @add-request="emit('add-request')"
        @rename-request="(requestId, name) => emit('rename-request', requestId, name)"
        @duplicate-request="emit('duplicate-request', $event)"
        @remove-request="emit('remove-request', $event)"
      />
    </section>

    <section
      v-else
      id="rail-tools-panel"
      class="rail-content rail-tools"
      role="tabpanel"
    >
      <div class="tools-header">
        <p class="eyebrow">LOCAL TOOLS</p>
        <h2>Ferramentas locais</h2>
        <p>Persistência, secrets e histórico ficam fora do fluxo principal da request.</p>
      </div>

      <div class="tool-list">
        <button class="tool-entry" type="button" @click="openTool('collection')">
          <span class="tool-entry-copy">
            <strong>Collection file</strong>
            <small>YAML legível e Git local</small>
          </span>
          <span class="tool-entry-action">Abrir</span>
        </button>

        <button class="tool-entry" type="button" @click="openTool('environment')">
          <span class="tool-entry-copy">
            <strong>Environment</strong>
            <small>Variáveis públicas e secrets separados</small>
          </span>
          <span class="tool-entry-action">Abrir</span>
        </button>

        <button class="tool-entry" type="button" @click="openTool('history')">
          <span class="tool-entry-copy">
            <strong>History</strong>
            <small>{{ props.historyEntries.length }} execuções no SQLite local</small>
          </span>
          <span class="tool-entry-action">Abrir</span>
        </button>
      </div>

      <p class="tools-note">Nenhum dado local é enviado para um backend do Larry.</p>
    </section>
  </aside>

  <AppModal
    :open="activeTool === 'collection'"
    :keep-mounted="true"
    kicker="FILE-FIRST"
    title="Collection local"
    description="Salve e carregue a collection como um arquivo YAML versionável."
    primary-label="Fechar"
    secondary-label=""
    @close="closeTool"
    @confirm="closeTool"
  >
    <CollectionManager
      :requests="props.requests"
      :embedded="true"
      @loaded-collection="emit('loaded-collection', $event)"
    />
  </AppModal>

  <AppModal
    :open="activeTool === 'environment'"
    :keep-mounted="true"
    kicker="SECRETS"
    title="Environment local"
    description="Valores públicos ficam no YAML; secrets continuam no Credential Manager."
    primary-label="Fechar"
    secondary-label=""
    @close="closeTool"
    @confirm="closeTool"
  >
    <EnvironmentPanel
      :embedded="true"
      @environment-changed="emit('environment-changed', $event)"
    />
  </AppModal>

  <AppModal
    :open="activeTool === 'history'"
    :keep-mounted="true"
    size="wide"
    kicker="LOCAL HISTORY"
    title="Execuções locais"
    description="Reveja, repita e compare execuções armazenadas localmente."
    variant="info"
    primary-label="Fechar"
    secondary-label=""
    @close="closeTool"
    @confirm="closeTool"
  >
    <HistoryPanel
      :entries="props.historyEntries"
      :comparison="props.historyComparison"
      :is-loading="props.isHistoryLoading"
      :embedded="true"
      @refresh="emit('refresh-history')"
      @replay="emit('replay-history', $event)"
      @compare="(leftId, rightId) => emit('compare-history', leftId, rightId)"
    />
  </AppModal>
</template>

<style scoped>
.workspace-rail {
  display: flex;
  min-width: 0;
  min-height: 0;
  flex-direction: column;
  overflow: hidden;
  background: #12151a;
}

.rail-tabs {
  display: flex;
  flex: 0 0 auto;
  min-width: 0;
  border-bottom: 1px solid var(--color-border);
}

.rail-tab {
  min-width: 0;
  min-height: 40px;
  border: 0;
  border-bottom: 2px solid transparent;
  padding: 0 12px;
  overflow: hidden;
  color: var(--color-text-muted);
  background: transparent;
  cursor: pointer;
  font-size: 11px;
  font-weight: 700;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.rail-tab:first-child {
  flex: 0 0 auto;
}

.rail-tab:last-child {
  flex: 1 1 auto;
}

.rail-tab:hover {
  color: var(--color-text);
  background: transparent;
}

.rail-tab-active {
  border-bottom-color: var(--color-brand);
  color: var(--color-text);
}

.rail-content {
  min-height: 0;
  flex: 1 1 auto;
  overflow-y: auto;
}

.rail-requests {
  display: flex;
  flex-direction: column;
}

.rail-tools {
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 16px;
  background: var(--color-surface-1);
}

.tools-header {
  padding-bottom: 2px;
}

.tools-header h2 {
  margin: 0;
  color: var(--color-text);
  font-size: 16px;
}

.tools-header p:not(.eyebrow),
.tools-note {
  margin: 5px 0 0;
  color: var(--color-text-muted);
  font-size: 12px;
  line-height: 1.45;
}

.eyebrow {
  margin: 0 0 5px;
  color: var(--color-text-subtle);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.08em;
}

.tool-list {
  display: flex;
  flex-direction: column;
  gap: 1px;
  border-top: 1px solid var(--color-border);
  border-bottom: 1px solid var(--color-border);
}

.tool-entry {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  min-width: 0;
  min-height: 64px;
  border: 0;
  border-bottom: 1px solid var(--color-border);
  padding: 10px 2px;
  color: var(--color-text);
  background: transparent;
  cursor: pointer;
  text-align: left;
}

.tool-entry:last-child {
  border-bottom: 0;
}

.tool-entry:hover {
  background: transparent;
  color: var(--color-text-muted);
}

.tool-entry-copy {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 4px;
}

.tool-entry-copy strong {
  overflow: hidden;
  color: inherit;
  font-size: 13px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tool-entry-copy small {
  overflow: hidden;
  color: var(--color-text-subtle);
  font-size: 11px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tool-entry-action {
  flex: 0 0 auto;
  color: var(--color-brand);
  font-size: 11px;
  font-weight: 700;
}

.tools-note {
  margin-top: auto;
  color: var(--color-text-subtle);
  font-size: 11px;
}

@media (max-width: 520px) {
  .rail-tab {
    padding: 0 10px;
  }
}
</style>
