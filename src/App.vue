<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

interface AppInfo {
  appName: string;
  version: string;
  operatingSystem: string;
  architecture: string;
}

interface HeaderEntry {
  name: string;
  value: string;
  enabled: boolean;
}

interface QueryParam {
  name: string;
  value: string;
  enabled: boolean;
}

type RequestBody =
  | {
    type: "json";
    value: unknown;
  }
  | {
    type: "text";
    value: string
  };

interface RequestDefinition {
  id: string;
  name: string;
  method: string;
  url: string;
  query: QueryParam[];
  headers: HeaderEntry[];
  body: RequestBody | null;
}

const httpMethods = [
  "GET",
  "POST",
  "PUT",
  "PATCH",
  "DELETE",
] as const;


const appInfo = ref<AppInfo | null>(null)
const errorMessage = ref("");

const sampleRequest = ref<RequestDefinition | null>(null);
const requestDraft = ref<RequestDefinition | null>(null);

const requestDraftJson = computed(() => {
  if (!requestDraft.value) {
    return "";
  }

  return JSON.stringify(requestDraft.value, null, 2);
});

async function loadAppInfo() {
  try {
    appInfo.value = await invoke<AppInfo>("get_app_info");
  } catch (error) {
    errorMessage.value = String(error)
  }
}

async function loadSampleRequest() {
  try {
    const request = await invoke<RequestDefinition>(
      "get_sample_request",
    );

    sampleRequest.value = request;
    requestDraft.value = structuredClone(request);

  } catch (error) {
    errorMessage.value = String(error)
  }
}

function addHeader() {
  if (!requestDraft.value) {
    return;
  }

  requestDraft.value.headers.push({
    name: "",
    value: "",
    enabled: true,
  })

}

function removeHeader(index: number) {
  if (!requestDraft.value) {
    return;
  }

  requestDraft.value.headers.splice(index, 1)

}



function resetRequest() {
  if (!sampleRequest.value) {
    return;
  }

  requestDraft.value = structuredClone(sampleRequest.value);
}

onMounted(async () => {
  await loadAppInfo();
  await loadSampleRequest();
})


</script>

<template>
  <main class="container">
    <h1>Larry API Client</h1>

    <section v-if="appInfo">
      <h2>Diagnóstico da aplicação</h2>

      <dl>
        <dt>Nome</dt>
        <dd>{{ appInfo.appName }}</dd>

        <dt>Versão</dt>
        <dd>{{ appInfo.version }}</dd>

        <dt>Sistema operacional</dt>
        <dd>{{ appInfo.operatingSystem }}</dd>

        <dt>Arquitetura</dt>
        <dd>{{ appInfo.architecture }}</dd>
      </dl>
    </section>

    <section v-if="requestDraft">
      <h2>Editor de requisição</h2>

      <section class="headers-section">
        <div class="section-heading">
          <h3>Headers</h3>

          <button type="button" @click="addHeader">
            Adicionar header
          </button>
        </div>

        <div v-for="(header, index) in requestDraft.headers" :key="index" class="header-row">
          <input v-model="header.enabled" type="checkbox" :aria-label="`Habilitar header ${index + 1}`" />
          <input v-model="header.name" type="text" placeholder="Nome" :aria-label="`Nome do header ${index + 1}`" />
          <input v-model="header.value" type="text" placeholder="Valor" :aria-label="`Valor do header ${index + 1}`" />

          <button type="button" @click="removeHeader(index)">
            Remover
          </button>

        </div>

        <p v-if="requestDraft.headers.length === 0">
          Nenhum header configurado.
        </p>
      </section>

      <form @submit.prevent>
        <label>
          Nome

          <input v-model="requestDraft.name" type="text" />
        </label>

        <label>
          Método

          <select v-model="requestDraft.method">
            <option v-for="method in httpMethods" :key="method" :value="method">
              {{ method }}
            </option>
          </select>
        </label>

        <label>
          URL

          <input v-model="requestDraft.url" type="url" />
        </label>

        <button type="button" @click="resetRequest">
          Restaurar exemplo
        </button>
      </form>

      <h3>Preview da requisição</h3>

      <pre>{{ requestDraftJson }}</pre>
    </section>

    <p v-else-if="errorMessage">
      Erro: {{ errorMessage }}
    </p>

    <p v-else>
      Carregando informações...
    </p>
  </main>
</template>

<style scoped>
.logo.vite:hover {
  filter: drop-shadow(0 0 2em #747bff);
}

.logo.vue:hover {
  filter: drop-shadow(0 0 2em #249b73);
}
</style>
<style>
:root {
  font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
  font-size: 16px;
  line-height: 24px;
  font-weight: 400;

  color: #0f0f0f;
  background-color: #f6f6f6;

  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

.container {
  margin: 0;
  padding-top: 10vh;
  display: flex;
  flex-direction: column;
  justify-content: center;
  text-align: center;
}

.logo {
  height: 6em;
  padding: 1.5em;
  will-change: filter;
  transition: 0.75s;
}

.logo.tauri:hover {
  filter: drop-shadow(0 0 2em #24c8db);
}

.header-section {
  max-width: 800px;
  margin: 24px auto;
  text-align: left;
}

.section-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px
}

.header-row {
  display: grid;
  grid-template-columns: auto 1fr 2fr auto;
  gap: 8px;
  align-items: center;
  margin-bottom: 8px;
}

.header-row input[type="checkbox"] {
  width: auto;
}


.row {
  display: flex;
  justify-content: center;
}

a {
  font-weight: 500;
  color: #646cff;
  text-decoration: inherit;
}

a:hover {
  color: #535bf2;
}

h1 {
  text-align: center;
}

form {
  display: flex;
  flex-direction: column;
  gap: 12px;
  max-width: 800px;
  margin: 0 auto 24px;
  text-align: left;
}

label {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

input,
select {
  padding: 8px;
  border: 1px solid #999;
  border-radius: 6px;
}

input,
button {
  border-radius: 8px;
  border: 1px solid transparent;
  padding: 0.6em 1.2em;
  font-size: 1em;
  font-weight: 500;
  font-family: inherit;
  color: #0f0f0f;
  background-color: #ffffff;
  transition: border-color 0.25s;
  box-shadow: 0 2px 2px rgba(0, 0, 0, 0.2);
}

button {
  cursor: pointer;
}

button:hover {
  border-color: #396cd8;
}

button:active {
  border-color: #396cd8;
  background-color: #e8e8e8;
}

input,
button {
  outline: none;
}

#greet-input {
  margin-right: 5px;
}

@media (prefers-color-scheme: dark) {
  :root {
    color: #f6f6f6;
    background-color: #2f2f2f;
  }

  a:hover {
    color: #24c8db;
  }

  input,
  button {
    color: #ffffff;
    background-color: #0f0f0f98;
  }

  button:active {
    background-color: #0f0f0f69;
  }
}
</style>