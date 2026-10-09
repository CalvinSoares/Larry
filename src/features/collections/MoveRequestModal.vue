<script setup lang="ts">
import { computed, ref, watch } from "vue";

import AppModal from "../../components/ui/AppModal.vue";
import CustomSelect from "../../components/ui/CustomSelect.vue";
import type { CollectionFolder } from "../../types/api";
import type { CustomSelectOption } from "../../types/ui";

const props = defineProps<{
  open: boolean;
  requestName: string;
  folders: CollectionFolder[];
  currentFolderId: string | null;
}>();

const emit = defineEmits<{
  (event: "close"): void;
  (event: "move", folderId: string | null): void;
}>();

const selectedFolderId = ref("");

function flattenFolderOptions(folders: CollectionFolder[], parentPath = ""): CustomSelectOption[] {
  return folders.flatMap((folder) => {
    const path = parentPath ? `${parentPath} / ${folder.name}` : folder.name;
    return [
      { value: folder.id, label: path },
      ...flattenFolderOptions(folder.folders, path),
    ];
  });
}

const folderOptions = computed<CustomSelectOption[]>(() => [
  { value: "", label: "Raiz da collection" },
  ...flattenFolderOptions(props.folders),
]);

const description = computed(() => `Escolha a pasta de destino para "${props.requestName}".`);

watch(
  () => [props.open, props.currentFolderId] as const,
  ([open, currentFolderId]) => {
    if (open) {
      selectedFolderId.value = currentFolderId ?? "";
    }
  },
  { immediate: true },
);

function confirmMove() {
  emit("move", selectedFolderId.value || null);
  emit("close");
}
</script>

<template>
  <AppModal
    :open="props.open"
    title="Mover request"
    :description="description"
    :close-on-backdrop="false"
    size="new-request"
    primary-label="Mover request"
    secondary-label="Cancelar"
    @close="emit('close')"
    @confirm="confirmMove"
  >
    <label class="destination-field">
      <span>Pasta de destino</span>
      <CustomSelect
        v-model="selectedFolderId"
        :options="folderOptions"
        label="Pasta de destino"
      />
    </label>
  </AppModal>
</template>

<style scoped>
.destination-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.destination-field span {
  color: var(--color-text-subtle);
  font-size: 11px;
  font-weight: 700;
}
</style>
