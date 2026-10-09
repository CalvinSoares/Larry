<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import { formatIpcError } from "../../services/errors";
import {
  closeWebSocket,
  openWebSocket,
  sendWebSocketMessage,
} from "../../services/ipc";
import type { WebSocketEvent, WebSocketSession } from "../../types/api";

const props = defineProps<{
  open: boolean;
}>();

const url = ref("ws://localhost:8080");
const message = ref("");
const session = ref<WebSocketSession | null>(null);
const events = ref<WebSocketEvent[]>([]);
const error = ref("");
const isConnecting = ref(false);
const isSending = ref(false);
let unlisten: UnlistenFn | null = null;

const MAX_VISIBLE_EVENTS = 200;

function appendEvent(event: WebSocketEvent) {
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
    error.value = "Informe uma URL ws:// ou wss://.";
    return;
  }

  isConnecting.value = true;

  try {
    const opened = await openWebSocket({ url: normalizedUrl, headers: [] });
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
    await closeWebSocket(sessionId);
  } catch (value) {
    error.value = formatIpcError(value);
  }
}

async function send() {
  if (!session.value || !message.value) {
    return;
  }

  error.value = "";
  isSending.value = true;

  try {
    await sendWebSocketMessage(session.value.sessionId, message.value);
    message.value = "";
  } catch (value) {
    error.value = formatIpcError(value);
  } finally {
    isSending.value = false;
  }
}

function clearEvents() {
  events.value = [];
}

function eventLabel(event: WebSocketEvent) {
  if (event.eventType === "opened") {
    return "Conexão aberta";
  }

  if (event.eventType === "closed") {
    return "Conexão fechada";
  }

  if (event.eventType === "error") {
    return "Erro";
  }

  return event.direction === "outgoing" ? "Enviado" : "Recebido";
}

function eventPayload(event: WebSocketEvent) {
  if (event.binary) {
    return `[binário: ${event.sizeBytes} bytes]`;
  }

  return event.payload ?? event.technical ?? "";
}

async function startListening() {
  if (unlisten) {
    return;
  }

  unlisten = await listen<WebSocketEvent>("websocket_event", (event) => {
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
    await closeWebSocket(session.value.sessionId).catch(() => undefined);
  }
});
</script>

<template>
  <div class="websocket-panel">
    <div class="connection-row">
      <input
        v-model="url"
        class="url-input"
        type="text"
        spellcheck="false"
        placeholder="ws://localhost:8080/socket"
        :disabled="Boolean(session) || isConnecting"
        aria-label="URL WebSocket"
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
      <span v-if="session" class="status-session">{{ session.sessionId }}</span>
    </div>

    <div class="message-composer">
      <textarea
        v-model="message"
        rows="3"
        spellcheck="false"
        placeholder="Mensagem de texto"
        :disabled="!session || isSending"
        aria-label="Mensagem WebSocket"
        @keydown.ctrl.enter.prevent="send"
      />
      <div class="composer-actions">
        <span>Ctrl + Enter para enviar</span>
        <button class="action-button primary" type="button" :disabled="!session || !message || isSending" @click="send">
          {{ isSending ? "Enviando..." : "Enviar" }}
        </button>
      </div>
    </div>

    <div class="events-header">
      <div>
        <span class="section-label">Mensagens</span>
        <span class="event-count">{{ events.length }}</span>
      </div>
      <button class="text-button" type="button" :disabled="events.length === 0" @click="clearEvents">Limpar</button>
    </div>

    <div class="event-list" aria-live="polite">
      <p v-if="events.length === 0" class="empty-state">Aguardando conexão e mensagens.</p>
      <article v-for="(event, index) in events" :key="`${event.timestampMs}-${index}`" class="event-entry" :class="`event-${event.eventType}`">
        <div class="event-meta">
          <strong>{{ eventLabel(event) }}</strong>
          <span>{{ event.sizeBytes }} bytes</span>
        </div>
        <pre>{{ eventPayload(event) }}</pre>
      </article>
    </div>
  </div>
</template>

<style scoped>
.websocket-panel {
  display: flex;
  min-height: 420px;
  flex-direction: column;
  gap: 12px;
}

.connection-row {
  display: flex;
  gap: 8px;
}

.url-input,
textarea {
  min-width: 0;
  border: 1px solid var(--color-border-strong);
  border-radius: 6px;
  padding: 8px 10px;
  color: var(--color-text);
  background: var(--color-code-bg);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 12px;
}

.url-input {
  flex: 1 1 auto;
  min-height: 34px;
}

textarea {
  width: 100%;
  resize: vertical;
  line-height: 1.45;
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
  background: var(--color-surface-2);
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
textarea:disabled,
.url-input:disabled {
  cursor: not-allowed;
  opacity: 0.55;
}

.panel-error {
  margin: 0;
  color: var(--color-danger);
  font-size: 12px;
  line-height: 1.4;
  white-space: pre-wrap;
}

.panel-status,
.events-header,
.event-meta,
.composer-actions {
  display: flex;
  align-items: center;
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

.status-session {
  margin-left: auto;
  color: var(--color-text-subtle);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
}

.message-composer {
  display: flex;
  flex-direction: column;
  gap: 7px;
  border-top: 1px solid var(--color-border);
  padding-top: 12px;
}

.composer-actions {
  justify-content: space-between;
  gap: 12px;
}

.composer-actions span,
.event-count {
  color: var(--color-text-subtle);
  font-size: 11px;
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

.event-meta strong {
  color: var(--color-text);
}

.event-outgoing .event-meta strong {
  color: var(--color-brand);
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

  .status-session {
    display: none;
  }
}
</style>
