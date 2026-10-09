<script setup lang="ts">
import { ref, watch } from "vue";

import AppModal from "../../components/ui/AppModal.vue";

const props = defineProps<{
  open: boolean;
  mode?: "create" | "rename";
  initialName?: string;
}>();

const emit = defineEmits<{
  (event: "close"): void;
  (event: "created", name: string): void;
}>();

const name = ref("Nova pasta");
const error = ref("");

const modalTitle = () => (props.mode === "rename" ? "Renomear pasta" : "Nova pasta");
const modalDescription = () =>
  props.mode === "rename"
    ? "Atualize o nome da pasta sem alterar as requests que ela contém."
    : "Organize requests relacionadas em uma pasta local da collection.";
const primaryLabel = () => (props.mode === "rename" ? "Salvar nome" : "Criar pasta");

watch(
  [() => props.open, () => props.mode, () => props.initialName],
  ([open]) => {
    if (open) {
      name.value = props.mode === "rename" ? props.initialName ?? "" : "Nova pasta";
      error.value = "";
    }
  },
);

function createFolder() {
  const trimmedName = name.value.trim();
  if (!trimmedName) {
    error.value = "Informe um nome para a pasta.";
    return;
  }

  emit("created", trimmedName);
  emit("close");
}
</script>

<template>
  <AppModal
    :open="props.open"
    :title="modalTitle()"
    :description="modalDescription()"
    :close-on-backdrop="false"
    size="new-request"
    :primary-label="primaryLabel()"
    secondary-label="Cancelar"
    @close="emit('close')"
    @confirm="createFolder"
  >
    <label class="folder-name-field">
      <span>Nome</span>
      <input
        v-model="name"
        type="text"
        autocomplete="off"
        spellcheck="false"
        :aria-label="props.mode === 'rename' ? 'Novo nome da pasta' : 'Nome da pasta'"
        @keydown.enter.prevent="createFolder"
      />
    </label>
    <p v-if="error" class="folder-error" role="alert">{{ error }}</p>
  </AppModal>
</template>

<style scoped>
.folder-name-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.folder-name-field span {
  color: var(--color-text-subtle);
  font-size: 11px;
  font-weight: 700;
}

.folder-name-field input {
  width: 100%;
  height: 32px;
  border: 1px solid var(--color-border);
  border-radius: 5px;
  padding: 0 10px;
  color: var(--color-text);
  background: var(--color-surface-2);
  font: inherit;
  font-size: 12px;
}

.folder-name-field input:focus-visible {
  border-color: var(--color-brand);
  outline: 2px solid var(--color-brand);
  outline-offset: 2px;
}

.folder-error {
  margin: 8px 0 0;
  color: var(--color-danger);
  font-size: 12px;
}
</style>
