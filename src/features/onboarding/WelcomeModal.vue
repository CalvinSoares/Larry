<script setup lang="ts">
import AppModal from "../../components/ui/AppModal.vue";

defineProps<{
  open: boolean;
}>();

const emit = defineEmits<{
  (event: "close"): void;
  (event: "import-postman"): void;
  (event: "create-collection"): void;
  (event: "start-request"): void;
  (event: "explore"): void;
}>();
</script>

<template>
  <AppModal
    :open="open"
    variant="neutral"
    kicker="COMEÇAR"
    title="Bem-vindo ao Larry"
    description="API testing e diagnóstico local"
    primary-label="Explorar workspace"
    secondary-label=""
    :close-on-backdrop="false"
    @close="emit('close')"
    @confirm="emit('explore')"
  >
    <div class="welcome-content">
      <img class="welcome-logo" src="/assets/larry-platypus.png" alt="Larry" />
      <h3>O que você deseja fazer?</h3>

      <div class="welcome-options">
        <button class="welcome-option welcome-option-featured" type="button" @click="emit('import-postman')">
          <span class="welcome-option-icon" aria-hidden="true">
            <svg viewBox="0 0 16 16">
              <path d="M8 2v8M5 7l3 3 3-3M3 11.5v1.5h10v-1.5" />
            </svg>
          </span>
          <span>
            <strong>Importar do Postman</strong>
            <small>Traga uma collection existente para o Larry</small>
          </span>
        </button>

        <div class="welcome-option-row">
          <button class="welcome-option" type="button" @click="emit('create-collection')">
            <span class="welcome-option-icon" aria-hidden="true">
              <svg viewBox="0 0 16 16">
                <path d="M8 3v10M3 8h10" />
              </svg>
            </span>
            <span>
              <strong>Criar collection</strong>
              <small>Comece uma collection local e versionável</small>
            </span>
          </button>

          <button class="welcome-option" type="button" @click="emit('start-request')">
            <span class="welcome-option-icon" aria-hidden="true">
              <svg viewBox="0 0 16 16">
                <path d="m3 3 10 5-10 5 2.2-5L3 3Z" />
                <path d="M5.2 8h7.4" />
              </svg>
            </span>
            <span>
              <strong>Começar com uma request</strong>
              <small>Teste um endpoint sem criar uma collection</small>
            </span>
          </button>
        </div>
      </div>
    </div>
  </AppModal>
</template>

<style scoped>
.welcome-content {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.welcome-logo {
  width: 44px;
  height: 44px;
  border-radius: 9px;
  object-fit: cover;
}

.welcome-content h3 {
  margin: 0;
  color: var(--color-text);
  font-size: 17px;
}

.welcome-options {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.welcome-option-row {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
}

.welcome-option {
  display: flex;
  min-width: 0;
  align-items: flex-start;
  gap: 12px;
  border: 1px solid var(--color-border);
  border-radius: 7px;
  padding: 13px;
  color: var(--color-text);
  background: var(--color-surface-2);
  cursor: pointer;
  text-align: left;
}

.welcome-option:hover,
.welcome-option:focus-visible {
  border-color: var(--color-brand);
  background: var(--color-surface-3);
}

.welcome-option-featured {
  border-color: var(--color-brand-strong);
  background: #172b30;
}

.welcome-option-icon {
  display: inline-flex;
  flex: 0 0 auto;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 6px;
  color: var(--color-brand);
  background: #20373b;
}

.welcome-option-icon svg {
  width: 17px;
  height: 17px;
  fill: none;
  stroke: currentColor;
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 1.2;
}

.welcome-option > span:last-child {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 4px;
}

.welcome-option strong {
  font-size: 13px;
}

.welcome-option small {
  color: var(--color-text-muted);
  font-size: 12px;
  line-height: 1.35;
}

@media (max-width: 520px) {
  .welcome-option-row {
    grid-template-columns: 1fr;
  }
}
</style>
