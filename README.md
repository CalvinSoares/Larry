# Larry API Client

Cliente de API local, offline-first e open-source para montar, executar e investigar requisições HTTP sem depender de backend, login ou telemetria.

O Larry está sendo construído para manter o frontend simples e separar a interface do núcleo local em Rust. A comunicação entre Vue e o core acontece por Tauri IPC, mantendo acesso a rede, filesystem e secrets fora dos componentes Vue.

> Projeto em desenvolvimento: a versão atual é um MVP inicial com editor HTTP, execução local, collections YAML e leitura do estado Git.

## Funcionalidades atuais

- diagnóstico local com nome, versão, sistema operacional e arquitetura;
- editor de método, URL, nome, query params, headers e body JSON/texto;
- execução HTTP/HTTPS no core Rust, com timeout e limite de resposta;
- resposta formatada com status, headers, body, tempo e tamanho medidos;
- estados explícitos para trace e diagnóstico de rede ainda indisponíveis;
- criação, duplicação, seleção e remoção de requests locais;
- salvar e carregar collections YAML legíveis e versionáveis pelo Git;
- inspeção read-only do branch, status e diff da collection no Git;
- onboarding local de primeiro uso;
- prévia de importação de collections Postman v2.1 com avisos e sanitização básica;
- environments YAML separados das collections, com placeholders públicos e secret references;
- secrets armazenados localmente pelo mecanismo nativo do sistema e nunca retornados para a UI;
- trace HTTP inicial com DNS preflight, TTFB, download, total, IPs resolvidos e versão HTTP;
- diagnóstico inicial por camada de aplicação para status HTTP e falhas de execução;
- histórico local SQLite com replay sanitizado e comparação de execuções;
- modal acessível para ações destrutivas, sem `window.alert`, `window.confirm` ou `window.prompt`;
- execução local com Vue 3, TypeScript, Vite e Tauri 2;
- interface sem backend obrigatório, login ou telemetria.

## Stack

- Vue 3 com `<script setup lang="ts">`;
- TypeScript e Vite;
- Tauri 2;
- Rust;
- caminho preparado para Tokio, SQLite, arquivos locais e armazenamento seguro de credenciais.

## Requisitos

- Node.js compatível com o Vite instalado;
- npm ou pnpm compatível com o lockfile;
- Rust e Cargo;
- dependências de desenvolvimento do Tauri 2 para o sistema operacional.

## Desenvolvimento

Instale as dependências com npm no ambiente atual:

```bash
npm install
```

Inicie apenas o frontend:

```bash
npm run dev
```

Execute o aplicativo Tauri durante o desenvolvimento:

```bash
npm run tauri dev
```

Valide o build do frontend:

```bash
npm run build
```

O projeto mantém `pnpm-lock.yaml` para quem preferir pnpm. Nesse caso, os equivalentes são `pnpm install`, `pnpm dev`, `pnpm tauri dev` e `pnpm build`.

## Estrutura

```text
src/             interface Vue e ponto de entrada do frontend
src-tauri/       core Rust, comandos Tauri e configuração do desktop
public/assets/   assets públicos, incluindo o favicon do Larry
docs/            documentação de decisões e evolução do projeto
```

## Princípios do projeto

- funcionar localmente e preservar o fluxo offline;
- não registrar ou exportar secrets;
- manter o erro técnico original junto de uma explicação amigável;
- declarar a origem das métricas: medida, observada, inferida ou indisponível;
- validar URLs, paths, tamanhos e permissões no Rust;
- permitir cancelamento em operações de rede, streams e profiling;
- manter collections, migrations e eventos versionados e legíveis fora do app;
- manter secrets fora de collections, environments versionados, histórico, logs e exportações;
- usar capabilities Tauri mínimas e específicas.

## Roadmap inicial

- cancelamento de requests e tratamento de erro por camada;
- collections importáveis e exportáveis;
- persistência local versionada com SQLite e arquivos editáveis;
- precedência de variables, detecção de secrets e seletor nativo de environments;
- histórico de execuções com `run_id`, timestamp monotônico, fase, fonte e confiança;
- cobertura de testes para caminho feliz, timeout, cancelamento e erro;
- suporte progressivo a HTTP/2, streams e métricas com provenance.

## Contribuindo

Issues, discussões e pull requests são bem-vindos. Antes de abrir uma contribuição:

1. descreva o problema e o comportamento esperado;
2. explique o impacto no fluxo local/offline;
3. inclua testes ou uma fixture reproduzível quando aplicável;
4. não envie secrets reais, tokens, cookies, collections privadas ou logs sensíveis;
5. atualize a documentação quando alterar eventos, schema, collections ou capabilities.

## Licença

O projeto está sendo preparado para publicação open-source. A licença será definida antes da primeira distribuição pública; até lá, não presuma permissões de reutilização além das concedidas pelo mantenedor.

## Setup recomendado

- [VS Code](https://code.visualstudio.com/);
- [Vue - Official](https://marketplace.visualstudio.com/items?itemName=Vue.volar);
- extensão [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode);
- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).
