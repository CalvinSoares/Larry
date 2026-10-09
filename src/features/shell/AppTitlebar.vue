<script setup lang="ts">
import { onMounted, ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";

defineProps<{
  environmentName: string;
  requestName: string;
}>();

const emit = defineEmits<{
  (event: "open-environment"): void;
}>();

type TauriWindow = ReturnType<typeof getCurrentWindow>;

let appWindow: TauriWindow | null = null;
const isMaximized = ref(false);

function getTauriWindow() {
  const tauriWindow = window as Window & { __TAURI_INTERNALS__?: unknown };

  if (!tauriWindow.__TAURI_INTERNALS__) {
    return null;
  }

  appWindow ??= getCurrentWindow();
  return appWindow;
}

async function refreshMaximizedState() {
  const currentWindow = getTauriWindow();

  if (!currentWindow) {
    return;
  }

  try {
    isMaximized.value = await currentWindow.isMaximized();
  } catch {
    // O preview Vite não possui uma janela Tauri real.
  }
}

async function startDragging() {
  const currentWindow = getTauriWindow();

  if (!currentWindow) {
    return;
  }

  try {
    await currentWindow.startDragging();
  } catch {
    // Arrastar só existe quando a aplicação está dentro do Tauri.
  }
}

async function minimizeWindow() {
  const currentWindow = getTauriWindow();

  if (currentWindow) {
    await currentWindow.minimize();
  }
}

async function toggleMaximize() {
  const currentWindow = getTauriWindow();

  if (!currentWindow) {
    return;
  }

  await currentWindow.toggleMaximize();
  await refreshMaximizedState();
}

async function closeWindow() {
  const currentWindow = getTauriWindow();

  if (currentWindow) {
    await currentWindow.close();
  }
}

onMounted(refreshMaximizedState);
</script>

<template>
  <header
    class="titlebar"
    aria-label="Barra de título do Larry"
    @dblclick="toggleMaximize"
    @mousedown="startDragging"
  >
    <div class="titlebar-brand">
      <img class="brand-mark" src="/assets/larry-platypus.png" alt="" />
      <div>
        <strong>Larry</strong>
        <span>Local workbench</span>
      </div>
    </div>

    <div class="titlebar-route" :title="`Minha API / ${requestName}`">
      <span>Minha API</span>
      <span class="route-separator" aria-hidden="true">/</span>
      <strong>{{ requestName }}</strong>
    </div>

    <div class="titlebar-right" @mousedown.stop @dblclick.stop>
      <button
        class="environment-trigger"
        type="button"
        :title="environmentName ? `Editar environment ${environmentName}` : 'Abrir environments'"
        @click="emit('open-environment')"
      >
        <span>{{ environmentName || "Sem environment" }}</span>
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path d="m4 6 4 4 4-4" />
        </svg>
      </button>
      <button class="window-button" type="button" aria-label="Minimizar janela" title="Minimizar" @click="minimizeWindow">
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3 8h10" /></svg>
      </button>
      <button class="window-button" type="button" :aria-label="isMaximized ? 'Restaurar janela' : 'Maximizar janela'" :title="isMaximized ? 'Restaurar' : 'Maximizar'" @click="toggleMaximize">
        <svg v-if="!isMaximized" viewBox="0 0 16 16" aria-hidden="true"><rect x="3.5" y="3.5" width="9" height="9" rx="0.5" /></svg>
        <svg v-else viewBox="0 0 16 16" aria-hidden="true"><path d="M5 5h7.5v7.5H5z" /><path d="M3.5 10.5V3.5h7" /></svg>
      </button>
      <button class="window-button window-button-close" type="button" aria-label="Fechar janela" title="Fechar" @click="closeWindow">
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="m4 4 8 8M12 4l-8 8" /></svg>
      </button>
    </div>
  </header>
</template>

<style scoped>
.titlebar {
  display: grid;
  grid-template-columns: minmax(176px, 0.7fr) minmax(0, 1fr) minmax(176px, 0.7fr);
  align-items: center;
  min-height: 48px;
  margin: 0;
  padding: 0 20px;
  border-bottom: 1px solid var(--color-border);
  color: var(--color-text-muted);
  user-select: none;
}

.titlebar-brand,
.titlebar-right {
  display: flex;
  align-items: center;
  gap: 9px;
}

.titlebar-brand {
  gap: 8px;
  min-width: 0;
}

.titlebar-brand strong,
.titlebar-route strong {
  display: block;
  color: var(--color-text);
  font-size: 14px;
  line-height: 1.1;
}

.titlebar-brand span {
  display: block;
  margin-top: 2px;
  color: var(--color-text-subtle);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.titlebar-route {
  display: flex;
  justify-self: center;
  min-width: 0;
  overflow: hidden;
  color: var(--color-text-muted);
  font-size: 12px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.titlebar-route strong {
  overflow: hidden;
  color: var(--color-text);
  font-size: 12px;
  line-height: inherit;
  text-overflow: ellipsis;
}

.route-separator {
  margin: 0 8px;
  color: var(--color-text-subtle);
}

.titlebar-right {
  justify-content: flex-end;
  min-width: 0;
}

.titlebar-context {
  color: var(--color-text-muted);
  font-size: 12px;
}

.environment-trigger {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  min-width: 0;
  min-height: 30px;
  border: 0;
  border-radius: 5px;
  padding: 0 7px;
  color: var(--color-text-muted);
  background: transparent;
  cursor: pointer;
  font-size: 12px;
}

.environment-trigger:hover {
  color: var(--color-text);
  background: transparent;
}

.environment-trigger span {
  max-width: 150px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.environment-trigger svg {
  width: 13px;
  height: 13px;
  fill: none;
  stroke: currentColor;
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 1.2;
}

.window-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  border: 0;
  border-radius: 5px;
  color: var(--color-text-muted);
  background: transparent;
  cursor: pointer;
}

.window-button:hover {
  color: var(--color-text-muted);
  background: transparent;
}

.window-button-close:hover {
  color: #fff;
  background: #b63f4c;
}

.window-button svg {
  width: 16px;
  height: 16px;
  fill: none;
  stroke: currentColor;
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 1.2;
}

.brand-mark {
  width: 28px;
  height: 28px;
  border-radius: 6px;
  object-fit: cover;
}

@media (max-width: 720px) {
  .titlebar {
    grid-template-columns: minmax(0, 1fr) auto;
    padding: 0 14px;
  }

  .titlebar-route {
    display: none;
  }

  .environment-trigger {
    display: none;
  }
}

@media (max-width: 440px) {
  .titlebar {
    padding: 0 10px;
  }
}
</style>
