<script setup lang="ts">
import { computed, ref } from "vue";

import AppModal from "../../components/ui/AppModal.vue";
import CollectionManager from "../collections/CollectionManager.vue";
import CollectionSidebar from "../collections/CollectionSidebar.vue";
import MoveRequestModal from "../collections/MoveRequestModal.vue";
import NewFolderModal from "../collections/NewFolderModal.vue";
import NewRequestModal from "../collections/NewRequestModal.vue";
import WebSocketPanel from "../websocket/WebSocketPanel.vue";
import SsePanel from "../sse/SsePanel.vue";
import GrpcPanel from "../grpc/GrpcPanel.vue";
import type {
  CollectionFolder,
  CollectionFile,
  RequestDefinition,
} from "../../types/api";
import type { NewRequestDraft } from "../../types/ui";

const props = defineProps<{
  requests: RequestDefinition[];
  folders: CollectionFolder[];
  collectionName: string;
  collectionDirty: boolean;
  activeRequestId: string;
}>();

const emit = defineEmits<{
  (event: "select-request", requestId: string): void;
  (event: "create-request", draft: NewRequestDraft, folderId: string | null): void;
  (event: "rename-request", requestId: string, name: string): void;
  (event: "duplicate-request", requestId: string): void;
  (event: "remove-request", requestId: string): void;
  (event: "loaded-collection", collection: CollectionFile, path: string): void;
  (event: "collection-saved", path: string): void;
  (event: "collection-name-change", name: string): void;
  (event: "import-curl"): void;
  (event: "create-folder", name: string, parentFolderId: string | null): void;
  (event: "rename-folder", folderId: string, name: string): void;
  (event: "remove-folder", folderId: string): void;
  (event: "move-request", requestId: string, folderId: string | null): void;
}>();

type LocalTool = "collection" | "websocket" | "sse" | "grpc";
const activeTool = ref<LocalTool | null>(null);
const isNewRequestOpen = ref(false);
const isNewFolderOpen = ref(false);
const isMoveRequestOpen = ref(false);
const moveRequestId = ref("");
const newRequestFolderId = ref<string | null>(null);
const folderModalMode = ref<"create" | "rename">("create");
const folderModalParentId = ref<string | null>(null);
const folderModalId = ref("");

function findRequest(requestId: string, folders: CollectionFolder[]): RequestDefinition | null {
  const rootRequest = props.requests.find((request) => request.id === requestId);
  if (rootRequest) {
    return rootRequest;
  }

  for (const folder of folders) {
    const folderRequest = folder.requests.find((request) => request.id === requestId);
    if (folderRequest) {
      return folderRequest;
    }

    const nestedRequest = findRequest(requestId, folder.folders);
    if (nestedRequest) {
      return nestedRequest;
    }
  }

  return null;
}

function findRequestFolderId(requestId: string, folders: CollectionFolder[]): string | null {
  if (props.requests.some((request) => request.id === requestId)) {
    return null;
  }

  for (const folder of folders) {
    if (folder.requests.some((request) => request.id === requestId)) {
      return folder.id;
    }

    const nestedFolderId = findRequestFolderId(requestId, folder.folders);
    if (nestedFolderId) {
      return nestedFolderId;
    }
  }

  return null;
}

function findFolder(folderId: string, folders: CollectionFolder[]): CollectionFolder | null {
  for (const folder of folders) {
    if (folder.id === folderId) {
      return folder;
    }

    const nested = findFolder(folderId, folder.folders);
    if (nested) {
      return nested;
    }
  }

  return null;
}

const moveRequestName = computed(() => findRequest(moveRequestId.value, props.folders)?.name ?? "request");
const currentRequestFolderId = computed(() => findRequestFolderId(moveRequestId.value, props.folders));
const folderModalName = computed(() => {
  if (folderModalMode.value !== "rename") {
    return "";
  }

  return findFolder(folderModalId.value, props.folders)?.name ?? "";
});

function openTool(tool: LocalTool) {
  activeTool.value = tool;
}

function closeTool() {
  activeTool.value = null;
}

function openNewRequest(folderId: string | null = null) {
  newRequestFolderId.value = folderId;
  isNewRequestOpen.value = true;
}

function closeNewRequest() {
  isNewRequestOpen.value = false;
  newRequestFolderId.value = null;
}

function handleNewRequest(draft: NewRequestDraft) {
  const folderId = newRequestFolderId.value;
  closeNewRequest();

  if (draft.protocol === "http") {
    emit("create-request", draft, folderId);
  } else if (draft.protocol === "websocket") {
    openTool("websocket");
  } else if (draft.protocol === "grpc") {
    openTool("grpc");
  } else if (draft.protocol === "sse") {
    openTool("sse");
  }
}

function handleCurlRequest() {
  closeNewRequest();
  emit("import-curl");
}

function openCreateFolder(parentFolderId: string | null = null) {
  folderModalMode.value = "create";
  folderModalParentId.value = parentFolderId;
  folderModalId.value = "";
  isNewFolderOpen.value = true;
}

function openRenameFolder(folderId: string) {
  folderModalMode.value = "rename";
  folderModalParentId.value = null;
  folderModalId.value = folderId;
  isNewFolderOpen.value = true;
}

function closeFolderModal() {
  isNewFolderOpen.value = false;
  folderModalMode.value = "create";
  folderModalParentId.value = null;
  folderModalId.value = "";
}

function handleFolderSaved(name: string) {
  if (folderModalMode.value === "rename" && folderModalId.value) {
    emit("rename-folder", folderModalId.value, name);
  } else {
    emit("create-folder", name, folderModalParentId.value);
  }

  closeFolderModal();
}

function openMoveRequest(requestId: string) {
  moveRequestId.value = requestId;
  isMoveRequestOpen.value = true;
}

function closeMoveRequest() {
  isMoveRequestOpen.value = false;
  moveRequestId.value = "";
}

function moveRequest(folderId: string | null) {
  if (moveRequestId.value) {
    emit("move-request", moveRequestId.value, folderId);
  }
  closeMoveRequest();
}
</script>

<template>
  <aside class="workspace-rail" aria-label="Árvore da collection">
    <CollectionSidebar
      :requests="props.requests"
      :folders="props.folders"
      :collection-name="props.collectionName"
      :collection-dirty="props.collectionDirty"
      :active-request-id="props.activeRequestId"
      @select-request="emit('select-request', $event)"
      @rename-request="(requestId, name) => emit('rename-request', requestId, name)"
      @duplicate-request="emit('duplicate-request', $event)"
      @remove-request="emit('remove-request', $event)"
      @move-request="openMoveRequest"
      @open-collection="openTool('collection')"
      @open-new-request="openNewRequest"
      @open-new-folder="openCreateFolder"
      @open-rename-folder="openRenameFolder"
      @remove-folder="emit('remove-folder', $event)"
    />
  </aside>

  <NewRequestModal
    :open="isNewRequestOpen"
    @close="closeNewRequest"
    @created="handleNewRequest"
    @import-curl="handleCurlRequest"
  />

  <NewFolderModal
    :open="isNewFolderOpen"
    :mode="folderModalMode"
    :initial-name="folderModalName"
    @close="closeFolderModal"
    @created="handleFolderSaved"
  />

  <MoveRequestModal
    :open="isMoveRequestOpen"
    :request-name="moveRequestName"
    :folders="props.folders"
    :current-folder-id="currentRequestFolderId"
    @close="closeMoveRequest"
    @move="moveRequest"
  />

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
      :folders="props.folders"
      :collection-name="props.collectionName"
      :dirty="props.collectionDirty"
      :embedded="true"
      @loaded-collection="(collection, path) => emit('loaded-collection', collection, path)"
      @collection-saved="emit('collection-saved', $event)"
      @collection-name-change="emit('collection-name-change', $event)"
    />
  </AppModal>

  <AppModal
    :open="activeTool === 'websocket'"
    :keep-mounted="true"
    size="wide"
    kicker="REALTIME"
    title="Nova request WebSocket"
    description="Conecte a um endpoint ws:// ou wss://, envie mensagens e acompanhe os eventos da sessão."
    variant="info"
    primary-label="Fechar painel"
    secondary-label=""
    @close="closeTool"
    @confirm="closeTool"
  >
    <WebSocketPanel :open="activeTool === 'websocket'" />
  </AppModal>

  <AppModal
    :open="activeTool === 'sse'"
    :keep-mounted="true"
    size="wide"
    kicker="SERVER EVENTS"
    title="Nova request SSE"
    description="Conecte a um endpoint HTTP de eventos e acompanhe frames enviados pelo servidor."
    variant="info"
    primary-label="Fechar painel"
    secondary-label=""
    @close="closeTool"
    @confirm="closeTool"
  >
    <SsePanel :open="activeTool === 'sse'" />
  </AppModal>

  <AppModal
    :open="activeTool === 'grpc'"
    :keep-mounted="true"
    size="wide"
    kicker="DYNAMIC GRPC"
    title="Nova request gRPC"
    description="Importe um arquivo .proto local, descubra os métodos e execute chamadas sem gerar código estático."
    variant="info"
    primary-label="Fechar painel"
    secondary-label=""
    @close="closeTool"
    @confirm="closeTool"
  >
    <GrpcPanel :open="activeTool === 'grpc'" />
  </AppModal>
</template>

<style scoped>
.workspace-rail {
  display: flex;
  min-width: 0;
  min-height: 0;
  flex-direction: column;
  overflow: hidden;
  background: var(--color-surface-1);
}
</style>
