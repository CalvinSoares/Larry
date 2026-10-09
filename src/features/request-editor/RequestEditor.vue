<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { open } from "@tauri-apps/plugin-dialog";

import { formatIpcError } from "../../services/errors";
import { parseCurlRequest } from "../../services/ipc";
import CustomSelect from "../../components/ui/CustomSelect.vue";
import type {
  AssertionDefinition,
  FormField,
  MultipartFile,
  RequestAuth,
  RequestBody,
  RequestDefinition,
} from "../../types/api";
import type { CustomSelectOption } from "../../types/ui";

const httpMethods = [
  "GET",
  "POST",
  "PUT",
  "PATCH",
  "DELETE",
] as const;

const httpMethodOptions: CustomSelectOption[] = httpMethods.map((method) => ({
  value: method,
  label: method,
  tone: method === "GET" ? "success" : method === "POST" || method === "DELETE" ? "warning" : "info",
}));

const authTypeOptions: CustomSelectOption[] = [
  { value: "none", label: "No auth" },
  { value: "bearer", label: "Bearer token", tone: "info" },
  { value: "basic", label: "Basic auth" },
  { value: "apiKey", label: "API key", tone: "warning" },
];

const apiKeyLocationOptions: CustomSelectOption[] = [
  { value: "header", label: "Header" },
  { value: "query", label: "Query" },
];

const assertionTypeOptions: CustomSelectOption[] = [
  { value: "statusEquals", label: "Status é igual a" },
  { value: "headerContains", label: "Header contém" },
  { value: "bodyContains", label: "Body contém" },
];

const props = defineProps<{
  request: RequestDefinition;
  isExecuting: boolean;
  resetToken: number;
}>();

const emit = defineEmits<{
  (event: "update:request", request: RequestDefinition): void;
  (event: "submit"): void;
  (event: "reset"): void;
  (event: "validation-error", message: string): void;
}>();

const localRequest = ref<RequestDefinition>(cloneRequest(props.request));
const bodyType = ref<"json" | "text" | "form-urlencoded" | "multipart" | "none">("json");
const bodyText = ref("");
const bodyError = ref("");
const formFields = ref<FormField[]>([]);
const multipartFiles = ref<MultipartFile[]>([]);
const formError = ref("");
const isBodyMenuOpen = ref(false);
const bodyEditor = ref<HTMLElement | null>(null);
const bodyHighlight = ref<HTMLElement | null>(null);
const bodyGutter = ref<HTMLElement | null>(null);
type ComposerTab = "params" | "headers" | "auth" | "cookies" | "body" | "assertions";
const activeTab = ref<ComposerTab>("params");

type BodyFormat = "multipart" | "form-urlencoded" | "json" | "xml" | "text" | "sparql" | "binary" | "none";

const enabledParameterCount = computed(
  () => localRequest.value.query.filter((parameter) => parameter.enabled).length,
);
const enabledHeaderCount = computed(
  () => localRequest.value.headers.filter((header) => header.enabled).length,
);
const enabledCookieCount = computed(
  () => localRequest.value.cookies.filter((cookie) => cookie.enabled).length,
);
const assertionCount = computed(() => localRequest.value.assertions.length);
const bodyLineCount = computed(() => Math.max(1, bodyText.value.split("\n").length));
const bodyFormatLabel = computed(() => {
  if (bodyType.value === "json") {
    return "JSON";
  }

  if (bodyType.value === "text") {
    return "TEXT";
  }

  if (bodyType.value === "form-urlencoded") {
    return "FORM URLENCODED";
  }

  if (bodyType.value === "multipart") {
    return "MULTIPART";
  }

  return "No Body";
});
type AuthType = "none" | RequestAuth["type"];
const selectedAuthType = computed<AuthType>({
  get: () => localRequest.value.auth?.type ?? "none",
  set: (value) => setAuthType(value),
});

function cloneRequest(request: RequestDefinition) {
  return JSON.parse(JSON.stringify(request)) as RequestDefinition;
}

async function handleUrlPaste(event: ClipboardEvent) {
  const pastedText = event.clipboardData?.getData("text").trim() ?? "";

  if (!/^curl(?:\.exe)?\s/i.test(pastedText)) {
    return;
  }

  event.preventDefault();

  try {
    const preview = await parseCurlRequest(pastedText);
    const importedRequest = cloneRequest(preview.request);

    importedRequest.id = localRequest.value.id;
    importedRequest.name = localRequest.value.name;
    localRequest.value = importedRequest;
    loadBodyEditor(importedRequest.body);
    emit("validation-error", "");
  } catch (error) {
    emit("validation-error", formatIpcError(error));
  }
}

function createAuth(type: Exclude<AuthType, "none">): RequestAuth {
  if (type === "bearer") {
    return { type, token: "" };
  }

  if (type === "basic") {
    return { type, username: "", password: "" };
  }

  return { type, name: "", value: "", location: "header" };
}

function setAuthType(type: AuthType) {
  localRequest.value.auth = type === "none" ? null : createAuth(type);
}

function updateBearerToken(event: Event) {
  if (localRequest.value.auth?.type === "bearer") {
    localRequest.value.auth.token = (event.target as HTMLInputElement).value;
  }
}

function updateBasicField(field: "username" | "password", event: Event) {
  if (localRequest.value.auth?.type === "basic") {
    localRequest.value.auth[field] = (event.target as HTMLInputElement).value;
  }
}

function updateApiKeyField(field: "name" | "value", event: Event) {
  if (localRequest.value.auth?.type === "apiKey") {
    localRequest.value.auth[field] = (event.target as HTMLInputElement).value;
  }
}

function updateApiKeyLocation(value: string) {
  if (localRequest.value.auth?.type === "apiKey") {
    localRequest.value.auth.location = value as "header" | "query";
  }
}

function escapeHtml(value: string) {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
}

function highlightJson(value: string) {
  const tokenPattern = /"(?:\\.|[^"\\])*"|-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?|\b(?:true|false|null)\b/g;
  let output = "";
  let cursor = 0;

  for (const match of value.matchAll(tokenPattern)) {
    const token = match[0];
    const index = match.index ?? 0;
    const trailingText = value.slice(index + token.length);
    const tokenClass = token.startsWith("\"")
      ? /^\s*:/.test(trailingText)
        ? "token-key"
        : "token-string"
      : /\d/.test(token)
        ? "token-number"
        : "token-literal";

    output += escapeHtml(value.slice(cursor, index));
    output += `<span class="${tokenClass}">${escapeHtml(token)}</span>`;
    cursor = index + token.length;
  }

  return `${output}${escapeHtml(value.slice(cursor))}`;
}

const highlightedBody = computed(() => (
  bodyType.value === "json" ? highlightJson(bodyText.value) : escapeHtml(bodyText.value)
));

function syncBodyScroll() {
  if (!bodyEditor.value) {
    return;
  }

  if (bodyHighlight.value) {
    bodyHighlight.value.scrollTop = bodyEditor.value.scrollTop;
    bodyHighlight.value.scrollLeft = bodyEditor.value.scrollLeft;
  }

  if (bodyGutter.value) {
    bodyGutter.value.scrollTop = bodyEditor.value.scrollTop;
  }
}

function syncBodyEditor() {
  void nextTick(() => {
    if (!bodyEditor.value) {
      return;
    }

    if (bodyEditor.value.textContent !== bodyText.value) {
      bodyEditor.value.textContent = bodyText.value;
    }

    syncBodyScroll();
  });
}

function handleBodyInput(event: Event) {
  const target = event.currentTarget as HTMLElement;
  const nextBodyText = (target.innerText || target.textContent || "").replace(/\r\n/g, "\n");

  if (bodyText.value !== nextBodyText) {
    bodyText.value = nextBodyText;
  }
}

function selectBodyFormat(format: BodyFormat) {
  if (format !== "json" && format !== "text" && format !== "none") {
    if (format !== "multipart" && format !== "form-urlencoded") {
      return;
    }
  }

  bodyType.value = format;
  bodyError.value = "";
  formError.value = "";
  isBodyMenuOpen.value = false;
}

function prettifyBody() {
  if (bodyType.value !== "json" || !bodyText.value.trim()) {
    return;
  }

  try {
    bodyText.value = JSON.stringify(JSON.parse(bodyText.value), null, 2);
    bodyError.value = "";
  } catch {
    bodyError.value = "JSON inválido. Corrija o conteúdo antes de executar.";
  }
}

function loadBodyEditor(body: RequestBody | null) {
  bodyError.value = "";
  formError.value = "";

  if (!body) {
    bodyType.value = "json";
    bodyText.value = "";
    formFields.value = [];
    multipartFiles.value = [];
    syncBodyEditor();
    return;
  }

  bodyType.value = body.type;
  if (body.type === "json") {
    bodyText.value = JSON.stringify(body.value, null, 2);
    formFields.value = [];
    multipartFiles.value = [];
  } else if (body.type === "text") {
    bodyText.value = body.value;
    formFields.value = [];
    multipartFiles.value = [];
  } else if (body.type === "form-urlencoded") {
    bodyText.value = "";
    formFields.value = body.value.map((field) => ({ ...field }));
    multipartFiles.value = [];
  } else {
    bodyText.value = "";
    formFields.value = body.value.fields.map((field) => ({ ...field }));
    multipartFiles.value = body.value.files.map((file) => ({ ...file }));
  }
  syncBodyEditor();
}

function addFormField() {
  formFields.value.push({
    name: "",
    value: "",
    enabled: true,
  });
  formError.value = "";
}

function removeFormField(index: number) {
  formFields.value.splice(index, 1);
}

function removeMultipartFile(index: number) {
  multipartFiles.value.splice(index, 1);
}

function fileNameFromPath(path: string) {
  return path.split(/[\\/]/).pop() || path;
}

async function chooseMultipartFile(index?: number) {
  formError.value = "";

  try {
    const selected = await open({
      multiple: false,
      directory: false,
      title: "Selecionar arquivo para multipart",
    });
    const path = Array.isArray(selected) ? selected[0] : selected;

    if (!path) {
      return;
    }

    if (index === undefined) {
      multipartFiles.value.push({
        name: "file",
        path,
        enabled: true,
      });
    } else {
      multipartFiles.value[index].path = path;
    }
  } catch (error) {
    formError.value = error instanceof Error
      ? `Não foi possível selecionar o arquivo: ${error.message}`
      : "Não foi possível selecionar o arquivo.";
  }
}

function addHeader() {
  localRequest.value.headers.push({
    name: "",
    value: "",
    enabled: true,
  });
}

function removeHeader(index: number) {
  localRequest.value.headers.splice(index, 1);
}

function addCookie() {
  localRequest.value.cookies.push({
    name: "",
    value: "",
    enabled: true,
  });
}

function removeCookie(index: number) {
  localRequest.value.cookies.splice(index, 1);
}

function addQueryParam() {
  localRequest.value.query.push({
    name: "",
    value: "",
    enabled: true,
  });
}

function removeQueryParam(index: number) {
  localRequest.value.query.splice(index, 1);
}

function createAssertion(type: AssertionDefinition["type"]): AssertionDefinition {
  if (type === "statusEquals") {
    return { type, expected: 200 };
  }

  if (type === "headerContains") {
    return { type, name: "Content-Type", value: "application/json" };
  }

  return { type, value: "" };
}

function addAssertion() {
  localRequest.value.assertions.push(createAssertion("statusEquals"));
}

function removeAssertion(index: number) {
  localRequest.value.assertions.splice(index, 1);
}

function updateAssertionType(index: number, value: string) {
  const type = value as AssertionDefinition["type"];
  localRequest.value.assertions[index] = createAssertion(type);
}

function updateAssertionField(index: number, field: "expected" | "name" | "value", event: Event) {
  const assertion = localRequest.value.assertions[index];
  if (!assertion) {
    return;
  }

  const value = (event.target as HTMLInputElement).value;
  if (assertion.type === "statusEquals" && field === "expected") {
    assertion.expected = Number(value) || 0;
  } else if (assertion.type === "headerContains" && (field === "name" || field === "value")) {
    assertion[field] = value;
  } else if (assertion.type === "bodyContains" && field === "value") {
    assertion.value = value;
  }
}

function handleSubmit() {
  if (bodyError.value) {
    activeTab.value = "body";
    emit("validation-error", bodyError.value);
    return;
  }

  emit("submit");
}

watch(
  localRequest,
  (request) => {
    emit("update:request", cloneRequest(request));
  },
  { deep: true },
);

watch([bodyType, bodyText, formFields, multipartFiles], () => {
  if (bodyType.value === "none") {
    localRequest.value.body = null;
    bodyText.value = "";
    bodyError.value = "";
    formError.value = "";
    syncBodyEditor();
    return;
  }

  if (bodyType.value === "text") {
    localRequest.value.body = {
      type: "text",
      value: bodyText.value,
    };
    bodyError.value = "";
    syncBodyEditor();
    return;
  }

  if (bodyType.value === "form-urlencoded") {
    localRequest.value.body = {
      type: "form-urlencoded",
      value: formFields.value.map((field) => ({ ...field })),
    };
    bodyError.value = "";
    syncBodyEditor();
    return;
  }

  if (bodyType.value === "multipart") {
    localRequest.value.body = {
      type: "multipart",
      value: {
        fields: formFields.value.map((field) => ({ ...field })),
        files: multipartFiles.value.map((file) => ({ ...file })),
      },
    };
    bodyError.value = "";
    syncBodyEditor();
    return;
  }

  if (!bodyText.value.trim()) {
    localRequest.value.body = null;
    bodyError.value = "";
    syncBodyEditor();
    return;
  }

  try {
    localRequest.value.body = {
      type: "json",
      value: JSON.parse(bodyText.value),
    };
    bodyError.value = "";
  } catch {
    bodyError.value = "JSON inválido. Corrija o conteúdo antes de executar.";
  }

  syncBodyEditor();
}, { deep: true });

watch(activeTab, () => {
  if (activeTab.value === "body") {
    syncBodyEditor();
  }
});

watch(
  () => props.resetToken,
  () => {
    localRequest.value = cloneRequest(props.request);
    loadBodyEditor(localRequest.value.body);
  },
);

loadBodyEditor(localRequest.value.body);
onMounted(syncBodyEditor);
</script>

<template>
  <form class="request-editor" @submit.prevent="handleSubmit">
    <div class="request-bar" aria-label="Barra de requisição">
      <CustomSelect
        v-model="localRequest.method"
        class="request-method-custom-select"
        :options="httpMethodOptions"
        label="Método HTTP"
        mono
      />

      <input
        v-model="localRequest.url"
        class="request-url"
        type="text"
        aria-label="URL"
        placeholder="https://example.com ou cole um comando cURL"
        required
        @paste="handleUrlPaste"
      />

      <button class="primary-action" type="submit" :disabled="props.isExecuting || Boolean(bodyError)">
        {{ props.isExecuting ? "Enviando..." : "Enviar" }}
      </button>
    </div>

    <nav class="composer-tabs" aria-label="Configurações da request" role="tablist">
      <button
        class="composer-tab"
        :class="{ 'composer-tab-active': activeTab === 'params' }"
        type="button"
        role="tab"
        :aria-selected="activeTab === 'params'"
        aria-controls="request-params-panel"
        @click="activeTab = 'params'"
      >
        Params <span>{{ enabledParameterCount }}</span>
      </button>
      <button
        class="composer-tab"
        :class="{ 'composer-tab-active': activeTab === 'headers' }"
        type="button"
        role="tab"
        :aria-selected="activeTab === 'headers'"
        aria-controls="request-headers-panel"
        @click="activeTab = 'headers'"
      >
        Headers <span>{{ enabledHeaderCount }}</span>
      </button>
      <button
        class="composer-tab"
        :class="{ 'composer-tab-active': activeTab === 'auth' }"
        type="button"
        role="tab"
        :aria-selected="activeTab === 'auth'"
        aria-controls="request-auth-panel"
        @click="activeTab = 'auth'"
      >
        Auth
      </button>
      <button
        class="composer-tab"
        :class="{ 'composer-tab-active': activeTab === 'cookies' }"
        type="button"
        role="tab"
        :aria-selected="activeTab === 'cookies'"
        aria-controls="request-cookies-panel"
        @click="activeTab = 'cookies'"
      >
        Cookies <span>{{ enabledCookieCount }}</span>
      </button>
      <button
        class="composer-tab"
        :class="{ 'composer-tab-active': activeTab === 'body' }"
        type="button"
        role="tab"
        :aria-selected="activeTab === 'body'"
        aria-controls="request-body-panel"
        @click="activeTab = 'body'"
      >
        Body <span>{{ localRequest.body ? 1 : 0 }}</span>
      </button>
      <button
        class="composer-tab"
        :class="{ 'composer-tab-active': activeTab === 'assertions' }"
        type="button"
        role="tab"
        :aria-selected="activeTab === 'assertions'"
        aria-controls="request-assertions-panel"
        @click="activeTab = 'assertions'"
      >
        Tests <span>{{ assertionCount }}</span>
      </button>
    </nav>

    <section
      v-if="activeTab === 'params'"
      id="request-params-panel"
      class="editor-section"
      role="tabpanel"
    >
      <div class="section-heading">
        <h3>Query</h3>
        <button type="button" @click="addQueryParam">Adicionar</button>
      </div>

      <div class="params-table" role="table" aria-label="Parâmetros de query">
        <div class="params-table-header" role="row">
          <span role="columnheader" aria-label="Ativo"></span>
          <span role="columnheader">Chave</span>
          <span role="columnheader">Valor</span>
          <span role="columnheader" aria-label="Ação"></span>
        </div>

        <div v-for="(param, index) in localRequest.query" :key="index" class="parameter-row" role="row">
          <input
            v-model="param.enabled"
            type="checkbox"
            :aria-label="`Habilitar parâmetro ${index + 1}`"
          />
          <input
            v-model="param.name"
            class="parameter-cell"
            type="text"
            placeholder="Nome"
            :aria-label="`Nome do parâmetro ${index + 1}`"
          />
          <input
            v-model="param.value"
            class="parameter-cell"
            type="text"
            placeholder="Valor"
            :aria-label="`Valor do parâmetro ${index + 1}`"
          />
          <button
            class="icon-button"
            type="button"
            :aria-label="`Remover parâmetro ${index + 1}`"
            :title="`Remover parâmetro ${index + 1}`"
            @click="removeQueryParam(index)"
          >
            <svg viewBox="0 0 16 16" aria-hidden="true">
              <path d="M3 5h10M6 5V3.5h4V5m-5.5 0 .6 8h5.8l.6-8M7 7.5v3.5m2-3.5v3.5" />
            </svg>
          </button>
        </div>
      </div>

      <p v-if="localRequest.query.length === 0" class="muted">Nenhum parâmetro configurado.</p>
    </section>

    <section
      v-else-if="activeTab === 'headers'"
      id="request-headers-panel"
      class="editor-section"
      role="tabpanel"
    >
      <div class="section-heading">
        <div>
          <h3>Headers</h3>
          <p>Somente headers habilitados serão enviados.</p>
        </div>

        <button type="button" @click="addHeader">Adicionar</button>
      </div>

      <div v-for="(header, index) in localRequest.headers" :key="index" class="pair-row">
        <input v-model="header.enabled" type="checkbox" :aria-label="`Habilitar header ${index + 1}`" />
        <input v-model="header.name" type="text" placeholder="Nome" :aria-label="`Nome do header ${index + 1}`" />
        <input v-model="header.value" type="text" placeholder="Valor" :aria-label="`Valor do header ${index + 1}`" />
        <button type="button" @click="removeHeader(index)">Remover</button>
      </div>

      <p v-if="localRequest.headers.length === 0" class="muted">Nenhum header configurado.</p>
    </section>

    <section
      v-else-if="activeTab === 'auth'"
      id="request-auth-panel"
      class="editor-section auth-section"
      role="tabpanel"
    >
      <div class="section-heading">
        <div>
          <h3>Authentication</h3>
          <p>Use referências como <code v-pre>{{secret.accessToken}}</code> para manter credenciais fora da collection.</p>
        </div>

        <CustomSelect
          v-model="selectedAuthType"
          class="auth-type-select"
          :options="authTypeOptions"
          label="Tipo de autenticação"
        />
      </div>

      <div v-if="selectedAuthType === 'none'" class="auth-empty">
        A request será enviada sem autenticação automática.
      </div>

      <div v-else-if="selectedAuthType === 'bearer'" class="auth-fields">
        <label>
          <span>Token</span>
          <input
            :value="localRequest.auth?.type === 'bearer' ? localRequest.auth.token : ''"
            type="password"
            autocomplete="off"
            placeholder="{{secret.accessToken}}"
            @input="updateBearerToken"
          />
        </label>
        <p class="auth-note">Será enviado como o header Authorization: Bearer.</p>
      </div>

      <div v-else-if="selectedAuthType === 'basic'" class="auth-fields auth-grid">
        <label>
          <span>Username</span>
          <input
            :value="localRequest.auth?.type === 'basic' ? localRequest.auth.username : ''"
            type="text"
            autocomplete="off"
            placeholder="usuario"
            @input="updateBasicField('username', $event)"
          />
        </label>
        <label>
          <span>Password</span>
          <input
            :value="localRequest.auth?.type === 'basic' ? localRequest.auth.password : ''"
            type="password"
            autocomplete="off"
            placeholder="{{secret.password}}"
            @input="updateBasicField('password', $event)"
          />
        </label>
        <p class="auth-note">As credenciais serão codificadas no header Authorization durante a execução.</p>
      </div>

      <div v-else class="auth-fields auth-grid">
        <label>
          <span>Key</span>
          <input
            :value="localRequest.auth?.type === 'apiKey' ? localRequest.auth.name : ''"
            type="text"
            autocomplete="off"
            placeholder="X-API-Key"
            @input="updateApiKeyField('name', $event)"
          />
        </label>
        <label>
          <span>Value</span>
          <input
            :value="localRequest.auth?.type === 'apiKey' ? localRequest.auth.value : ''"
            type="password"
            autocomplete="off"
            placeholder="{{secret.apiKey}}"
            @input="updateApiKeyField('value', $event)"
          />
        </label>
        <label>
          <span>Send in</span>
          <CustomSelect
            :model-value="localRequest.auth?.type === 'apiKey' ? localRequest.auth.location : 'header'"
            :options="apiKeyLocationOptions"
            label="Local do API key"
            @update:model-value="updateApiKeyLocation"
          />
        </label>
      </div>
    </section>

    <section
      v-else-if="activeTab === 'cookies'"
      id="request-cookies-panel"
      class="editor-section cookies-section"
      role="tabpanel"
    >
      <div class="section-heading">
        <div>
          <h3>Cookies</h3>
          <p>Cookies habilitados serão enviados no header Cookie desta request.</p>
        </div>

        <button type="button" @click="addCookie">Adicionar</button>
      </div>

      <div class="params-table" role="table" aria-label="Cookies da request">
        <div class="params-table-header" role="row">
          <span role="columnheader" aria-label="Ativo"></span>
          <span role="columnheader">Nome</span>
          <span role="columnheader">Valor</span>
          <span role="columnheader" aria-label="Ação"></span>
        </div>

        <div v-for="(cookie, index) in localRequest.cookies" :key="index" class="parameter-row" role="row">
          <input
            v-model="cookie.enabled"
            type="checkbox"
            :aria-label="`Habilitar cookie ${index + 1}`"
          />
          <input
            v-model="cookie.name"
            class="parameter-cell"
            type="text"
            placeholder="Nome"
            :aria-label="`Nome do cookie ${index + 1}`"
          />
          <input
            v-model="cookie.value"
            class="parameter-cell"
            type="text"
            placeholder="Valor ou {{secret.cookie}}"
            :aria-label="`Valor do cookie ${index + 1}`"
          />
          <button
            class="icon-button"
            type="button"
            :aria-label="`Remover cookie ${index + 1}`"
            :title="`Remover cookie ${index + 1}`"
            @click="removeCookie(index)"
          >
            <svg viewBox="0 0 16 16" aria-hidden="true">
              <path d="M3 5h10M6 5V3.5h4V5m-5.5 0 .6 8h5.8l.6-8M7 7.5v3.5m2-3.5v3.5" />
            </svg>
          </button>
        </div>
      </div>

      <p v-if="localRequest.cookies.length === 0" class="muted">Nenhum cookie configurado.</p>
    </section>

    <section
      v-else-if="activeTab === 'body'"
      id="request-body-panel"
      class="editor-section body-editor-section"
      role="tabpanel"
      @click="isBodyMenuOpen = false"
    >
      <div class="body-toolbar">
        <div class="body-toolbar-context">
          <span class="body-toolbar-mark" aria-hidden="true"></span>
          <span class="body-toolbar-title">Request payload</span>
          <span class="body-toolbar-state">local draft</span>
        </div>

        <div class="body-format-menu" @click.stop @keydown.esc.stop="isBodyMenuOpen = false">
          <button
            class="body-format-trigger"
            type="button"
            :aria-expanded="isBodyMenuOpen"
            aria-haspopup="menu"
            aria-label="Selecionar formato do body"
            @click="isBodyMenuOpen = !isBodyMenuOpen"
          >
            {{ bodyFormatLabel }}
            <svg class="body-format-chevron" viewBox="0 0 16 16" aria-hidden="true">
              <path d="m4 6 4 4 4-4" />
            </svg>
          </button>

          <div v-if="isBodyMenuOpen" class="body-format-popover" role="menu">
            <div class="body-format-popover-heading">
              <span>Payload format</span>
              <span>Core support</span>
            </div>

            <div class="body-format-group">
              <span class="body-format-group-label">FORM</span>
              <button
                class="body-format-option"
                :class="{ 'body-format-option-active': bodyType === 'multipart' }"
                type="button"
                role="menuitem"
                :aria-checked="bodyType === 'multipart'"
                @click="selectBodyFormat('multipart')"
              >
                <svg class="body-format-icon" viewBox="0 0 16 16" aria-hidden="true">
                  <path d="M3 4.5h10v7H3zM6 4.5v7m4-7v7" />
                </svg>
                Multipart Form
                <svg v-if="bodyType === 'multipart'" class="body-format-check" viewBox="0 0 16 16" aria-hidden="true">
                  <path d="m3.5 8 3 3 6-6" />
                </svg>
              </button>
              <button
                class="body-format-option"
                :class="{ 'body-format-option-active': bodyType === 'form-urlencoded' }"
                type="button"
                role="menuitem"
                :aria-checked="bodyType === 'form-urlencoded'"
                @click="selectBodyFormat('form-urlencoded')"
              >
                <svg class="body-format-icon" viewBox="0 0 16 16" aria-hidden="true">
                  <path d="M3 4.5h10v7H3zM5 7h6M5 9h4" />
                </svg>
                Form URL Encoded
                <svg v-if="bodyType === 'form-urlencoded'" class="body-format-check" viewBox="0 0 16 16" aria-hidden="true">
                  <path d="m3.5 8 3 3 6-6" />
                </svg>
              </button>
            </div>

            <div class="body-format-group">
              <span class="body-format-group-label">RAW</span>
              <button
                class="body-format-option"
                :class="{ 'body-format-option-active': bodyType === 'json' }"
                type="button"
                role="menuitem"
                :aria-checked="bodyType === 'json'"
                @click="selectBodyFormat('json')"
              >
                <svg class="body-format-icon" viewBox="0 0 16 16" aria-hidden="true">
                  <path d="M5.5 3.5 3 8l2.5 4.5m5-9L13 8l-2.5 4.5" />
                </svg>
                JSON
                <svg v-if="bodyType === 'json'" class="body-format-check" viewBox="0 0 16 16" aria-hidden="true">
                  <path d="m3.5 8 3 3 6-6" />
                </svg>
              </button>
              <button class="body-format-option" type="button" role="menuitem" disabled>
                <svg class="body-format-icon" viewBox="0 0 16 16" aria-hidden="true">
                  <path d="m6 3.5-3 4.5 3 4.5m4-9 3 4.5-3 4.5" />
                </svg>
                XML
              </button>
              <button
                class="body-format-option"
                :class="{ 'body-format-option-active': bodyType === 'text' }"
                type="button"
                role="menuitem"
                :aria-checked="bodyType === 'text'"
                @click="selectBodyFormat('text')"
              >
                <svg class="body-format-icon" viewBox="0 0 16 16" aria-hidden="true">
                  <path d="M3 4.5h10M3 8h7M3 11.5h10" />
                </svg>
                TEXT
                <svg v-if="bodyType === 'text'" class="body-format-check" viewBox="0 0 16 16" aria-hidden="true">
                  <path d="m3.5 8 3 3 6-6" />
                </svg>
              </button>
              <button class="body-format-option" type="button" role="menuitem" disabled>
                <svg class="body-format-icon" viewBox="0 0 16 16" aria-hidden="true">
                  <circle cx="5" cy="5" r="1.5" />
                  <circle cx="11" cy="11" r="1.5" />
                  <path d="m6.2 6.2 3.6 3.6" />
                </svg>
                SPARQL
              </button>
            </div>

            <div class="body-format-group">
              <span class="body-format-group-label">OTHER</span>
              <button class="body-format-option" type="button" role="menuitem" disabled>
                <svg class="body-format-icon" viewBox="0 0 16 16" aria-hidden="true">
                  <path d="M4 2.5h5l3 3v8H4zM9 2.5v3h3" />
                </svg>
                File / Binary
              </button>
              <button
                class="body-format-option"
                :class="{ 'body-format-option-active': bodyType === 'none' }"
                type="button"
                role="menuitem"
                :aria-checked="bodyType === 'none'"
                @click="selectBodyFormat('none')"
              >
                <svg class="body-format-icon" viewBox="0 0 16 16" aria-hidden="true">
                  <path d="m5 5 6 6m0-6-6 6" />
                </svg>
                No Body
                <svg v-if="bodyType === 'none'" class="body-format-check" viewBox="0 0 16 16" aria-hidden="true">
                  <path d="m3.5 8 3 3 6-6" />
                </svg>
              </button>
            </div>
          </div>
        </div>

        <button class="prettify-button" type="button" :disabled="bodyType !== 'json'" @click="prettifyBody">
          Prettify
        </button>
      </div>

      <div
        v-if="bodyType === 'json' || bodyType === 'text'"
        class="body-code-editor"
        :class="{ 'body-code-editor-error': bodyError }"
      >
        <div ref="bodyGutter" class="body-editor-gutter" aria-hidden="true">
          <div class="body-gutter-content">
            <div v-for="lineNumber in bodyLineCount" :key="lineNumber" class="body-gutter-line">
              <svg v-if="lineNumber === 1" class="body-gutter-fold" viewBox="0 0 16 16" aria-hidden="true">
                <path d="m4 6 4 4 4-4" />
              </svg>
              <span>{{ lineNumber }}</span>
            </div>
          </div>
        </div>

        <pre ref="bodyHighlight" class="body-code-highlight" aria-hidden="true"><code v-html="highlightedBody || '&nbsp;'"></code></pre>

        <div
          ref="bodyEditor"
          class="body-code-input"
          role="textbox"
          aria-multiline="true"
          aria-label="Conteúdo do body"
          :aria-invalid="Boolean(bodyError)"
          :aria-readonly="false"
          contenteditable="true"
          spellcheck="false"
          @input="handleBodyInput"
          @scroll="syncBodyScroll"
        ></div>
      </div>

      <div v-else-if="bodyType === 'form-urlencoded'" class="form-body-editor">
        <div class="form-body-heading">
          <div>
            <h3>Form fields</h3>
            <p>Campos habilitados serão enviados como application/x-www-form-urlencoded.</p>
          </div>
          <button type="button" @click="addFormField">Adicionar campo</button>
        </div>

        <div class="params-table" role="table" aria-label="Campos form URL encoded">
          <div class="params-table-header" role="row">
            <span role="columnheader" aria-label="Ativo"></span>
            <span role="columnheader">Nome</span>
            <span role="columnheader">Valor</span>
            <span role="columnheader" aria-label="Ação"></span>
          </div>

          <div v-for="(field, index) in formFields" :key="index" class="parameter-row" role="row">
            <input
              v-model="field.enabled"
              type="checkbox"
              :aria-label="`Habilitar campo ${index + 1}`"
            />
            <input
              v-model="field.name"
              class="parameter-cell"
              type="text"
              placeholder="Nome"
              :aria-label="`Nome do campo ${index + 1}`"
            />
            <input
              v-model="field.value"
              class="parameter-cell"
              type="text"
              placeholder="Valor ou {{secret.field}}"
              :aria-label="`Valor do campo ${index + 1}`"
            />
            <button
              class="icon-button"
              type="button"
              :aria-label="`Remover campo ${index + 1}`"
              :title="`Remover campo ${index + 1}`"
              @click="removeFormField(index)"
            >
              <svg viewBox="0 0 16 16" aria-hidden="true">
                <path d="M3 5h10M6 5V3.5h4V5m-5.5 0 .6 8h5.8l.6-8M7 7.5v3.5m2-3.5v3.5" />
              </svg>
            </button>
          </div>
        </div>

        <p v-if="formFields.length === 0" class="muted">Nenhum campo configurado.</p>
      </div>

      <div v-else-if="bodyType === 'multipart'" class="form-body-editor">
        <div class="form-body-heading">
          <div>
            <h3>Multipart form</h3>
            <p>Combine campos de texto e arquivos locais no mesmo envio.</p>
          </div>
          <button type="button" @click="addFormField">Adicionar campo</button>
        </div>

        <div class="params-table" role="table" aria-label="Campos multipart">
          <div class="params-table-header" role="row">
            <span role="columnheader" aria-label="Ativo"></span>
            <span role="columnheader">Nome</span>
            <span role="columnheader">Valor</span>
            <span role="columnheader" aria-label="Ação"></span>
          </div>

          <div v-for="(field, index) in formFields" :key="index" class="parameter-row" role="row">
            <input
              v-model="field.enabled"
              type="checkbox"
              :aria-label="`Habilitar campo multipart ${index + 1}`"
            />
            <input
              v-model="field.name"
              class="parameter-cell"
              type="text"
              placeholder="Nome"
              :aria-label="`Nome do campo multipart ${index + 1}`"
            />
            <input
              v-model="field.value"
              class="parameter-cell"
              type="text"
              placeholder="Valor ou {{secret.field}}"
              :aria-label="`Valor do campo multipart ${index + 1}`"
            />
            <button
              class="icon-button"
              type="button"
              :aria-label="`Remover campo multipart ${index + 1}`"
              :title="`Remover campo multipart ${index + 1}`"
              @click="removeFormField(index)"
            >
              <svg viewBox="0 0 16 16" aria-hidden="true">
                <path d="M3 5h10M6 5V3.5h4V5m-5.5 0 .6 8h5.8l.6-8M7 7.5v3.5m2-3.5v3.5" />
              </svg>
            </button>
          </div>
        </div>

        <div class="multipart-files-heading">
          <div>
            <h3>Arquivos</h3>
            <p>O caminho é lido pelo core Rust somente durante a execução.</p>
          </div>
          <button type="button" @click="chooseMultipartFile()">Selecionar arquivo</button>
        </div>

        <div v-for="(file, index) in multipartFiles" :key="index" class="multipart-file-row">
          <input
            v-model="file.enabled"
            type="checkbox"
            :aria-label="`Habilitar arquivo ${index + 1}`"
          />
          <input
            v-model="file.name"
            class="parameter-cell"
            type="text"
            placeholder="Nome do campo"
            :aria-label="`Nome do campo do arquivo ${index + 1}`"
          />
          <button
            class="multipart-file-path"
            type="button"
            :title="file.path"
            @click="chooseMultipartFile(index)"
          >
            {{ fileNameFromPath(file.path) }}
          </button>
          <button
            class="icon-button"
            type="button"
            :aria-label="`Remover arquivo ${index + 1}`"
            :title="`Remover arquivo ${index + 1}`"
            @click="removeMultipartFile(index)"
          >
            <svg viewBox="0 0 16 16" aria-hidden="true">
              <path d="M3 5h10M6 5V3.5h4V5m-5.5 0 .6 8h5.8l.6-8M7 7.5v3.5m2-3.5v3.5" />
            </svg>
          </button>
        </div>

        <p v-if="multipartFiles.length === 0" class="muted">Nenhum arquivo selecionado.</p>
      </div>

      <div v-else class="body-empty-state">
        Esta request não possui body.
      </div>

      <p v-if="bodyError" class="field-error">{{ bodyError }}</p>
      <p v-if="formError" class="field-error">{{ formError }}</p>
    </section>

    <section
      v-else
      id="request-assertions-panel"
      class="editor-section assertions-section"
      role="tabpanel"
    >
      <div class="section-heading">
        <div>
          <h3>Assertions</h3>
          <p>Verificações locais executadas depois que a resposta chegar.</p>
        </div>
        <button type="button" @click="addAssertion">Adicionar</button>
      </div>

      <div v-if="localRequest.assertions.length" class="assertion-list">
        <div v-for="(assertion, index) in localRequest.assertions" :key="index" class="assertion-row">
          <CustomSelect
            :model-value="assertion.type"
            :options="assertionTypeOptions"
            :label="`Tipo da assertion ${index + 1}`"
            @update:model-value="updateAssertionType(index, $event)"
          />

          <input
            v-if="assertion.type === 'statusEquals'"
            :value="assertion.expected"
            type="number"
            min="100"
            max="599"
            placeholder="200"
            :aria-label="`Status esperado da assertion ${index + 1}`"
            @input="updateAssertionField(index, 'expected', $event)"
          />
          <template v-else-if="assertion.type === 'headerContains'">
            <input
              :value="assertion.name"
              type="text"
              placeholder="Nome do header"
              :aria-label="`Nome do header da assertion ${index + 1}`"
              @input="updateAssertionField(index, 'name', $event)"
            />
            <input
              :value="assertion.value"
              type="text"
              placeholder="Valor esperado"
              :aria-label="`Valor do header da assertion ${index + 1}`"
              @input="updateAssertionField(index, 'value', $event)"
            />
          </template>
          <input
            v-else
            :value="assertion.value"
            type="text"
            placeholder="Texto esperado no body"
            :aria-label="`Texto esperado da assertion ${index + 1}`"
            @input="updateAssertionField(index, 'value', $event)"
          />

          <button
            class="icon-button"
            type="button"
            :aria-label="`Remover assertion ${index + 1}`"
            :title="`Remover assertion ${index + 1}`"
            @click="removeAssertion(index)"
          >
            <svg viewBox="0 0 16 16" aria-hidden="true">
              <path d="M3 5h10M6 5V3.5h4V5m-5.5 0 .6 8h5.8l.6-8M7 7.5v3.5m2-3.5v3.5" />
            </svg>
          </button>
        </div>
      </div>
      <p v-else class="muted">Nenhuma assertion configurada.</p>
    </section>
  </form>
</template>

<style scoped>
.request-editor {
  display: flex;
  flex-direction: column;
  gap: 18px;
  min-height: 100%;
}

.request-bar {
  display: flex;
  align-items: stretch;
  width: 100%;
  min-height: 36px;
  position: relative;
  z-index: 20;
  overflow: visible;
  border: 1px solid var(--color-border-strong);
  border-radius: 6px;
  background: var(--color-surface-2);
}

.request-url {
  min-width: 0;
  min-height: 34px;
  border: 0;
  border-radius: 0;
  background: transparent;
}

.request-url {
  flex: 1 1 auto;
  border-left: 1px solid var(--color-border);
  border-right: 1px solid var(--color-border);
  padding-right: 10px;
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 12px;
}

.request-url:focus {
  outline: none;
}

.request-method-custom-select {
  flex: 0 0 96px;
}

.request-bar .request-method-custom-select :deep(.custom-select-trigger) {
  height: 34px;
  border: 0;
  border-radius: 0;
  background: transparent;
}

.request-bar .request-method-custom-select :deep(.custom-select-trigger:hover:not(:disabled)) {
  border: 0;
  background: transparent;
}

.request-bar .primary-action {
  flex: 0 0 auto;
  min-height: 34px;
  border: 0;
  border-radius: 0 5px 5px 0;
  padding: 0 16px;
}

.request-bar .primary-action:hover:not(:disabled) {
  border: 0;
}

.request-method-custom-select,
.request-url,
.request-bar .primary-action {
  min-width: 0;
}

.editor-section {
  display: flex;
  flex-direction: column;
  gap: 10px;
  min-width: 0;
  padding-top: 2px;
}

.body-editor-section {
  flex: 1 1 auto;
  min-height: 0;
  gap: 8px;
}

.form-body-editor {
  display: flex;
  flex: 1 1 auto;
  min-height: 0;
  flex-direction: column;
  gap: 12px;
  overflow-y: auto;
}

.form-body-heading,
.multipart-files-heading {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}

.form-body-heading h3,
.multipart-files-heading h3 {
  margin: 0;
}

.form-body-heading p,
.multipart-files-heading p {
  margin: 4px 0 0;
  color: var(--color-text-muted);
  font-size: 12px;
}

.multipart-file-row {
  display: grid;
  grid-template-columns: 24px minmax(120px, 0.8fr) minmax(0, 2fr) 28px;
  gap: 8px;
  align-items: center;
}

.multipart-file-path {
  min-width: 0;
  overflow: hidden;
  border: 1px solid var(--color-border-strong);
  padding: 6px 10px;
  color: var(--color-text-muted);
  background: var(--color-surface-2);
  font-size: 12px;
  text-align: left;
  text-overflow: ellipsis;
}

.multipart-file-path:hover:not(:disabled) {
  border-color: var(--color-brand);
  color: var(--color-text);
  background: var(--color-surface-2);
}

.body-empty-state {
  display: grid;
  min-height: 220px;
  flex: 1 1 auto;
  place-items: center;
  color: var(--color-text-muted);
  background: #0d0f12;
  font-size: 12px;
}

.composer-tabs {
  display: flex;
  min-width: 0;
  overflow-x: auto;
  border-bottom: 1px solid var(--color-border);
}

.composer-tab {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  min-height: 36px;
  border: 0;
  border-bottom: 2px solid transparent;
  border-radius: 0;
  padding: 0 10px;
  color: var(--color-text-muted);
  background: transparent;
  font-size: 12px;
  font-weight: 700;
}

.composer-tab:hover:not(:disabled) {
  border-color: transparent;
  color: var(--color-text);
  background: transparent;
}

.composer-tab-active {
  border-bottom-color: var(--color-brand);
  color: var(--color-text);
}

.composer-tab span {
  color: var(--color-text-subtle);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 11px;
  font-weight: 600;
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

.section-heading p {
  margin: 4px 0 0;
  color: var(--color-text-muted);
  font-size: 13px;
}

.auth-section {
  max-width: 760px;
}

.assertions-section {
  max-width: 900px;
}

.assertion-list {
  display: grid;
  gap: 8px;
}

.assertion-row {
  display: grid;
  grid-template-columns: minmax(170px, 0.9fr) minmax(120px, 1fr) minmax(0, 1.2fr) 28px;
  gap: 8px;
  align-items: center;
}

.assertion-row .custom-select,
.assertion-row input {
  min-width: 0;
}

.auth-type-select {
  width: 150px;
  flex: 0 0 150px;
}

.auth-empty {
  border: 1px solid var(--color-border);
  border-radius: 6px;
  padding: 14px;
  color: var(--color-text-muted);
  background: var(--color-surface-2);
  font-size: 12px;
}

.auth-fields {
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-width: 620px;
}

.auth-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  align-items: end;
}

.auth-grid label:last-of-type {
  grid-column: 1 / -1;
}

.auth-fields label {
  display: flex;
  flex-direction: column;
  gap: 5px;
}

.auth-fields label span {
  color: var(--color-text-muted);
  font-size: 11px;
  font-weight: 700;
}

.auth-note {
  margin: 0;
  color: var(--color-text-subtle);
  font-size: 11px;
}

input {
  box-sizing: border-box;
  width: 100%;
  min-height: 34px;
  border: 1px solid var(--color-border-strong);
  border-radius: 6px;
  padding: 6px 10px;
  font: inherit;
  background: var(--color-surface-2);
  color: var(--color-text);
}

input::placeholder {
  color: var(--color-text-subtle);
}

input[type="checkbox"] {
  width: 18px;
  height: 18px;
  min-height: 18px;
  margin: 0 6px 0 0;
}

button {
  min-height: 34px;
  border: 1px solid var(--color-border-strong);
  border-radius: 6px;
  padding: 6px 12px;
  font: inherit;
  cursor: pointer;
  background: var(--color-surface-2);
  color: var(--color-text);
  white-space: nowrap;
}

button:hover:not(:disabled) {
  border-color: var(--color-border-strong);
  background: transparent;
  color: var(--color-text-muted);
}

button:disabled {
  cursor: wait;
  opacity: 0.65;
}

.primary-action {
  border-color: var(--color-brand-strong);
  color: #071315;
  background: var(--color-brand);
  font-weight: 700;
}

.primary-action:hover:not(:disabled) {
  border-color: var(--color-brand-strong);
  color: #071315;
  background: var(--color-brand);
}

.body-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  min-height: 28px;
}

.body-toolbar-context {
  display: inline-flex;
  align-items: center;
  min-width: 0;
  gap: 7px;
  color: var(--color-text-muted);
  font-size: 11px;
}

.body-toolbar-mark {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--color-brand);
  box-shadow: 0 0 0 3px rgb(98 217 220 / 10%);
}

.body-toolbar-title {
  color: var(--color-text);
  font-weight: 700;
}

.body-toolbar-state {
  color: var(--color-text-subtle);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 10px;
}

.body-format-menu {
  position: relative;
}

.body-format-trigger,
.prettify-button {
  min-height: 28px;
  border: 0;
  border-radius: 4px;
  padding: 3px 6px;
  background: transparent;
  font-size: 12px;
}

.body-format-trigger {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  color: var(--color-brand-warm);
  font-weight: 700;
}

.body-format-trigger:hover:not(:disabled) {
  color: #f8fafc;
  background: var(--color-surface-2);
}

.body-format-trigger span {
  display: none;
}

.body-format-chevron {
  width: 12px;
  height: 12px;
  fill: none;
  stroke: currentColor;
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 1.3;
}

.prettify-button {
  color: #94a3b8;
}

.prettify-button:hover:not(:disabled) {
  color: #f8fafc;
  background: var(--color-surface-2);
}

.prettify-button:disabled {
  cursor: not-allowed;
  opacity: 0.45;
}

.body-format-popover {
  position: absolute;
  z-index: 5;
  top: calc(100% + 6px);
  right: 0;
  display: grid;
  width: 224px;
  gap: 9px;
  padding: 8px;
  border: 1px solid var(--color-border-strong);
  border-radius: 8px;
  background: var(--color-surface-1);
  box-shadow: 0 12px 24px rgb(0 0 0 / 28%);
}

.body-format-popover-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 2px 7px 5px;
  border-bottom: 1px solid var(--color-border);
  color: var(--color-text);
  font-size: 11px;
  font-weight: 700;
}

.body-format-popover-heading span:last-child {
  color: var(--color-text-subtle);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 9px;
  font-weight: 500;
}

.body-format-group {
  display: grid;
  gap: 2px;
}

.body-format-group-label {
  padding: 2px 7px 3px;
  color: var(--color-text-subtle);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.05em;
}

.body-format-option {
  display: grid;
  grid-template-columns: 18px minmax(0, 1fr) auto;
  align-items: center;
  gap: 6px;
  min-height: 28px;
  border: 0;
  border-radius: 4px;
  padding: 4px 7px;
  color: var(--color-text-muted);
  background: transparent;
  font-size: 12px;
  text-align: left;
}

.body-format-option:hover:not(:disabled),
.body-format-option-active {
  color: var(--color-brand);
  background: rgb(98 217 220 / 10%);
}

.body-format-option:disabled {
  cursor: not-allowed;
  opacity: 0.9;
}

.body-format-check {
  width: 14px;
  height: 14px;
  fill: none;
  stroke: var(--color-brand);
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 1.5;
}

.body-format-icon {
  width: 15px;
  height: 15px;
  fill: none;
  stroke: currentColor;
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 1.2;
}

.body-code-editor {
  position: relative;
  flex: 1 1 auto;
  min-height: 220px;
  overflow: hidden;
  background: #0d0f12;
  color: #e2e8f0;
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 12px;
  line-height: 1.6;
}

.body-editor-gutter,
.body-code-highlight,
.body-code-input {
  position: absolute;
  inset: 0;
}

.body-editor-gutter {
  z-index: 1;
  right: auto;
  width: 36px;
  overflow: hidden;
  padding-top: 10px;
  color: #475569;
  user-select: none;
}

.body-gutter-content {
  min-height: 100%;
}

.body-gutter-line {
  position: relative;
  display: flex;
  height: 1.6em;
  align-items: center;
  justify-content: flex-end;
  padding-right: 7px;
}

.body-gutter-fold {
  position: absolute;
  left: 7px;
  width: 10px;
  height: 10px;
  fill: none;
  stroke: #94a3b8;
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 1.3;
}

.body-code-highlight,
.body-code-input {
  box-sizing: border-box;
  margin: 0;
  border: 0;
  padding: 10px 14px 10px 48px;
  font: inherit;
  line-height: inherit;
  white-space: pre;
  overflow-wrap: normal;
}

.body-code-highlight {
  z-index: 0;
  max-height: none;
  overflow: auto;
  background: #0d0f12;
  color: #e2e8f0;
  pointer-events: none;
  scrollbar-width: none;
}

.body-code-highlight::-webkit-scrollbar,
.body-code-input::-webkit-scrollbar {
  display: none;
}

.body-code-input {
  z-index: 2;
  overflow: auto;
  outline: none;
  color: transparent;
  caret-color: var(--color-brand);
  background: transparent;
  white-space: pre;
  overflow-wrap: normal;
  scrollbar-width: none;
}

.body-code-input:focus {
  outline: none;
}

:deep(.token-key) {
  color: #60a5fa;
}

:deep(.token-number) {
  color: #c084fc;
}

:deep(.token-string) {
  color: #fb923c;
}

:deep(.token-literal) {
  color: #c084fc;
}

.body-code-editor-error {
  box-shadow: inset 2px 0 0 var(--color-danger);
}

.pair-row {
  display: grid;
  grid-template-columns: 24px minmax(0, 1fr) minmax(0, 2fr) auto;
  gap: 8px;
  align-items: center;
}

.params-table {
  overflow: hidden;
  border: 1px solid var(--color-border);
  border-radius: 6px;
}

.params-table-header,
.parameter-row {
  display: grid;
  grid-template-columns: 32px minmax(0, 1fr) minmax(0, 2fr) 34px;
  align-items: center;
}

.params-table-header {
  min-height: 24px;
  border-bottom: 1px solid var(--color-border);
  color: var(--color-text-muted);
  background: var(--color-surface-1);
  font-size: 11px;
}

.params-table-header span {
  padding: 0 8px;
}

.parameter-row {
  min-height: 28px;
  border-bottom: 1px solid var(--color-border);
}

.parameter-row:last-child {
  border-bottom: 0;
}

.parameter-row > input[type="checkbox"] {
  justify-self: center;
  margin: 0;
}

.parameter-cell {
  min-height: 28px;
  border: 0;
  border-left: 1px solid var(--color-border);
  border-radius: 0;
  padding: 4px 8px;
  background: transparent;
  font-size: 12px;
}

.parameter-cell:focus {
  outline: 1px solid var(--color-brand);
  outline-offset: -1px;
}

.icon-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  min-height: 28px;
  border: 0;
  border-radius: 0;
  padding: 0;
  color: var(--color-text-subtle);
  background: transparent;
}

.icon-button:hover:not(:disabled) {
  color: var(--color-danger);
  background: transparent;
}

.icon-button svg {
  width: 15px;
  height: 15px;
  fill: none;
  stroke: currentColor;
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 1.2;
}

.muted {
  margin: 0;
  color: var(--color-text-muted);
}

.field-error {
  margin: 0;
  color: var(--color-danger);
}

.field-success {
  margin: 0;
  color: var(--color-success);
}

@media (max-width: 700px) {
  .pair-row {
    grid-template-columns: 1fr;
  }

  .request-bar {
    min-width: 0;
  }

  .request-method-custom-select {
    flex-basis: 76px;
  }

  .request-bar .primary-action {
    padding: 0 10px;
  }

  .auth-grid {
    grid-template-columns: 1fr;
  }

  .assertion-row {
    grid-template-columns: 1fr 28px;
  }

  .assertion-row .custom-select,
  .assertion-row input {
    grid-column: auto;
  }

  .auth-grid label:last-of-type {
    grid-column: auto;
  }

  .form-body-heading,
  .multipart-files-heading {
    flex-direction: column;
  }

  .multipart-file-row {
    grid-template-columns: 24px minmax(0, 1fr) 28px;
  }

  .multipart-file-path {
    grid-column: 2 / -1;
  }

  input[type="checkbox"] {
    justify-self: start;
  }

  .primary-action {
    width: 100%;
  }
}
</style>
