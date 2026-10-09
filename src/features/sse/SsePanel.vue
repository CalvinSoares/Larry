<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import { formatIpcError } from "../../services/errors";
import { closeSse, openSse } from "../../services/ipc";
import type { SseEvent, SseSession } from "../../types/api";

const props = defineProps<{
  open: boolean;
}>();

const url = ref("http://localhost:3000/events");
const session = ref<SseSession | null>(null);
const events = ref<SseEvent[]>([]);
const error = ref("");
const isConnecting = ref(false);
let unlisten: UnlistenFn | null = null;

const MAX_VISIBLE_EVENTS = 200;

function appendEvent(event: SseEvent) {
  if (session.value && event.sessionId !== session.value.sessionId) {
    return;
  }

  events.value = [...events.value, event].slice(-MAX_VISIBLE_EVENTS);

  if (event.eventType === "closed" || event.eventType === "error") {
    session.value = null;
  }
}

async function connect() {
  error.value = "";
  const normalizedUrl = url.value.trim();

  if (!normalizedUrl) {
    error.value = "Informe uma URL http:// ou https://.";
    return;
  }

  isConnecting.value = true;

  try {
    const opened = await openSse({ url: normalizedUrl, headers: [] });
    session.value = opened;
    events.value = [];
  } catch (value) {
    error.value = formatIpcError(value);
  } finally {
    isConnecting.value = false;
  }
}

async function disconnect() {
  if (!session.value) {
    return;
  }

  const sessionId = session.value.sessionId;
  error.value = "";

  try {
    await closeSse(sessionId);
  } catch (value) {
    error.value = formatIpcError(value);
  }
}

function clearEvents() {
  events.value = [];
}

function eventLabel(event: SseEvent) {
  if (event.eventType === "opened") {
    return "Conexão aberta";
  }

  if (event.eventType === "closed") {
    return "Conexão fechada";
  }

  if (event.eventType === "error") {
    return "Erro";
  }

  return event.eventName || "message";
}

function eventPayload(event: SseEvent) {
  if (event.data) {
    return event.data;
  }

  return event.technical ?? "";
}

async function startListening() {
  if (unlisten) {
    return;
  }

  unlisten = await listen<SseEvent>("sse_event", (event) => {
    appendEvent(event.payload);
  });
}

watch(
  () => props.open,
  (open) => {
    if (open) {
      void startListening();
    }
  },
);

onMounted(() => {
  if (props.open) {
    void startListening();
  }
});

onBeforeUnmount(async () => {
  unlisten?.();
  unlisten = null;

  if (session.value) {
    await closeSse(session.value.sessionId).catch(() => undefined);
  }
});
</script>

<template>
  <div class="sse-panel">
    <div class="connection-row">
      <input
        v-model="url"
        class="url-input"
        type="text"
        spellcheck="false"
        placeholder="http://localhost:3000/events"
        :disabled="Boolean(session) || isConnecting"
        aria-label="URL SSE"
      />
      <button v-if="!session" class="action-button primary" type="button" :disabled="isConnecting" @click="connect">
        {{ isConnecting ? "Conectando..." : "Conectar" }}
      </button>
      <button v-else class="action-button" type="button" @click="disconnect">Fechar</button>
    </div>

    <p v-if="error" class="panel-error" aria-live="assertive">{{ error }}</p>

    <div class="panel-status">
      <span class="status-dot" :class="{ connected: Boolean(session) }" aria-hidden="true"></span>
      <span>{{ session ? "Conectado" : "Desconectado" }}</span>
      <span v-if="session?.contentType" class="status-content-type">{{ session.contentType }}</span>
    </div>

    <div class="events-header">
      <div>
        <span class="section-label">Eventos</span>
        <span class="event-count">{{ events.length }}</span>
      </div>
      <button class="text-button" type="button" :disabled="events.length === 0" @click="clearEvents">Limpar</button>
    </div>

    <div class="event-list" aria-live="polite">
      <p v-if="events.length === 0" class="empty-state">Aguardando conexão e eventos SSE.</p>
      <article v-for="(event, index) in events" :key="`${event.timestampMs}-${index}`" class="event-entry" :class="`event-${event.eventType}`">
        <div class="event-meta">
          <div>
            <strong>{{ eventLabel(event) }}</strong>
            <span v-if="event.eventId">id {{ event.eventId }}</span>
          </div>
          <span>{{ event.sizeBytes }} bytes</span>
        </div>
        <pre>{{ eventPayload(event) }}</pre>
      </article>
    </div>
  </div>
</template>

<style scoped>
.sse-panel {
  display: flex;
  min-height: 360px;
  flex-direction: column;
  gap: 12px;
}

.connection-row,
.panel-status,
.events-header,
.event-meta {
  display: flex;
  align-items: center;
}

.connection-row {
  gap: 8px;
}

.url-input {
  flex: 1 1 auto;
  min-width: 0;
  min-height: 34px;
  border: 1px solid var(--color-border-strong);
  border-radius: 6px;
  padding: 8px 10px;
  color: var(--color-text);
  background: var(--color-code-bg);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 12px;
}

.action-button,
.text-button {
  border: 1px solid var(--color-border-strong);
  border-radius: 5px;
  padding: 7px 11px;
  color: var(--color-text);
  background: var(--color-surface-2);
  cursor: pointer;
  font-size: 12px;
  font-weight: 700;
  white-space: nowrap;
}

.action-button:hover:not(:disabled),
.text-button:hover:not(:disabled) {
  border-color: var(--color-brand);
  color: var(--color-brand);
}

.action-button.primary {
  border-color: var(--color-brand);
  color: #071113;
  background: var(--color-brand);
}

.action-button.primary:hover:not(:disabled) {
  color: #071113;
  background: var(--color-brand-strong);
}

.action-button:disabled,
.text-button:disabled,
.url-input:disabled {
  cursor: not-allowed;
  opacity: 0.55;
}

.panel-error {
  margin: 0;
  color: var(--color-danger);
  font-size: 12px;
  white-space: pre-wrap;
}

.panel-status {
  gap: 7px;
  color: var(--color-text-muted);
  font-size: 11px;
}

.status-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--color-text-subtle);
}

.status-dot.connected {
  background: var(--color-success);
}

.status-content-type {
  margin-left: auto;
  color: var(--color-text-subtle);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
}

.events-header {
  justify-content: space-between;
  border-top: 1px solid var(--color-border);
  padding-top: 12px;
}

.events-header > div {
  display: flex;
  align-items: center;
  gap: 7px;
}

.section-label {
  color: var(--color-text);
  font-size: 12px;
  font-weight: 700;
}

.event-count {
  border: 1px solid var(--color-border);
  border-radius: 999px;
  padding: 2px 6px;
  color: var(--color-text-subtle);
  font-size: 11px;
}

.text-button {
  border-color: transparent;
  padding: 4px 6px;
  color: var(--color-text-muted);
  background: transparent;
  font-weight: 600;
}

.event-list {
  min-height: 150px;
  overflow: auto;
  border: 1px solid var(--color-border);
  border-radius: 6px;
  background: var(--color-code-bg);
}

.empty-state {
  margin: 0;
  padding: 28px 16px;
  color: var(--color-text-subtle);
  font-size: 12px;
  text-align: center;
}

.event-entry {
  border-bottom: 1px solid var(--color-border);
  padding: 8px 10px;
}

.event-entry:last-child {
  border-bottom: 0;
}

.event-meta {
  justify-content: space-between;
  gap: 10px;
  margin-bottom: 5px;
  color: var(--color-text-muted);
  font-size: 11px;
}

.event-meta div {
  display: flex;
  gap: 8px;
}

.event-meta strong {
  color: var(--color-text);
}

.event-error .event-meta strong {
  color: var(--color-danger);
}

.event-entry pre {
  max-height: 180px;
  margin: 0;
  padding: 0;
  overflow: auto;
  color: var(--color-text-muted);
  background: transparent;
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 11px;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}

@media (max-width: 520px) {
  .connection-row {
    align-items: stretch;
    flex-direction: column;
  }

  .status-content-type {
    display: none;
  }
}
</style>
