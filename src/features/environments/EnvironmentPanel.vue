<script setup lang="ts">
import { ref, watch } from "vue";

import {
  loadEnvironment,
  saveEnvironment,
  setEnvironmentSecret,
} from "../../services/ipc";
import { formatIpcError } from "../../services/errors";
import type { EnvironmentFile, EnvironmentVariable } from "../../types/api";

const CURRENT_SCHEMA_VERSION = 1;

const props = withDefaults(
  defineProps<{
    embedded?: boolean;
  }>(),
  {
    embedded: false,
  },
);

const emit = defineEmits<{
  (event: "environment-changed", environment: EnvironmentFile | null): void;
}>();

const path = ref("");
const environmentName = ref("Local");
const variables = ref<EnvironmentVariable[]>([]);
const secretDrafts = ref<Record<string, string>>({});
const isSaving = ref(false);
const isLoading = ref(false);
const message = ref("");
const error = ref("");

function cloneEnvironment(environment: EnvironmentFile): EnvironmentFile {
  return JSON.parse(JSON.stringify(environment)) as EnvironmentFile;
}

function clearFeedback() {
  message.value = "";
  error.value = "";
}

function currentEnvironment(): EnvironmentFile {
  return {
    schemaVersion: CURRENT_SCHEMA_VERSION,
    name: environmentName.value.trim() || "Local",
    variables: variables.value.map((variable) => ({
      name: variable.name.trim(),
      value: variable.secretRef ? null : variable.value ?? "",
      secretRef: variable.secretRef,
    })),
  };
}

function emitCurrentEnvironment() {
  emit("environment-changed", cloneEnvironment(currentEnvironment()));
}

function addPublicVariable() {
  variables.value.push({
    name: `variable${variables.value.length + 1}`,
    value: "",
    secretRef: null,
  });
}

function addSecretVariable() {
  const name = `secret${variables.value.length + 1}`;
  variables.value.push({
    name,
    value: null,
    secretRef: name,
  });
  secretDrafts.value[name] = "";
}

function removeVariable(index: number) {
  const variable = variables.value[index];
  variables.value.splice(index, 1);

  if (variable?.secretRef) {
    message.value = "A referência foi removida do arquivo, mas o secret permanece no Credential Manager até ser apagado explicitamente.";
  }
}

async function handleSave() {
  clearFeedback();

  if (!path.value.trim()) {
    error.value = "Informe o caminho de um arquivo .yaml para o environment.";
    return;
  }

  const environment = currentEnvironment();
  if (environment.variables.some((variable) => !variable.name)) {
    error.value = "Toda variável precisa de um nome.";
    return;
  }

  isSaving.value = true;

  try {
    for (const variable of environment.variables) {
      if (!variable.secretRef) {
        continue;
      }

      const draft = secretDrafts.value[variable.secretRef]?.trim();
      if (draft) {
        await setEnvironmentSecret(variable.secretRef, draft);
      }
    }

    await saveEnvironment(path.value, environment);
    emit("environment-changed", cloneEnvironment(environment));
    message.value = "Environment salvo sem colocar valores secretos no arquivo.";
    secretDrafts.value = {};
  } catch (value) {
    error.value = formatIpcError(value);
  } finally {
    isSaving.value = false;
  }
}

async function handleLoad() {
  clearFeedback();

  if (!path.value.trim()) {
    error.value = "Informe o caminho do arquivo .yaml do environment.";
    return;
  }

  isLoading.value = true;

  try {
    const environment = await loadEnvironment(path.value);
    environmentName.value = environment.name;
    variables.value = environment.variables.map((variable) => ({ ...variable }));
    secretDrafts.value = {};
    emit("environment-changed", cloneEnvironment(environment));
    message.value = `Environment "${environment.name}" carregado sem ler secrets para a UI.`;
  } catch (value) {
    error.value = formatIpcError(value);
  } finally {
    isLoading.value = false;
  }
}

watch(
  [environmentName, variables],
  () => {
    emitCurrentEnvironment();
  },
  { deep: true },
);
</script>

<template>
  <section class="environment-panel">
    <div v-if="!props.embedded" class="section-heading">
      <div>
        <p class="eyebrow">ENVIRONMENT</p>
        <h3>{{ environmentName }}</h3>
        <p>Valores públicos no YAML. Secrets ficam no Credential Manager.</p>
      </div>
    </div>

    <label>
      <span>Nome</span>
      <input v-model="environmentName" type="text" />
    </label>

    <label>
      <span>Caminho do arquivo</span>
      <input
        v-model="path"
        type="text"
        placeholder="C:\\projetos\\minha-api\\local.environment.yaml"
      />
    </label>

    <div class="variable-heading">
      <span>Variáveis</span>
      <div class="variable-actions">
        <button type="button" @click="addPublicVariable">Pública</button>
        <button type="button" @click="addSecretVariable">Secret</button>
      </div>
    </div>

    <div v-if="variables.length" class="variable-list">
      <div v-for="(variable, index) in variables" :key="variable.name + '-' + index" class="variable-row">
        <div class="variable-row-heading">
          <span class="variable-kind">{{ variable.secretRef ? "SECRET" : "PÚBLICA" }}</span>
          <button type="button" class="remove-button" @click="removeVariable(index)">Remover</button>
        </div>

        <input v-model="variable.name" type="text" placeholder="Nome da variável" />

        <input
          v-if="variable.secretRef"
          v-model="secretDrafts[variable.secretRef]"
          type="password"
          placeholder="Digite para substituir no Credential Manager"
        />
        <input
          v-else
          v-model="variable.value"
          type="text"
          placeholder="Valor público"
        />

        <small v-if="variable.secretRef">
          Referência: {{ variable.secretRef }}. O valor existente nunca é carregado para a interface.
        </small>
      </div>
    </div>

    <p v-else class="empty-state">
      Nenhuma variável. A request ainda pode ser executada sem environment.
    </p>

    <div class="collection-actions">
      <button type="button" :disabled="isSaving" @click="handleSave">
        {{ isSaving ? "Salvando..." : "Salvar environment" }}
      </button>
      <button type="button" :disabled="isLoading" @click="handleLoad">
        {{ isLoading ? "Carregando..." : "Carregar environment" }}
      </button>
    </div>

    <p v-if="message" class="feedback-success" aria-live="polite">{{ message }}</p>
    <p v-if="error" class="feedback-error" aria-live="assertive">{{ error }}</p>
  </section>
</template>

<style scoped>
.environment-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
  border-bottom: 1px solid var(--color-border);
  padding: 16px;
  text-align: left;
}

.section-heading h3 {
  margin: 0;
  color: var(--color-text);
}

.section-heading p:not(.eyebrow) {
  margin: 4px 0 0;
  color: var(--color-text-muted);
  font-size: 12px;
  line-height: 1.4;
}

.eyebrow {
  margin: 0 0 4px;
  color: var(--color-text-subtle);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.08em;
}

label {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

label span,
.variable-heading > span {
  color: var(--color-text);
  font-size: 12px;
  font-weight: 600;
}

input {
  width: 100%;
  min-height: 32px;
  border: 1px solid var(--color-border-strong);
  border-radius: 6px;
  padding: 6px 9px;
  color: var(--color-text);
  background: var(--color-surface-2);
}

input::placeholder {
  color: var(--color-text-subtle);
}

.variable-heading,
.variable-row-heading,
.collection-actions,
.variable-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.variable-heading {
  border-top: 1px solid var(--color-border);
  padding-top: 12px;
}

.variable-actions {
  justify-content: flex-end;
}

button {
  min-height: 30px;
  border: 1px solid var(--color-border-strong);
  border-radius: 5px;
  padding: 5px 9px;
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
  cursor: wait;
  opacity: 0.6;
}

.variable-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.variable-row {
  display: flex;
  flex-direction: column;
  gap: 6px;
  border: 1px solid var(--color-border);
  border-radius: 6px;
  padding: 9px;
  background: var(--color-surface-2);
}

.variable-row-heading {
  justify-content: space-between;
}

.variable-kind {
  color: var(--color-text-subtle);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.06em;
}

.remove-button {
  min-height: 24px;
  color: var(--color-danger);
  font-size: 10px;
}

.variable-row small,
.empty-state,
.feedback-success,
.feedback-error {
  margin: 0;
  color: var(--color-text-muted);
  font-size: 11px;
  line-height: 1.4;
}

.empty-state {
  border: 1px dashed var(--color-border-strong);
  padding: 10px;
}

.feedback-success {
  color: var(--color-success);
}

.feedback-error {
  color: var(--color-danger);
  white-space: pre-wrap;
}

@media (max-width: 520px) {
  .variable-heading,
  .collection-actions {
    align-items: stretch;
    flex-direction: column;
  }

  .variable-actions,
  .collection-actions button {
    width: 100%;
  }
}
</style>
