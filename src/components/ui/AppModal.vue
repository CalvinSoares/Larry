<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref, watch } from "vue";

type ModalVariant = "neutral" | "info" | "danger" | "error";

const props = withDefaults(
  defineProps<{
    open: boolean;
    title: string;
    description?: string;
    variant?: ModalVariant;
    primaryLabel?: string;
    secondaryLabel?: string;
    closeOnBackdrop?: boolean;
  }>(),
  {
    description: "",
    variant: "neutral",
    primaryLabel: "Confirmar",
    secondaryLabel: "Cancelar",
    closeOnBackdrop: true,
  },
);

const emit = defineEmits<{
  (event: "close"): void;
  (event: "confirm"): void;
}>();

const modalElement = ref<HTMLElement | null>(null);
const secondaryButton = ref<HTMLButtonElement | null>(null);
let previouslyFocused: HTMLElement | null = null;

const focusableSelector = [
  "button:not([disabled])",
  "[href]",
  "input:not([disabled])",
  "select:not([disabled])",
  "textarea:not([disabled])",
  "[tabindex]:not([tabindex=\"-1\"])",
].join(", ");

function focusModal() {
  const target = secondaryButton.value ?? modalElement.value?.querySelector<HTMLElement>(focusableSelector);
  target?.focus();
}

function closeModal() {
  emit("close");
}

function handleBackdrop() {
  if (props.closeOnBackdrop) {
    closeModal();
  }
}

function handleKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    closeModal();
    return;
  }

  if (event.key !== "Tab" || !modalElement.value) {
    return;
  }

  const focusableElements = Array.from(
    modalElement.value.querySelectorAll<HTMLElement>(focusableSelector),
  );

  if (focusableElements.length === 0) {
    event.preventDefault();
    modalElement.value.focus();
    return;
  }

  const firstElement = focusableElements[0];
  const lastElement = focusableElements[focusableElements.length - 1];

  if (event.shiftKey && document.activeElement === firstElement) {
    event.preventDefault();
    lastElement.focus();
  } else if (!event.shiftKey && document.activeElement === lastElement) {
    event.preventDefault();
    firstElement.focus();
  }
}

watch(
  () => props.open,
  async (isOpen) => {
    if (isOpen) {
      previouslyFocused = document.activeElement instanceof HTMLElement
        ? document.activeElement
        : null;
      await nextTick();
      focusModal();
      return;
    }

    await nextTick();
    previouslyFocused?.focus();
    previouslyFocused = null;
  },
  { immediate: true },
);

onBeforeUnmount(() => {
  previouslyFocused?.focus();
});
</script>

<template>
  <Teleport to="body">
    <div v-if="props.open" class="modal-backdrop" @mousedown.self="handleBackdrop">
      <section
        ref="modalElement"
        class="app-modal"
        :class="`app-modal-${props.variant}`"
        role="dialog"
        aria-modal="true"
        :aria-labelledby="`${props.title.replace(/\s+/g, '-').toLowerCase()}-title`"
        @keydown="handleKeydown"
      >
        <header class="modal-header">
          <span class="modal-kicker">{{ props.variant === "danger" ? "CONFIRMAÇÃO" : "ATENÇÃO" }}</span>
          <button class="modal-close" type="button" aria-label="Fechar modal" title="Fechar" @click="closeModal">
            <svg viewBox="0 0 16 16" aria-hidden="true">
              <path d="m4 4 8 8M12 4l-8 8" />
            </svg>
          </button>
        </header>

        <div class="modal-body">
          <h2 :id="`${props.title.replace(/\s+/g, '-').toLowerCase()}-title`">{{ props.title }}</h2>
          <p v-if="props.description">{{ props.description }}</p>
          <div v-if="$slots.default" class="modal-content">
            <slot />
          </div>
        </div>

        <footer class="modal-actions">
          <button
            v-if="props.secondaryLabel"
            ref="secondaryButton"
            class="modal-button modal-button-secondary"
            type="button"
            @click="closeModal"
          >
            {{ props.secondaryLabel }}
          </button>
          <button
            class="modal-button modal-button-primary"
            :class="{ 'modal-button-danger': props.variant === 'danger' }"
            type="button"
            @click="emit('confirm')"
          >
            {{ props.primaryLabel }}
          </button>
        </footer>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.modal-backdrop {
  position: fixed;
  z-index: 100;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px;
  background: rgb(5 7 10 / 76%);
}

.app-modal {
  width: min(460px, 100%);
  max-height: calc(100vh - 40px);
  overflow: auto;
  border: 1px solid var(--color-border-strong);
  border-radius: 8px;
  background: var(--color-surface-1);
  box-shadow: 0 18px 48px rgb(0 0 0 / 36%);
}

.modal-header,
.modal-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.modal-header {
  min-height: 48px;
  border-bottom: 1px solid var(--color-border);
  padding: 0 16px;
}

.modal-kicker {
  color: var(--color-text-subtle);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.08em;
}

.app-modal-danger .modal-kicker,
.app-modal-error .modal-kicker {
  color: var(--color-warning);
}

.modal-close {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  border: 1px solid transparent;
  border-radius: 5px;
  color: var(--color-text-muted);
  background: transparent;
  cursor: pointer;
}

.modal-close:hover {
  border-color: var(--color-border-strong);
  color: var(--color-text);
  background: var(--color-surface-2);
}

.modal-close svg {
  width: 16px;
  height: 16px;
  fill: none;
  stroke: currentColor;
  stroke-linecap: round;
  stroke-width: 1.2;
}

.modal-body {
  padding: 20px 20px 8px;
}

.modal-body h2 {
  margin: 0;
  color: var(--color-text);
  font-size: 18px;
}

.modal-body p {
  margin: 10px 0 0;
  color: var(--color-text-muted);
  line-height: 1.5;
}

.modal-content {
  margin-top: 14px;
  color: var(--color-text-muted);
}

.modal-actions {
  justify-content: flex-end;
  padding: 12px 20px 20px;
}

.modal-button {
  min-height: 34px;
  border: 1px solid var(--color-border-strong);
  border-radius: 5px;
  padding: 6px 12px;
  color: var(--color-text);
  background: var(--color-surface-2);
  cursor: pointer;
  font: inherit;
}

.modal-button:hover {
  border-color: var(--color-brand);
  background: var(--color-surface-3);
}

.modal-button-primary {
  border-color: var(--color-brand-strong);
  color: #071315;
  background: var(--color-brand);
  font-weight: 700;
}

.modal-button-danger {
  border-color: #b8545e;
  color: #fff;
  background: #8f3541;
}

.modal-button-danger:hover {
  border-color: #ef8585;
  background: #a8404d;
}

@media (max-width: 520px) {
  .modal-backdrop {
    align-items: flex-end;
    padding: 12px;
  }

  .app-modal {
    max-height: calc(100vh - 24px);
  }

  .modal-actions {
    align-items: stretch;
    flex-direction: column-reverse;
  }

  .modal-button {
    width: 100%;
  }
}
</style>
