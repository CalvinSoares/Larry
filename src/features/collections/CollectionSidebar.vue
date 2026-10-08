<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref } from "vue";
import type { ComponentPublicInstance } from "vue";

import AppModal from "../../components/ui/AppModal.vue";
import type { RequestDefinition } from "../../types/api";

const props = defineProps<{
  requests: RequestDefinition[];
  activeRequestId: string;
}>();

const emit = defineEmits<{
  (event: "select-request", requestId: string): void;
  (event: "add-request"): void;
  (event: "rename-request", requestId: string, name: string): void;
  (event: "duplicate-request", requestId: string): void;
  (event: "remove-request", requestId: string): void;
}>();

const pendingRemoval = ref<RequestDefinition | null>(null);
const openMenuId = ref<string | null>(null);
const editingRequestId = ref<string | null>(null);
const renameDraft = ref("");
const renameInput = ref<HTMLInputElement | null>(null);

function selectRequest(requestId: string) {
  openMenuId.value = null;
  emit("select-request", requestId);
}

function duplicateRequest(requestId: string) {
  openMenuId.value = null;
  emit("duplicate-request", requestId);
}

function toggleMenu(requestId: string) {
  openMenuId.value = openMenuId.value === requestId ? null : requestId;
}

function closeMenuOnOutsidePointer(event: PointerEvent) {
  const target = event.target;

  if (!(target instanceof Element) || !target.closest(".request-item-actions")) {
    openMenuId.value = null;
  }
}

async function startRename(request: RequestDefinition) {
  openMenuId.value = null;
  editingRequestId.value = request.id;
  renameDraft.value = request.name;

  await nextTick();
  renameInput.value?.focus();
  renameInput.value?.select();
}

function cancelRename() {
  editingRequestId.value = null;
  renameDraft.value = "";
}

function setRenameInput(element: Element | ComponentPublicInstance | null) {
  renameInput.value = element instanceof HTMLInputElement ? element : null;
}

function confirmRename(request: RequestDefinition) {
  const name = renameDraft.value.trim();

  if (name && name !== request.name) {
    emit("rename-request", request.id, name);
  }

  cancelRename();
}

function removeRequest(requestId: string) {
  if (props.requests.length <= 1) {
    return;
  }

  const request = props.requests.find((item) => item.id === requestId);
  if (!request) {
    return;
  }

  openMenuId.value = null;
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

onMounted(() => {
  document.addEventListener("pointerdown", closeMenuOnOutsidePointer);
});

onBeforeUnmount(() => {
  document.removeEventListener("pointerdown", closeMenuOnOutsidePointer);
});
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
      <article
        v-for="item in props.requests"
        :key="item.id"
        class="request-item"
        :class="{ 'request-item-active': item.id === props.activeRequestId }"
      >
        <form
          v-if="editingRequestId === item.id"
          class="request-rename-form"
          @submit.prevent="confirmRename(item)"
        >
          <input
            :ref="setRenameInput"
            v-model="renameDraft"
            :aria-label="`Novo nome para ${item.name}`"
            type="text"
            required
            @keydown.esc.prevent="cancelRename"
            @click.stop
          />
        </form>

        <button
          v-else
          class="request-item-select"
          type="button"
          :aria-current="item.id === props.activeRequestId ? 'page' : undefined"
          :title="item.name"
          @click="selectRequest(item.id)"
        >
          <span class="request-item-main">
            <span class="request-item-title">
              <span class="method-badge">{{ item.method }}</span>
              <strong>{{ item.name }}</strong>
            </span>
          </span>
        </button>

        <div class="request-item-actions">
          <button
            class="request-menu-trigger"
            type="button"
            :aria-label="`Ações para ${item.name}`"
            :aria-expanded="openMenuId === item.id"
            aria-haspopup="menu"
            :title="`Ações para ${item.name}`"
            @click.stop="toggleMenu(item.id)"
          >
            <svg viewBox="0 0 16 16" aria-hidden="true">
              <circle cx="3" cy="8" r="1" />
              <circle cx="8" cy="8" r="1" />
              <circle cx="13" cy="8" r="1" />
            </svg>
          </button>

          <div
            v-if="openMenuId === item.id"
            class="request-menu"
            role="menu"
            :aria-label="`Ações da request ${item.name}`"
            @keydown.esc.prevent="openMenuId = null"
          >
            <button type="button" role="menuitem" @click="startRename(item)">Renomear</button>
            <button type="button" role="menuitem" @click="duplicateRequest(item.id)">Duplicar</button>
            <button
              type="button"
              role="menuitem"
              class="danger-action"
              :disabled="props.requests.length <= 1"
              @click="removeRequest(item.id)"
            >
              Remover
            </button>
          </div>
        </div>
      </article>

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
  border-color: var(--color-border-strong);
  background: transparent;
  color: var(--color-text-muted);
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
  gap: 1px;
  min-height: 0;
}

.request-item {
  position: relative;
  height: 32px;
  min-height: 32px;
  display: flex;
  align-items: center;
  gap: 4px;
  border: 0;
  border-radius: 4px;
  padding: 1px 6px;
  background: transparent;
}

.request-item:hover,
.request-item:focus-within {
  outline: none;
  background: var(--color-surface-2);
}

.request-item-active {
  background: #142328;
}

.request-item-select,
.request-rename-form {
  min-width: 0;
  flex: 1 1 auto;
}

.request-item-select {
  display: block;
  min-height: 30px;
  border: 0;
  padding: 0 2px;
  overflow: hidden;
  text-align: left;
}

.request-item-select:hover:not(:disabled) {
  border-color: transparent;
  background: transparent;
}

.request-item-main {
  display: flex;
  align-items: center;
  min-width: 0;
}

.request-item-title {
  min-width: 0;
  gap: 7px;
}

.request-item-title strong {
  overflow: hidden;
  color: var(--color-text);
  font-size: 12px;
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

.request-item-actions {
  position: relative;
  display: flex;
  flex: 0 0 auto;
  opacity: 0;
  transition: opacity 120ms ease;
}

.request-item:hover .request-item-actions,
.request-item:focus-within .request-item-actions,
.request-item-active .request-item-actions {
  opacity: 1;
}

.request-menu-trigger {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  min-height: 28px;
  border-color: transparent;
  padding: 0;
}

.request-menu-trigger:hover:not(:disabled),
.request-menu-trigger[aria-expanded="true"] {
  border-color: var(--color-border-strong);
  background: transparent;
  color: var(--color-text-muted);
}

.request-menu-trigger svg {
  width: 16px;
  height: 16px;
  fill: currentColor;
}

.request-menu {
  position: absolute;
  z-index: 2;
  top: calc(100% + 4px);
  right: 0;
  display: grid;
  min-width: 132px;
  gap: 2px;
  padding: 4px;
  border: 1px solid var(--color-border-strong);
  border-radius: 6px;
  background: var(--color-surface-1);
  box-shadow: 0 3px 8px rgb(0 0 0 / 20%);
}

.request-item:last-child .request-menu {
  top: auto;
  bottom: calc(100% + 4px);
}

.request-menu button {
  min-height: 28px;
  border-color: transparent;
  padding: 4px 8px;
  color: var(--color-text-muted);
  text-align: left;
  font-size: 12px;
}

.request-menu button:hover:not(:disabled) {
  border-color: transparent;
  background: transparent;
  color: var(--color-text);
}

.request-rename-form {
  display: flex;
  align-items: center;
  min-height: 30px;
}

.request-rename-form input {
  width: 100%;
  min-height: 30px;
  border: 1px solid var(--color-border-strong);
  border-radius: 4px;
  padding: 5px 7px;
  color: var(--color-text);
  background: var(--color-surface-2);
  font: inherit;
  font-size: 13px;
}

.danger-action:hover:not(:disabled) {
  border-color: transparent;
  background: transparent;
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
