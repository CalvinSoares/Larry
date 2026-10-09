<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import type { ComponentPublicInstance } from "vue";

import AppModal from "../../components/ui/AppModal.vue";
import type { CollectionFolder, RequestDefinition } from "../../types/api";

const props = defineProps<{
  requests: RequestDefinition[];
  folders: CollectionFolder[];
  collectionName: string;
  collectionDirty: boolean;
  activeRequestId: string;
}>();

const emit = defineEmits<{
  (event: "select-request", requestId: string): void;
  (event: "rename-request", requestId: string, name: string): void;
  (event: "duplicate-request", requestId: string): void;
  (event: "remove-request", requestId: string): void;
  (event: "move-request", requestId: string): void;
  (event: "open-collection"): void;
  (event: "open-new-request", folderId: string | null): void;
  (event: "open-new-folder", parentFolderId: string | null): void;
  (event: "open-rename-folder", folderId: string): void;
  (event: "remove-folder", folderId: string): void;
}>();

type TreeRow =
  | { kind: "folder"; folder: CollectionFolder; depth: number }
  | { kind: "request"; request: RequestDefinition; depth: number };

const pendingRemoval = ref<RequestDefinition | null>(null);
const pendingFolderRemoval = ref<CollectionFolder | null>(null);
const openMenuId = ref<string | null>(null);
const openFolderMenuId = ref<string | null>(null);
const collectionMenuOpen = ref(false);
const editingRequestId = ref<string | null>(null);
const renameDraft = ref("");
const renameInput = ref<HTMLInputElement | null>(null);
const expandedFolderIds = ref(new Set<string>());

const totalRequestCount = computed(() => props.requests.length + countFolderRequests(props.folders));

function countFolderRequests(folders: CollectionFolder[]): number {
  return folders.reduce(
    (total, folder) => total + folder.requests.length + countFolderRequests(folder.folders),
    0,
  );
}

function countRequestsInFolder(folder: CollectionFolder): number {
  return folder.requests.length + countFolderRequests(folder.folders);
}

function buildFolderRows(folders: CollectionFolder[], depth: number): TreeRow[] {
  return folders.flatMap((folder) => {
    const folderRow: TreeRow = { kind: "folder", folder, depth };
    if (!expandedFolderIds.value.has(folder.id)) {
      return [folderRow];
    }

    return [
      folderRow,
      ...folder.requests.map((request) => ({ kind: "request", request, depth: depth + 1 }) as TreeRow),
      ...buildFolderRows(folder.folders, depth + 1),
    ];
  });
}

function findFolderPath(
  requestId: string,
  folders: CollectionFolder[],
  ancestors: string[] = [],
): string[] | null {
  for (const folder of folders) {
    const path = [...ancestors, folder.id];
    if (folder.requests.some((request) => request.id === requestId)) {
      return path;
    }

    const nestedPath = findFolderPath(requestId, folder.folders, path);
    if (nestedPath) {
      return nestedPath;
    }
  }

  return null;
}

const treeRows = computed<TreeRow[]>(() => [
  ...props.requests.map((request) => ({ kind: "request", request, depth: 0 }) as TreeRow),
  ...buildFolderRows(props.folders, 0),
]);

watch(
  [() => props.activeRequestId, () => props.folders],
  ([requestId, folders]) => {
    const path = findFolderPath(requestId, folders);
    if (!path) {
      return;
    }

    const next = new Set(expandedFolderIds.value);
    path.forEach((folderId) => next.add(folderId));
    expandedFolderIds.value = next;
  },
  { deep: true, immediate: true },
);

function selectRequest(requestId: string) {
  openMenuId.value = null;
  openFolderMenuId.value = null;
  collectionMenuOpen.value = false;
  emit("select-request", requestId);
}

function duplicateRequest(requestId: string) {
  openMenuId.value = null;
  emit("duplicate-request", requestId);
}

function moveRequest(requestId: string) {
  openMenuId.value = null;
  emit("move-request", requestId);
}

function toggleMenu(requestId: string) {
  collectionMenuOpen.value = false;
  openFolderMenuId.value = null;
  openMenuId.value = openMenuId.value === requestId ? null : requestId;
}

function toggleFolderMenu(folderId: string) {
  collectionMenuOpen.value = false;
  openMenuId.value = null;
  openFolderMenuId.value = openFolderMenuId.value === folderId ? null : folderId;
}

function toggleCollectionMenu() {
  openMenuId.value = null;
  openFolderMenuId.value = null;
  collectionMenuOpen.value = !collectionMenuOpen.value;
}

function openNewRequest(folderId: string | null = null) {
  collectionMenuOpen.value = false;
  openFolderMenuId.value = null;
  emit("open-new-request", folderId);
}

function openNewFolder(parentFolderId: string | null = null) {
  collectionMenuOpen.value = false;
  openFolderMenuId.value = null;
  emit("open-new-folder", parentFolderId);
}

function openRenameFolder(folderId: string) {
  openFolderMenuId.value = null;
  emit("open-rename-folder", folderId);
}

function openCollection() {
  collectionMenuOpen.value = false;
  emit("open-collection");
}

function closePopoverOnOutsidePointer(event: PointerEvent) {
  const target = event.target;

  if (!(target instanceof Element) || !target.closest(".sidebar-popover")) {
    openMenuId.value = null;
    openFolderMenuId.value = null;
    collectionMenuOpen.value = false;
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
  if (totalRequestCount.value <= 1) {
    return;
  }

  const row = treeRows.value.find(
    (candidate) => candidate.kind === "request" && candidate.request.id === requestId,
  );
  if (!row || row.kind !== "request") {
    return;
  }

  openMenuId.value = null;
  pendingRemoval.value = row.request;
}

function removeFolder(folder: CollectionFolder) {
  const remainingRequests = totalRequestCount.value - countRequestsInFolder(folder);
  if (remainingRequests < 1) {
    return;
  }

  openFolderMenuId.value = null;
  pendingFolderRemoval.value = folder;
}

function methodLabel(request: RequestDefinition) {
  return request.method.toUpperCase();
}

function methodClass(request: RequestDefinition) {
  return `method-${request.method.toLowerCase()}`;
}

function toggleFolder(folderId: string) {
  const next = new Set(expandedFolderIds.value);
  if (next.has(folderId)) {
    next.delete(folderId);
  } else {
    next.add(folderId);
  }
  expandedFolderIds.value = next;
}

function cancelRemoval() {
  pendingRemoval.value = null;
}

function cancelFolderRemoval() {
  pendingFolderRemoval.value = null;
}

function confirmRemoval() {
  if (!pendingRemoval.value) {
    return;
  }

  emit("remove-request", pendingRemoval.value.id);
  pendingRemoval.value = null;
}

function confirmFolderRemoval() {
  const folder = pendingFolderRemoval.value;
  if (!folder) {
    return;
  }

  const remainingRequests = totalRequestCount.value - countRequestsInFolder(folder);
  if (remainingRequests < 1) {
    pendingFolderRemoval.value = null;
    return;
  }

  emit("remove-folder", folder.id);
  pendingFolderRemoval.value = null;
}

onMounted(() => {
  document.addEventListener("pointerdown", closePopoverOnOutsidePointer);
});

onBeforeUnmount(() => {
  document.removeEventListener("pointerdown", closePopoverOnOutsidePointer);
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

    <AppModal
      :open="Boolean(pendingFolderRemoval)"
      variant="danger"
      title="Remover pasta?"
      :description='pendingFolderRemoval ? `A pasta "${pendingFolderRemoval.name}" e suas subpastas serão removidas desta collection local. Essa alteração ainda não foi salva no arquivo.` : ""'
      :close-on-backdrop="false"
      primary-label="Remover pasta"
      secondary-label="Cancelar"
      @close="cancelFolderRemoval"
      @confirm="confirmFolderRemoval"
    />

    <header class="collection-sidebar-header" @contextmenu.prevent="toggleCollectionMenu">
      <div
        class="collection-title"
        :title="props.collectionDirty ? `${props.collectionName} · alterações não salvas` : props.collectionName"
      >
        <svg class="collection-icon" viewBox="0 0 16 16" aria-hidden="true">
          <path d="M2.5 4.5h4l1.4 1.6h5.6v6.4h-11z" />
          <path d="M2.5 4.5v-1h3.8l1.3 1.5" />
        </svg>
        <span>{{ props.collectionName }}</span>
        <span
          v-if="props.collectionDirty"
          class="collection-dirty-dot"
          aria-label="Alterações não salvas"
          title="Alterações não salvas"
        ></span>
      </div>

      <div class="sidebar-header-actions">
        <div class="sidebar-popover">
          <button
            class="icon-button"
            type="button"
            aria-label="Ações da collection"
            :aria-expanded="collectionMenuOpen"
            aria-haspopup="menu"
            title="Ações da collection"
            @click.stop="toggleCollectionMenu"
          >
            <svg viewBox="0 0 16 16" aria-hidden="true">
              <circle cx="3" cy="8" r="1" />
              <circle cx="8" cy="8" r="1" />
              <circle cx="13" cy="8" r="1" />
            </svg>
          </button>

          <div v-if="collectionMenuOpen" class="collection-menu" role="menu" @keydown.esc.prevent="collectionMenuOpen = false">
            <button type="button" role="menuitem" @click="openNewRequest()">Nova Request</button>
            <button type="button" role="menuitem" @click="openNewFolder()">Nova pasta</button>
            <button type="button" role="menuitem" @click="openCollection">Abrir YAML</button>
          </div>
        </div>

        <button class="icon-button" type="button" aria-label="Nova request" title="Nova request" @click="openNewRequest()">
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <path d="M8 3v10M3 8h10" />
          </svg>
        </button>
      </div>
    </header>

    <div class="collection-tree" role="tree" aria-label="Requests da collection">
      <div class="tree-root" role="treeitem" aria-expanded="true">
        <svg class="tree-chevron" viewBox="0 0 16 16" aria-hidden="true">
          <path d="m4 6 4 4 4-4" />
        </svg>
        <svg class="tree-folder" viewBox="0 0 16 16" aria-hidden="true">
          <path d="M2.5 4.5h4l1.4 1.6h5.6v6.4h-11z" />
        </svg>
        <span>{{ props.collectionName }}</span>
        <small>{{ totalRequestCount }}</small>
      </div>

      <div class="request-list">
        <template v-for="row in treeRows" :key="`${row.kind}-${row.kind === 'folder' ? row.folder.id : row.request.id}`">
          <div
            v-if="row.kind === 'folder'"
            class="folder-item"
            role="treeitem"
            :aria-expanded="expandedFolderIds.has(row.folder.id)"
            :style="{ paddingLeft: `${7 + row.depth * 14}px` }"
          >
            <button
              class="folder-toggle"
              type="button"
              :title="`${expandedFolderIds.has(row.folder.id) ? 'Recolher' : 'Expandir'} ${row.folder.name}`"
              @click="toggleFolder(row.folder.id)"
            >
              <svg class="folder-chevron" :class="{ 'folder-chevron-open': expandedFolderIds.has(row.folder.id) }" viewBox="0 0 16 16" aria-hidden="true">
                <path d="m6 4 4 4-4 4" />
              </svg>
              <svg class="tree-folder" viewBox="0 0 16 16" aria-hidden="true">
                <path d="M2.5 4.5h4l1.4 1.6h5.6v6.4h-11z" />
              </svg>
              <strong>{{ row.folder.name }}</strong>
              <small>{{ countRequestsInFolder(row.folder) }}</small>
            </button>

            <div class="sidebar-popover folder-item-actions">
              <button
                class="request-menu-trigger folder-menu-trigger"
                type="button"
                :aria-label="`Ações para a pasta ${row.folder.name}`"
                :aria-expanded="openFolderMenuId === row.folder.id"
                aria-haspopup="menu"
                :title="`Ações para a pasta ${row.folder.name}`"
                @click.stop="toggleFolderMenu(row.folder.id)"
              >
                <svg viewBox="0 0 16 16" aria-hidden="true">
                  <circle cx="3" cy="8" r="1" />
                  <circle cx="8" cy="8" r="1" />
                  <circle cx="13" cy="8" r="1" />
                </svg>
              </button>

              <div
                v-if="openFolderMenuId === row.folder.id"
                class="request-menu folder-menu"
                role="menu"
                :aria-label="`Ações da pasta ${row.folder.name}`"
                @keydown.esc.prevent="openFolderMenuId = null"
              >
                <button type="button" role="menuitem" @click="openNewRequest(row.folder.id)">
                  Nova request nesta pasta
                </button>
                <button type="button" role="menuitem" @click="openNewFolder(row.folder.id)">
                  Nova subpasta
                </button>
                <button type="button" role="menuitem" @click="openRenameFolder(row.folder.id)">
                  Renomear
                </button>
                <button
                  type="button"
                  role="menuitem"
                  class="danger-action"
                  :disabled="totalRequestCount - countRequestsInFolder(row.folder) < 1"
                  @click="removeFolder(row.folder)"
                >
                  Remover
                </button>
              </div>
            </div>
          </div>

          <article
            v-else
            class="request-item"
            :class="{ 'request-item-active': row.request.id === props.activeRequestId }"
            role="treeitem"
            :aria-level="row.depth + 2"
            :style="{ marginLeft: `${row.depth * 14}px` }"
          >
            <form v-if="editingRequestId === row.request.id" class="request-rename-form" @submit.prevent="confirmRename(row.request)">
              <input
                :ref="setRenameInput"
                v-model="renameDraft"
                :aria-label="`Novo nome para ${row.request.name}`"
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
              :aria-current="row.request.id === props.activeRequestId ? 'page' : undefined"
              :title="row.request.name"
              @click="selectRequest(row.request.id)"
            >
              <span class="method-badge" :class="methodClass(row.request)">{{ methodLabel(row.request) }}</span>
              <strong>{{ row.request.name }}</strong>
            </button>

            <div class="sidebar-popover request-item-actions">
              <button
                class="request-menu-trigger"
                type="button"
                :aria-label="`Ações para ${row.request.name}`"
                :aria-expanded="openMenuId === row.request.id"
                aria-haspopup="menu"
                :title="`Ações para ${row.request.name}`"
                @click.stop="toggleMenu(row.request.id)"
              >
                <svg viewBox="0 0 16 16" aria-hidden="true">
                  <circle cx="3" cy="8" r="1" />
                  <circle cx="8" cy="8" r="1" />
                  <circle cx="13" cy="8" r="1" />
                </svg>
              </button>

              <div v-if="openMenuId === row.request.id" class="request-menu" role="menu" :aria-label="`Ações da request ${row.request.name}`" @keydown.esc.prevent="openMenuId = null">
                <button type="button" role="menuitem" @click="startRename(row.request)">Renomear</button>
                <button type="button" role="menuitem" @click="duplicateRequest(row.request.id)">Duplicar</button>
                <button type="button" role="menuitem" @click="moveRequest(row.request.id)">Mover para pasta</button>
                <button type="button" role="menuitem" class="danger-action" :disabled="totalRequestCount <= 1" @click="removeRequest(row.request.id)">
                  Remover
                </button>
              </div>
            </div>
          </article>
        </template>

        <p v-if="treeRows.length === 0" class="empty-list">Nenhuma request nesta collection.</p>
      </div>
    </div>
  </section>
</template>

<style scoped>
.collection-sidebar-content {
  display: flex;
  min-width: 0;
  min-height: 0;
  flex: 1 1 auto;
  flex-direction: column;
  overflow: hidden;
  background: var(--color-surface-1);
}

.collection-sidebar-header {
  display: flex;
  flex: 0 0 40px;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  min-width: 0;
  padding: 0 12px;
  border-bottom: 1px solid var(--color-border);
}

.collection-title,
.sidebar-header-actions,
.tree-root,
.request-item-select,
.folder-toggle {
  display: flex;
  align-items: center;
}

.collection-title {
  min-width: 0;
  gap: 7px;
  color: var(--color-text);
  font-size: 12px;
  font-weight: 700;
}

.collection-title span,
.folder-toggle strong {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.collection-dirty-dot {
  flex: 0 0 auto;
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--color-warning);
}

.collection-icon,
.tree-folder {
  flex: 0 0 auto;
  width: 15px;
  height: 15px;
  fill: none;
  stroke: var(--color-text-muted);
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 1.1;
}

.sidebar-header-actions {
  flex: 0 0 auto;
  gap: 2px;
}

.icon-button,
.request-menu-trigger {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: 0;
  border-radius: 4px;
  color: var(--color-text-muted);
  background: transparent;
  cursor: pointer;
}

.icon-button:hover,
.request-menu-trigger:hover {
  color: var(--color-text);
  background: var(--color-surface-2);
}

.icon-button:focus-visible,
.request-menu-trigger:focus-visible,
.folder-toggle:focus-visible,
.request-item-select:focus-visible {
  outline: 2px solid var(--color-brand);
  outline-offset: 1px;
}

.icon-button svg,
.request-menu-trigger svg {
  width: 15px;
  height: 15px;
  fill: none;
  stroke: currentColor;
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 1.2;
}

.sidebar-popover {
  position: relative;
}

.collection-menu,
.request-menu {
  position: absolute;
  z-index: 10;
  border: 1px solid var(--color-border-strong);
  border-radius: 6px;
  padding: 5px;
  background: var(--color-surface-2);
  box-shadow: 0 14px 32px var(--color-bg);
}

.collection-menu {
  top: 30px;
  right: 0;
  min-width: 172px;
}

.request-menu {
  top: 30px;
  right: 0;
  min-width: 130px;
}

.collection-menu button,
.request-menu button {
  display: flex;
  align-items: center;
  width: 100%;
  min-height: 29px;
  gap: 8px;
  border: 0;
  border-radius: 4px;
  padding: 0 7px;
  color: var(--color-text);
  background: transparent;
  cursor: pointer;
  font: inherit;
  font-size: 12px;
  text-align: left;
}

.collection-menu button:hover,
.request-menu button:hover:not(:disabled) {
  color: var(--color-text);
  background: var(--color-surface-3);
}

.request-menu button:disabled {
  cursor: not-allowed;
  opacity: 0.45;
}

.method-badge {
  flex: 0 0 auto;
  min-width: 34px;
  color: var(--color-success);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 10px;
  font-weight: 800;
}

.danger-action {
  color: var(--color-danger) !important;
}

.collection-tree {
  min-height: 0;
  flex: 1 1 auto;
  overflow-y: auto;
  padding: 7px 6px 14px;
}

.tree-root {
  height: 28px;
  gap: 5px;
  padding: 0 5px;
  color: var(--color-text-muted);
  font-size: 11px;
  font-weight: 700;
}

.tree-root span {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tree-root small,
.folder-toggle small {
  margin-left: auto;
  color: var(--color-text-subtle);
  font-size: 10px;
}

.tree-chevron,
.folder-chevron {
  width: 12px;
  height: 12px;
  fill: none;
  stroke: var(--color-text-subtle);
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 1.2;
}

.request-list {
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.folder-item,
.request-item {
  position: relative;
  display: flex;
  align-items: center;
  min-width: 0;
  height: 30px;
  border-radius: 4px;
}

.folder-item:hover,
.request-item:hover,
.request-item:focus-within {
  background: var(--color-surface-2);
}

.folder-toggle {
  width: auto;
  height: 30px;
  min-width: 0;
  flex: 1 1 auto;
  gap: 5px;
  border: 0;
  padding: 0 7px 0 0;
  color: var(--color-text-muted);
  background: transparent;
  cursor: pointer;
  font: inherit;
  text-align: left;
}

.folder-toggle strong {
  color: var(--color-text);
  font-size: 12px;
  font-weight: 600;
}

.folder-chevron {
  transition: transform 120ms ease;
}

.folder-chevron-open {
  transform: rotate(90deg);
}

.request-item-active {
  background: var(--color-surface-3);
}

.request-item-select,
.request-rename-form {
  min-width: 0;
  flex: 1 1 auto;
}

.request-item-select {
  height: 30px;
  gap: 7px;
  border: 0;
  padding: 0 0 0 7px;
  overflow: hidden;
  color: var(--color-text);
  background: transparent;
  cursor: pointer;
  font: inherit;
  text-align: left;
}

.request-item-select strong {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 12px;
  font-weight: 600;
}

.request-item-actions {
  flex: 0 0 auto;
}

.folder-item-actions {
  flex: 0 0 auto;
  position: relative;
}

.request-menu-trigger {
  opacity: 0;
}

.request-item:hover .request-menu-trigger,
.request-item:focus-within .request-menu-trigger,
.request-item-active .request-menu-trigger,
.folder-item:hover .folder-menu-trigger,
.folder-item:focus-within .folder-menu-trigger,
.folder-menu-trigger[aria-expanded="true"] {
  opacity: 1;
}

.folder-menu {
  top: 30px;
  right: 0;
  min-width: 190px;
}

.request-rename-form input {
  width: 100%;
  height: 26px;
  border: 1px solid var(--color-brand);
  border-radius: 4px;
  padding: 0 7px;
  color: var(--color-text);
  background: var(--color-surface-2);
  font: inherit;
  font-size: 12px;
}

.empty-list {
  margin: 10px 7px;
  color: var(--color-text-subtle);
  font-size: 12px;
}
</style>
