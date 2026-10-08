<script setup lang="ts">
import { ref } from "vue";

import { getGitSnapshot } from "../../services/ipc";
import { formatIpcError } from "../../services/errors";
import type { GitSnapshot } from "../../types/api";

const props = defineProps<{
  path: string;
}>();

const snapshot = ref<GitSnapshot | null>(null);
const isLoading = ref(false);
const error = ref("");

async function inspectGit() {
  error.value = "";
  snapshot.value = null;

  if (!props.path.trim()) {
    error.value = "Salve ou informe o caminho da collection antes de consultar o Git.";
    return;
  }

  isLoading.value = true;

  try {
    snapshot.value = await getGitSnapshot(props.path);
  } catch (gitError) {
    error.value = formatIpcError(gitError);
  } finally {
    isLoading.value = false;
  }
}

</script>

<template>
  <section class="git-panel">
    <div class="section-heading">
      <div>
        <p class="eyebrow">GIT · READ ONLY</p>
        <h3>Revisão da collection</h3>
        <p>Status e diff local, sem commit ou push automático.</p>
      </div>

      <button type="button" :disabled="isLoading" @click="inspectGit">
        {{ isLoading ? "Consultando..." : "Consultar Git" }}
      </button>
    </div>

    <p v-if="error" class="git-error" aria-live="assertive">{{ error }}</p>

    <div v-if="snapshot" class="git-content">
      <dl class="git-meta">
        <dt>Branch</dt>
        <dd>{{ snapshot.branch }}</dd>

        <dt>Repositório</dt>
        <dd>{{ snapshot.repositoryRoot }}</dd>
      </dl>

      <h4>Status</h4>
      <pre>{{ snapshot.status || "Working tree limpo." }}</pre>

      <h4>Diff da collection</h4>
      <pre>{{ snapshot.diff || "Nenhuma diferença rastreada para este arquivo." }}</pre>
    </div>
  </section>
</template>

<style scoped>
.git-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin-top: 20px;
  border-top: 1px solid var(--color-border);
  padding-top: 18px;
  text-align: left;
}

.section-heading {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}

.section-heading h3 {
  margin: 0;
}

.section-heading p:not(.eyebrow) {
  margin: 4px 0 0;
  color: var(--color-text-muted);
  font-size: 13px;
}

.eyebrow {
  margin: 0 0 4px;
  color: var(--color-text-subtle);
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.08em;
}

button {
  border: 1px solid var(--color-border-strong);
  border-radius: 6px;
  padding: 9px 12px;
  font: inherit;
  cursor: pointer;
  background: var(--color-surface-2);
  color: var(--color-text);
  white-space: nowrap;
}

button:hover:not(:disabled) {
  border-color: var(--color-brand);
  background: var(--color-surface-3);
}

button:disabled {
  cursor: wait;
  opacity: 0.65;
}

.git-error {
  margin: 0;
  color: var(--color-danger);
  white-space: pre-wrap;
}

.git-content {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.git-meta {
  display: grid;
  grid-template-columns: minmax(100px, 0.3fr) minmax(0, 1fr);
  gap: 6px 12px;
  margin: 0;
  font-size: 13px;
}

.git-meta dt {
  font-weight: 600;
  color: var(--color-text);
}

.git-meta dd {
  margin: 0;
  color: var(--color-text-muted);
  overflow-wrap: anywhere;
}

h4 {
  margin: 10px 0 0;
}

pre {
  max-height: 220px;
  margin: 0;
  padding: 12px;
  overflow: auto;
  border-radius: 6px;
  background: var(--color-code-bg);
  color: var(--color-text);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 12px;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}

@media (max-width: 520px) {
  .section-heading {
    flex-direction: column;
  }

  button {
    width: 100%;
  }
}
</style>
