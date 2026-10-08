<script setup lang="ts">
import { onMounted, ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";

defineProps<{
  isExecuting: boolean;
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
    <div class="titlebar-left">
      <button class="titlebar-icon-button" type="button" aria-label="Navegação" title="Navegação" @mousedown.stop>
        <span class="menu-glyph" aria-hidden="true"></span>
      </button>
      <button class="titlebar-icon-button" type="button" aria-label="Início" title="Início" @mousedown.stop>
        <svg class="home-glyph" viewBox="0 0 16 16" aria-hidden="true">
          <path d="m2.5 7.2 5.5-4.5 5.5 4.5v5.3a1 1 0 0 1-1 1H3.5a1 1 0 0 1-1-1V7.2Z" />
          <path d="M6 13.5V9.2h4v4.3" />
        </svg>
      </button>
      <span class="workspace-switcher" title="Workspace local">
        <span>My Workspace</span>
        <svg class="chevron-glyph" viewBox="0 0 16 16" aria-hidden="true">
          <path d="m4 6 4 4 4-4" />
        </svg>
      </span>
    </div>

    <div class="titlebar-brand">
      <img class="brand-mark" src="/assets/larry-platypus.png" alt="Larry" />
      <strong>Larry</strong>
    </div>

    <div class="titlebar-right" @mousedown.stop @dblclick.stop>
      <span class="titlebar-context">Workspace local</span>
      <span class="titlebar-divider" aria-hidden="true"></span>
      <span v-if="isExecuting" class="app-status">Executando</span>
      <span v-else class="app-status app-status-idle">Pronto</span>
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
  grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
  align-items: center;
  min-height: 48px;
  margin: 0;
  padding: 0 32px;
  border-bottom: 1px solid var(--color-border);
  color: var(--color-text-muted);
  user-select: none;
}

.titlebar-left,
.titlebar-brand,
.titlebar-right {
  display: flex;
  align-items: center;
  gap: 9px;
}

.titlebar-left {
  min-width: 0;
}

.titlebar-brand {
  justify-self: center;
  gap: 7px;
}

.titlebar-brand strong {
  color: var(--color-text);
  font-size: 14px;
  line-height: 1.1;
}

.titlebar-right {
  justify-content: flex-end;
  min-width: 0;
}

.titlebar-context {
  color: var(--color-text-muted);
  font-size: 12px;
}

.app-status {
  border-radius: 999px;
  padding: 5px 10px;
  color: var(--color-brand);
  background: #163238;
  font-size: 12px;
  font-weight: 700;
}

.app-status-idle {
  color: var(--color-text-muted);
  background: var(--color-surface-2);
}

.titlebar-divider {
  width: 1px;
  height: 16px;
  margin: 0 3px;
  background: var(--color-border);
}

.titlebar-icon-button,
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

.titlebar-icon-button:hover,
.window-button:hover {
  color: var(--color-text);
  background: var(--color-surface-2);
}

.window-button-close:hover {
  color: #fff;
  background: #b63f4c;
}

.menu-glyph {
  position: relative;
  width: 14px;
  height: 10px;
  border-top: 1px solid currentColor;
  border-bottom: 1px solid currentColor;
}

.menu-glyph::after {
  position: absolute;
  top: 4px;
  right: 0;
  left: 0;
  height: 1px;
  background: currentColor;
  content: "";
}

.home-glyph,
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
  width: 30px;
  height: 30px;
  border-radius: 7px;
  object-fit: cover;
}

.workspace-switcher {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
  color: var(--color-text);
  font-size: 13px;
  font-weight: 600;
}

.chevron-glyph {
  width: 14px;
  height: 14px;
  color: var(--color-text-subtle);
  fill: none;
  stroke: currentColor;
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 1.2;
}

@media (max-width: 720px) {
  .titlebar {
    grid-template-columns: minmax(0, 1fr) auto;
    padding-right: 20px;
    padding-left: 20px;
  }

  .titlebar-brand {
    display: none;
  }

  .titlebar-context,
  .titlebar-divider {
    display: none;
  }
}

@media (max-width: 440px) {
  .titlebar {
    padding-right: 12px;
    padding-left: 12px;
  }

  .workspace-switcher {
    max-width: 140px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
}
</style>
