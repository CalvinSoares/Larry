<script setup lang="ts">
import { ref } from "vue";

import AppModal from "../../components/ui/AppModal.vue";
import type { RequestDefinition } from "../../types/api";

const props = defineProps<{
  requests: RequestDefinition[];
  activeRequestId: string;
}>();

const emit = defineEmits<{
  (event: "select-request", requestId: string): void;
  (event: "add-request"): void;
  (event: "duplicate-request", requestId: string): void;
  (event: "remove-request", requestId: string): void;
}>();

const pendingRemoval = ref<RequestDefinition | null>(null);

function selectRequest(requestId: string) {
  emit("select-request", requestId);
}

function duplicateRequest(requestId: string) {
  emit("duplicate-request", requestId);
}

function removeRequest(requestId: string) {
  if (props.requests.length <= 1) {
    return;
  }

  const request = props.requests.find((item) => item.id === requestId);
  if (!request) {
    return;
  }

  pendingRemoval.value = request;
}

function cancelRemoval() {
  pendingRemoval.value = null;
}

function confirmRemoval() {
  if (!pendingRemoval.value) {
    return;
  }

  emit("remove-request", pendingRemoval.value.id);
  pendingRemoval.value = null;
}
</script>

<template>
  <section class="collection-sidebar-content" aria-label="Navegação da collection">
    <AppModal
      :open="Boolean(pendingRemoval)"
      variant="danger"
      title="Remover request?"
      :description='pendingRemoval ? `A request "${pendingRemoval.name}" será removida desta collection local. Essa alteração ainda não foi salva no arquivo.` : ""'
      :close-on-backdrop="false"
      primary-label="Remover request"
      secondary-label="Cancelar"
      @close="cancelRemoval"
      @confirm="confirmRemoval"
    />

    <div class="sidebar-heading">
      <div>
        <p class="eyebrow">COLLECTIONS</p>
        <h2>Minha API</h2>
      </div>

      <button
        type="button"
        class="add-request-button"
        aria-label="Adicionar request"
        title="Adicionar request"
        @click="emit('add-request')"
      >
        <svg class="add-icon" viewBox="0 0 16 16" aria-hidden="true">
          <path d="M8 3v10M3 8h10" />
        </svg>
      </button>
    </div>

    <div class="collection-meta">
      <span>Requests locais</span>
      <span>{{ props.requests.length }}</span>
    </div>

    <div class="request-list" aria-label="Requests da collection">
      <div
        v-for="item in props.requests"
        :key="item.id"
        class="request-item"
        :class="{ 'request-item-active': item.id === props.activeRequestId }"
        role="button"
        tabindex="0"
        :aria-current="item.id === props.activeRequestId ? 'page' : undefined"
        @click="selectRequest(item.id)"
        @keydown.enter="selectRequest(item.id)"
        @keydown.space.prevent="selectRequest(item.id)"
      >
        <div class="request-item-main">
          <div class="request-item-title">
            <span class="method-badge">{{ item.method }}</span>
            <strong :title="item.name">{{ item.name }}</strong>
          </div>
          <span class="request-item-url" :title="item.url">{{ item.url }}</span>
        </div>

        <div class="request-item-actions" @click.stop>
          <button
            type="button"
            class="row-action"
            :aria-label="`Duplicar ${item.name}`"
            :title="`Duplicar ${item.name}`"
            @click="duplicateRequest(item.id)"
          >
            Duplicar
          </button>
          <button
            type="button"
            class="row-action danger-action"
            :aria-label="`Remover ${item.name}`"
            :title="`Remover ${item.name}`"
            :disabled="props.requests.length <= 1"
            @click="removeRequest(item.id)"
          >
            Remover
          </button>
        </div>
      </div>

      <p v-if="props.requests.length === 0" class="empty-list">
        Nenhuma request nesta collection.
      </p>
    </div>
  </section>
</template>

<style scoped>
.collection-sidebar-content {
  display: flex;
  flex-direction: column;
  gap: 14px;
  min-height: 100%;
  padding: 16px;
}

.sidebar-heading,
.collection-meta,
.request-item,
.request-item-title {
  display: flex;
  align-items: center;
}

.sidebar-heading,
.collection-meta,
.request-item {
  justify-content: space-between;
  gap: 10px;
}

.eyebrow {
  margin: 0 0 5px;
  color: var(--color-text-subtle);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.08em;
}

h2 {
  margin: 0;
  color: var(--color-text);
  font-size: 16px;
}

button {
  min-height: 30px;
  border: 1px solid var(--color-border-strong);
  border-radius: 5px;
  background: var(--color-surface-2);
  color: var(--color-text-muted);
  cursor: pointer;
  font: inherit;
}

button:hover:not(:disabled) {
  border-color: var(--color-brand);
  background: var(--color-surface-3);
  color: var(--color-text);
}

button:disabled {
  cursor: not-allowed;
  opacity: 0.45;
}

.add-request-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
}

.add-icon {
  width: 16px;
  height: 16px;
  fill: none;
  stroke: currentColor;
  stroke-linecap: round;
  stroke-width: 1.3;
}

.collection-meta {
  padding-bottom: 10px;
  border-bottom: 1px solid var(--color-border);
  color: var(--color-text-subtle);
  font-size: 12px;
}

.request-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-height: 0;
}

.request-item {
  align-items: flex-start;
  border: 1px solid transparent;
  border-radius: 6px;
  padding: 9px;
  cursor: pointer;
  background: transparent;
}

.request-item:hover,
.request-item:focus-visible {
  border-color: var(--color-border-strong);
  outline: none;
  background: var(--color-surface-2);
}

.request-item-active {
  border-color: var(--color-brand-strong);
  background: #172b30;
}

.request-item-main {
  min-width: 0;
}

.request-item-title {
  min-width: 0;
  gap: 7px;
}

.request-item-title strong {
  overflow: hidden;
  color: var(--color-text);
  font-size: 13px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.method-badge {
  flex: 0 0 auto;
  color: var(--color-success);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 10px;
  font-weight: 800;
}

.request-item-url {
  display: block;
  max-width: 100%;
  margin-top: 4px;
  overflow: hidden;
  color: var(--color-text-subtle);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 11px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.request-item-actions {
  display: flex;
  flex: 0 0 auto;
  gap: 4px;
  opacity: 0;
  transition: opacity 120ms ease;
}

.request-item:hover .request-item-actions,
.request-item:focus-within .request-item-actions,
.request-item-active .request-item-actions {
  opacity: 1;
}

.row-action {
  padding: 4px 6px;
  font-size: 10px;
}

.danger-action:hover:not(:disabled) {
  border-color: var(--color-danger);
  color: var(--color-danger);
}

.empty-list {
  margin: 0;
  border: 1px dashed var(--color-border-strong);
  padding: 12px;
  color: var(--color-text-muted);
  font-size: 12px;
  text-align: center;
}

@media (max-width: 900px) {
  .collection-sidebar-content {
    max-height: 300px;
  }
}
</style>
