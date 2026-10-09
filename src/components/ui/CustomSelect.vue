<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from "vue";

import type { CustomSelectOption } from "../../types/ui";

const props = withDefaults(
  defineProps<{
    modelValue: string;
    options: CustomSelectOption[];
    label: string;
    placeholder?: string;
    disabled?: boolean;
    mono?: boolean;
  }>(),
  {
    placeholder: "Selecione uma opção",
    disabled: false,
    mono: false,
  },
);

const emit = defineEmits<{
  (event: "update:modelValue", value: string): void;
  (event: "change", value: string): void;
}>();

const rootElement = ref<HTMLElement | null>(null);
const triggerElement = ref<HTMLButtonElement | null>(null);
const popoverElement = ref<HTMLDivElement | null>(null);
const isOpen = ref(false);
const highlightedIndex = ref(-1);

const selectedOption = computed(() => (
  props.options.find((option) => option.value === props.modelValue) ?? null
));

const enabledOptions = computed(() => props.options.filter((option) => !option.disabled));

function optionIndex(option: CustomSelectOption) {
  return props.options.findIndex((candidate) => candidate.value === option.value);
}

function openMenu() {
  if (props.disabled || isOpen.value) {
    return;
  }

  isOpen.value = true;
  const initialOption = selectedOption.value && !selectedOption.value.disabled
    ? selectedOption.value
    : enabledOptions.value[0];
  highlightedIndex.value = initialOption ? optionIndex(initialOption) : -1;
  void nextTick(() => popoverElement.value?.focus());
}

function closeMenu(restoreFocus = false) {
  if (!isOpen.value) {
    return;
  }

  isOpen.value = false;
  highlightedIndex.value = -1;

  if (restoreFocus) {
    void nextTick(() => triggerElement.value?.focus());
  }
}

function toggleMenu() {
  if (isOpen.value) {
    closeMenu();
  } else {
    openMenu();
  }
}

function selectOption(option: CustomSelectOption) {
  if (option.disabled) {
    return;
  }

  emit("update:modelValue", option.value);
  emit("change", option.value);
  closeMenu(true);
}

function moveHighlight(direction: 1 | -1) {
  if (!isOpen.value) {
    openMenu();
    return;
  }

  if (enabledOptions.value.length === 0) {
    return;
  }

  const currentEnabledIndex = enabledOptions.value.findIndex(
    (option) => option.value === props.options[highlightedIndex.value]?.value,
  );
  const nextIndex = currentEnabledIndex < 0
    ? direction === 1 ? 0 : enabledOptions.value.length - 1
    : (currentEnabledIndex + direction + enabledOptions.value.length) % enabledOptions.value.length;

  highlightedIndex.value = optionIndex(enabledOptions.value[nextIndex]);
}

function selectHighlighted() {
  const option = props.options[highlightedIndex.value];
  if (option) {
    selectOption(option);
  }
}

function handleTriggerKeydown(event: KeyboardEvent) {
  if (event.key === "ArrowDown" || event.key === "ArrowUp") {
    event.preventDefault();
    moveHighlight(event.key === "ArrowDown" ? 1 : -1);
    return;
  }

  if (event.key === "Enter" || event.key === " ") {
    event.preventDefault();
    if (isOpen.value) {
      selectHighlighted();
    } else {
      openMenu();
    }
    return;
  }

  if (event.key === "Escape") {
    event.preventDefault();
    closeMenu();
  }
}

function handleMenuKeydown(event: KeyboardEvent) {
  if (event.key === "ArrowDown" || event.key === "ArrowUp") {
    event.preventDefault();
    moveHighlight(event.key === "ArrowDown" ? 1 : -1);
    return;
  }

  if (event.key === "Enter" || event.key === " ") {
    event.preventDefault();
    selectHighlighted();
    return;
  }

  if (event.key === "Escape") {
    event.preventDefault();
    closeMenu(true);
  }
}

function closeOnOutsidePointer(event: PointerEvent) {
  if (!(event.target instanceof Node) || !rootElement.value?.contains(event.target)) {
    closeMenu();
  }
}

function optionClass(option: CustomSelectOption, index: number) {
  return {
    "custom-select-option-highlighted": highlightedIndex.value === index,
    "custom-select-option-selected": option.value === props.modelValue,
    "custom-select-option-disabled": option.disabled,
    [`custom-select-option-${option.tone ?? "default"}`]: true,
  };
}

onMounted(() => {
  document.addEventListener("pointerdown", closeOnOutsidePointer);
});

onBeforeUnmount(() => {
  document.removeEventListener("pointerdown", closeOnOutsidePointer);
});
</script>

<template>
  <div
    ref="rootElement"
    class="custom-select"
    :class="{
      'custom-select-open': isOpen,
      'custom-select-mono': props.mono,
      'custom-select-tone-success': selectedOption?.tone === 'success',
      'custom-select-tone-warning': selectedOption?.tone === 'warning',
      'custom-select-tone-info': selectedOption?.tone === 'info',
    }"
  >
    <button
      ref="triggerElement"
      class="custom-select-trigger"
      :class="{ 'custom-select-trigger-mono': props.mono }"
      type="button"
      role="combobox"
      :aria-label="props.label"
      :aria-expanded="isOpen"
      aria-haspopup="listbox"
      :disabled="props.disabled"
      @click="toggleMenu"
      @keydown="handleTriggerKeydown"
    >
      <span class="custom-select-value" :class="{ 'custom-select-value-placeholder': !selectedOption }">
        {{ selectedOption?.label ?? props.placeholder }}
      </span>
      <svg class="custom-select-chevron" viewBox="0 0 16 16" aria-hidden="true">
        <path d="m4 6 4 4 4-4" />
      </svg>
    </button>

    <div
      v-if="isOpen"
      ref="popoverElement"
      class="custom-select-popover"
      role="listbox"
      :aria-label="props.label"
      :aria-activedescendant="highlightedIndex >= 0 ? `custom-select-option-${highlightedIndex}` : undefined"
      tabindex="-1"
      @keydown="handleMenuKeydown"
    >
      <button
        v-for="(option, index) in props.options"
        :key="option.value"
        :id="`custom-select-option-${index}`"
        class="custom-select-option"
        :class="optionClass(option, index)"
        type="button"
        role="option"
        :aria-selected="option.value === props.modelValue"
        :disabled="option.disabled"
        @mouseenter="highlightedIndex = index"
        @click="selectOption(option)"
      >
        <span>{{ option.label }}</span>
        <svg v-if="option.value === props.modelValue" class="custom-select-check" viewBox="0 0 16 16" aria-hidden="true">
          <path d="m3.5 8.5 3 3 6-7" />
        </svg>
      </button>
    </div>
  </div>
</template>

<style scoped>
.custom-select {
  position: relative;
  display: block;
  min-width: 0;
}

.custom-select-trigger {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  height: 32px;
  min-width: 0;
  gap: 8px;
  border: 1px solid var(--color-border);
  border-radius: 5px;
  padding: 0 10px;
  color: var(--color-text);
  background: var(--color-surface-1);
  cursor: pointer;
  font: inherit;
  font-size: 12px;
  text-align: left;
}

.custom-select-trigger:hover:not(:disabled) {
  border-color: var(--color-border-strong);
  background: var(--color-surface-2);
}

.custom-select-trigger:focus-visible {
  border-color: var(--color-brand);
  outline: 2px solid var(--color-brand);
  outline-offset: 2px;
}

.custom-select-trigger:disabled {
  cursor: not-allowed;
  opacity: 0.55;
}

.custom-select-trigger-mono,
.custom-select-trigger-mono .custom-select-value,
.custom-select-mono .custom-select-option {
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
}

.custom-select-tone-success .custom-select-trigger {
  color: var(--color-success);
}

.custom-select-tone-warning .custom-select-trigger {
  color: var(--color-warning);
}

.custom-select-tone-info .custom-select-trigger {
  color: var(--color-info);
}

.custom-select-value {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.custom-select-value-placeholder {
  color: var(--color-text-subtle);
}

.custom-select-chevron {
  flex: 0 0 auto;
  width: 12px;
  height: 12px;
  fill: none;
  stroke: var(--color-text-muted);
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 1.2;
  transition: transform 120ms ease;
}

.custom-select-open .custom-select-chevron {
  transform: rotate(180deg);
}

.custom-select-popover {
  position: absolute;
  z-index: 50;
  top: calc(100% + 4px);
  left: 0;
  min-width: 100%;
  max-height: 280px;
  overflow: auto;
  border: 1px solid var(--color-border-strong);
  border-radius: 7px;
  padding: 4px;
  background: var(--color-surface-2);
  box-shadow: 0 16px 32px var(--color-bg);
}

.custom-select-option {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  height: 29px;
  gap: 8px;
  border: 0;
  border-radius: 4px;
  padding: 0 8px;
  color: var(--color-text-muted);
  background: transparent;
  cursor: pointer;
  font: inherit;
  font-size: 12px;
  text-align: left;
}

.custom-select-option:hover:not(:disabled),
.custom-select-option-highlighted:not(:disabled) {
  background: var(--color-surface-3);
  color: var(--color-text);
}

.custom-select-option-selected,
.custom-select-option-success {
  color: var(--color-brand);
}

.custom-select-option-warning {
  color: var(--color-warning);
}

.custom-select-option-info {
  color: var(--color-info);
}

.custom-select-option-disabled {
  cursor: not-allowed;
  opacity: 0.45;
}

.custom-select-check {
  flex: 0 0 auto;
  width: 14px;
  height: 14px;
  fill: none;
  stroke: currentColor;
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 1.5;
}
</style>
