# PLAN: arquitetura e roadmap

## 1. Objetivo

Construir um executável desktop local-first, sem backend obrigatório, que una API testing, debugging de rede, replay, observabilidade e aprendizado de protocolos.

## 1.1 Modo de construção adotado

O projeto será construído em fases, mas cada tarefa pode conter um lote maior de mudanças relacionadas. O agente implementa o lote quando solicitado pelo usuário, executa os testes e encerra com uma explicação completa para estudo.

Cada entrega deve informar:

- escopo e objetivo;
- arquivos criados, alterados ou removidos;
- decisões de arquitetura;
- testes executados;
- limitações e riscos;
- conceitos novos para o usuário;
- comandos de revisão e commit granular.

O usuário continua revisando e aprendendo o código. Uma nova fase só começa depois da confirmação do usuário.

Boas práticas são aplicadas de forma pragmática: DRY, SRP, Clean Architecture, SOLID, KISS, YAGNI, segurança por padrão, testes reproduzíveis e contratos versionados. Não serão criadas abstrações cerimoniais antes de existir uma necessidade real.

## 2. Stack

### Frontend

- Vue 3;
- TypeScript;
- Vite;
- Composition API;
- Single-File Components (`.vue`);
- CSS simples no início;
- Pinia somente quando o estado compartilhado justificar;
- CodeMirror/Monaco apenas quando chegarmos aos editores avançados.

O padrão visual está documentado em [design.md](./design.md). A aplicação seguirá tema escuro, superfícies sólidas, tokens compartilhados e uma hierarquia de workspace inspirada em ferramentas de desenvolvimento. A logo existente em `public/assets/larry-platypus.png` será usada como marca do Larry, sem transformar o restante da interface em uma composição decorativa.

### Desktop/core

- Tauri 2;
- Rust;
- Tokio;
- SQLite para histórico/cache;
- arquivos locais para collections;
- Credential Manager/Keychain/libsecret para secrets.

## 3. Arquitetura

```text
Vue UI
  ↓ commands/events
Tauri boundary
  ↓ DTOs e ports
Rust domain + execution
  ├─ transport-http
  ├─ transport-realtime
  ├─ transport-grpc
  ├─ network-diagnostics
  ├─ profiling
  ├─ scripting
  ├─ persistence
  └─ secrets
```

O frontend não deve implementar requests nem acessar secrets diretamente. Ele edita modelos, dispara comandos e renderiza eventos.

### Estrutura Rust sugerida

```text
src-tauri/
  src/
    domain/
    execution/
    transport_http/
    transport_realtime/
    transport_grpc/
    diagnostics/
    profiling/
    persistence/
    secrets/
    commands/
```

Começar em um único crate é aceitável. Separar em crates só quando as fronteiras forem compreendidas.

### Estrutura Vue sugerida

```text
src/
  app/
  components/
  composables/
  features/
    request-editor/
    response-viewer/
    collections/
    environments/
    history/
    network-inspector/
    replay-compare/
    performance/
    protocol-lab/
  types/
```

## 4. Eventos de execução

Uma execução deve emitir eventos incrementais:

```text
RunStarted
DnsStarted / DnsCompleted
ConnectStarted / ConnectCompleted
TlsStarted / TlsCompleted
ProtocolNegotiated
RequestHeadersSent
RequestBodyProgress
ResponseHeadersReceived
ResponseBodyProgress
DiagnosticProduced
RunCompleted / RunFailed / RunCancelled
```

Cada evento possui `run_id`, timestamp monotônico, fase, status, fonte e confiança. HTTP/2 e gRPC podem adicionar `connection_id` e `stream_id`.

## 5. Persistência e Git

Collections são arquivos versionáveis:

```text
my-api/
  api-client.yaml
  collections/
  environments/
  schemas/
  scripts/
  .gitignore
```

### Formato inicial implementado

O primeiro formato será YAML legível pelo Git:

```yaml
schemaVersion: 1
name: Minha API
requests:
  - id: health
    name: Health check
    method: GET
    url: http://localhost:3000/health
    query: []
    headers: []
    body: null
```

O Rust valida a versão do schema, o nome da collection e IDs não vazios ou duplicados. O carregamento tem limite de 5 MiB. O salvamento escreve primeiro em arquivo temporário e depois substitui o destino. A primeira UI recebe o caminho digitado pelo usuário; o seletor nativo de arquivos será adicionado depois sem mudar o contrato.

SQLite guarda histórico, índices, metadados e snapshots opcionais. Não deve ser a fonte obrigatória das collections.

Git deve ser integrado como capacidade local e opcional:

- status, branch e diff;
- diff semântico de URL, método, headers, body e assertions;
- alerta de secrets antes de export/commit;
- nenhum commit, push ou troca de branch automática;
- usar Git CLI allowlisted no MVP para preservar SSH e credential helpers;
- avaliar `git2` depois, caso seja necessário.

### Primeiro lote Git implementado

O Larry consulta o Git usando o executável local com argumentos allowlisted e sem shell:

- raiz do repositório;
- branch atual;
- `git status --short --branch`;
- diff textual da collection;
- limite de 1 MiB para saída;
- nenhum commit, push, checkout ou troca automática de branch.

O diff exibido ainda é textual. O diff semântico de método, URL, headers, body e assertions continua planejado para o próximo incremento.

### Onboarding e migração Postman

O primeiro uso terá um onboarding local com quatro caminhos:

1. importar collection Postman;
2. criar collection;
3. começar com uma request;
4. explorar o workspace sem configuração.

A primeira versão de importação deve aceitar arquivo JSON de collection Postman v2.1 e converter, com prévia antes de salvar:

- nome e descrição da collection;
- pastas e requests;
- método, URL, query, headers e body;
- autenticação que tenha mapeamento seguro para o modelo Larry;
- variáveis públicas quando o destino estiver definido.

O importador deve gerar um relatório de conversão. Scripts Postman, exemplos, mocks, eventos e recursos específicos não podem ser silenciosamente descartados. Cada item não convertido precisa aparecer como aviso com motivo e arquivo de origem.

Regras de segurança:

- nunca gravar tokens literais em collection sem confirmação explícita;
- separar environments e secrets do arquivo importado;
- criar nova collection por padrão, sem sobrescrever arquivos existentes;
- validar tamanho, schema e IDs antes de salvar;
- preservar o JSON original em memória somente durante a prévia, sem colocá-lo no histórico ou log.

O suporte inicial será de importação local. Login Postman, sincronização Postman Cloud, API keys Postman e execução dependente do Postman não fazem parte do MVP.

Referências oficiais: [exportação de dados Postman](https://learning.postman.com/docs/getting-started/importing-and-exporting/exporting-data), [importação de dados Postman](https://learning.postman.com/docs/getting-started/importing-and-exporting/importing-data/) e [schema de collections](https://learning.postman.com/docs/use/use-collections/collections-schemas).

### Primeiro lote de onboarding e importação implementado

- onboarding local aparece somente no primeiro uso e não exige conta;
- modal oferece importar Postman, criar collection, começar request ou explorar;
- importador Rust aceita collections Postman v2.1 por caminho local;
- prévia mostra nome, quantidade de requests e avisos antes de colocar dados no workspace;
- folders são representadas no nome da request e não são descartadas silenciosamente;
- auth, scripts, eventos, descriptions, variáveis e formatos de body não suportados geram avisos;
- headers e campos JSON com nomes sensíveis são substituídos por referências `{{secret.*}}`;
- collection importada fica em memória até o usuário salvar pelo fluxo file-first existente;
- nenhum JSON original é escrito em histórico, log ou arquivo automaticamente.

Limitações deliberadas deste lote: o caminho do arquivo ainda é digitado manualmente, environments e secrets ainda não resolvem placeholders, e a autenticação importada precisa ser revisada pelo usuário.

### Segundo lote de collections concluído

- collection agora mantém múltiplas requests em memória no `App.vue`;
- lista de requests com seleção da request ativa;
- criação de request com defaults seguros e sem secret literal;
- duplicação com novo ID para evitar colisões no schema;
- remoção protegida para não deixar a collection sem request;
- salvamento de todas as requests, em vez de somente a request aberta;
- carregamento seleciona a primeira request e preserva as demais;
- layout responsivo da lista com foco visível, truncamento de URL/nome e labels acessíveis;
- Git continua read-only e consulta o arquivo completo da collection.

A coleção ainda usa um caminho digitado manualmente. O seletor nativo, diff semântico e grupos/pastas continuam separados para os próximos lotes.

### Lote visual de sidebar concluído

- `CollectionSidebar.vue` concentra a navegação das requests;
- o editor central não renderiza mais a lista da collection;
- o workspace usa sidebar, editor e response inspector;
- persistência e Git permanecem no contexto da collection;
- o layout possui breakpoints para reduzir, empilhar e recolher regiões;
- a decoração nativa do Windows foi desativada;
- a topbar web possui controles reais de arrastar, minimizar, maximizar/restaurar e fechar via Tauri;
- os ícones de bundle foram regenerados a partir da marca do Larry;
- o response inspector possui abas reais para Body e Headers;
- Trace e Diagnostics sinalizam indisponibilidade sem apresentar métricas inventadas.

### Revisão do sistema visual do shell

As referências recentes do Bruno e a comparação com Insomnia e Postman revelaram uma diferença estrutural que precisa ser corrigida antes de ampliar a UI:

- o workspace deve ocupar a janela sem padding externo de página;
- o `body` não deve rolar durante o uso normal;
- sidebar, request e response devem ter scroll independente;
- buttons, inputs, selects, tabs e icon buttons precisam de contratos comuns de altura, baseline e estados;
- chevrons e setas devem vir de SVGs consistentes, não de caracteres Unicode;
- a validação desses padrões fica registrada em `.cursor/skills/ui-visual-system-validation/SKILL.md`.

Este lote atualizou o contrato visual e o processo de validação. A implementação do shell sem scroll global foi concluída na sequência, mantendo sidebar, request e response como regiões de trabalho independentes.

### Implementação do shell sem scroll global

- `app-shell` ocupa 100% da janela e impede scroll no documento;
- o workspace ocupa a altura restante depois da topbar e do diagnóstico;
- sidebar, editor e response possuem scroll vertical próprio;
- no desktop, as regiões são contíguas e separadas por divisores, sem cards flutuantes e vãos externos;
- botões e campos principais usam altura compacta e alinhamento comum;
- selects de método e body usam chevron SVG;
- o botão de adicionar request usa ícone SVG com label acessível.

### Reformulação de identidade: Larry Network Workbench

Esta reformulação é uma etapa de qualidade de produto, não uma nova capacidade de protocolo. Ela acontece antes de WebSocket, SSE e gRPC para impedir que novos recursos sejam adicionados a uma UI que já mistura navegação, configuração e observabilidade.

- a identidade deixa de ser uma barra genérica com marca central e passa a usar a assinatura `Larry / Local workbench`;
- a topbar mostra o contexto ativo da request, environment, estado e controles reais de janela;
- a barra lateral vira **Request rail**, com abas separadas para `Requests` e `Ferramentas locais`;
- arquivo da collection, environment e histórico saem da navegação primária e ficam disponíveis sem reduzir a área da árvore;
- o centro passa a ser o **Composer**, com método, URL, envio e abas `Params`, `Headers` e `Body`;
- a coluna de resposta passa a ser o **Inspector**, organizado como evidência da execução e não como card auxiliar;
- uma statusbar discreta exibe informações locais sem competir com a request;
- o modelo, o IPC e o core Rust permanecem inalterados neste lote.

As próximas telas de protocolo devem reutilizar a mesma estrutura: Composer específico do protocolo à esquerda e Inspector de execução à direita. Isso permite evoluir WebSocket, gRPC e Protocol Lab sem duplicar o shell.

### Ajuste do rail de ferramentas e topbar

- `Ferramentas locais` virou um launcher compacto com três entradas, em vez de renderizar três formulários longos em sequência;
- collection, environment e history abrem o modal compartilhado com conteúdo persistente enquanto o modal está fechado;
- o modal de histórico usa largura maior para comparação e listas locais;
- o texto `Pronto` foi removido da topbar: a execução é comunicada diretamente pelo botão `Enviar` e pelo Inspector;
- environment continua visível porque muda a execução da request; o estado genérico não adicionava decisão nova.

### Sistema de modal implementado

- `AppModal.vue` centraliza confirmação, informação, erro e ação destrutiva;
- o fluxo de remoção de request não usa mais `window.confirm`;
- modais têm foco inicial, foco preso, `Esc`, backdrop configurável e restauração de foco;
- templates e regras estão em `docs/design.md` e na skill de validação visual.

A timeline de rede e o seletor nativo de arquivos ainda serão implementados quando cada capacidade tiver comportamento real.

### Primeiro lote de environments e secrets implementado

- `EnvironmentFile` possui `schemaVersion`, nome e variáveis públicas ou referências secretas;
- environments são arquivos YAML separados das collections e limitados a 1 MiB;
- o salvamento usa arquivo temporário antes da substituição do destino;
- placeholders `{{baseUrl}}` e `{{secret.token}}` são resolvidos no core Rust antes da execução;
- valores públicos permanecem no YAML; valores secretos usam o `keyring` e o armazenamento nativo do sistema;
- o frontend consegue salvar e carregar a definição, mas nunca lê o valor existente de um secret;
- erros de resolução preservam a camada e a mensagem técnica sem revelar o valor secreto;
- valores secretos conhecidos são mascarados antes de uma falha de transporte chegar à UI;
- a seleção ativa do environment é enviada pelo Tauri IPC junto da request;
- testes cobrem schema, duplicidade, persistência YAML, resolução pública e referências ausentes.

Formato inicial:

```yaml
schemaVersion: 1
name: Local
variables:
  - name: baseUrl
    value: http://localhost:3000
    secretRef: null
  - name: accessToken
    value: null
    secretRef: accessToken
```

Limitações deliberadas: ainda não há precedência entre workspace/collection/request, detecção automática de secrets, seletor nativo de arquivos, confirmação específica ao trocar para produção ou remoção de credencial pela UI. O usuário precisa apagar a credencial pelo mecanismo do sistema ou por um próximo fluxo explícito; remover a referência YAML não apaga automaticamente o secret.

### Primeiro lote de trace HTTP e diagnóstico implementado

- o executor faz uma resolução DNS de preflight com timeout local de 30 segundos;
- a resposta registra fases `dns`, `ttfb`, `download` e `total` quando a medição está disponível;
- cada fase carrega `durationMs`, `provenance` e detalhe explicativo;
- a resposta registra os endereços resolvidos e a versão HTTP observada;
- TCP, TLS detalhado e reuso de conexão aparecem como `unavailable`, sem duração inventada;
- status HTTP 401, 403, 404, 4xx e 5xx recebem diagnóstico na camada de aplicação;
- falhas DNS e falhas do adapter HTTP carregam diagnóstico técnico separado da mensagem amigável;
- a UI mostra a timeline, provenance e diagnóstico no response inspector;
- respostas e erros continuam sem incluir valores secretos resolvidos.

Limitações deliberadas: a resolução DNS é um preflight observado e não necessariamente o mesmo lookup interno do reqwest; TTFB mede o intervalo até os headers ficarem disponíveis e não prova o tempo gasto exclusivamente no servidor; TCP, TLS/certificado/ALPN, connection reuse e redirects exigirão um adapter instrumentado e eventos próprios.

### Primeiro lote de histórico, replay e comparação implementado

- o core cria `history.sqlite3` no diretório local da aplicação;
- a tabela possui migration inicial versionada e limite de 1000 execuções;
- request e response são sanitizadas antes da persistência;
- bodies de response acima de 256 KiB são omitidos e marcados como truncados;
- a UI lista execuções locais com status, ambiente, tempo e tamanho;
- `Repetir` carrega a request sanitizada no editor para revisão antes de novo envio;
- duas execuções podem ser comparadas por status, headers, body e diferença de latência;
- falhas também entram no histórico sem interromper a request por indisponibilidade do banco local;
- nenhuma conta, sincronização ou backend é necessário.

Limitações deliberadas: o histórico atual mantém um resumo de comparação, não um diff visual linha a linha; requests com secrets literais são sanitizadas e podem exigir revisão antes do replay; bodies grandes não são armazenados; limpeza seletiva e exportação de incident capsule ficam para um lote posterior.

## 6. Escopo

### MVP

- HTTP/HTTPS com GET, POST, PUT, PATCH e DELETE;
- headers, query, JSON, texto e form-data;
- Bearer, Basic, API Key e cookies;
- environments e secret references;
- collections file-first;
- Git status/diff básico;
- import cURL;
- onboarding de primeiro uso com importação, criação e exploração;
- importação local de collections Postman v2.1;
- response viewer;
- histórico local;
- replay e comparação;
- trace DNS/connect/TLS/TTFB/download quando disponível;
- diagnóstico por camada;
- profiler seguro para 1/10/100/1000 requests;
- WebSocket e SSE básicos;
- modo educacional;
- gRPC unary dinâmico como vertical slice.

### V2

- gRPC Reflection e quatro padrões de streaming;
- GraphQL introspection/schema explorer;
- HTTP/2 connection/stream lab;
- Socket.IO;
- diff semântico avançado e merge assistance;
- CLI local para CI;
- OpenAPI import/export;
- Git commit/pull/push guiados e explícitos;
- backend opcional para sync e colaboração.

### Experimental

- HTTP/3/QUIC;
- MQTT;
- proxy/MITM;
- packet capture/ETW;
- diagnóstico baseado em evidências.

## 7. Fases de implementação

1. Preparação, projeto Tauri/Vue e mapa de diretórios.
2. Primeira ponte Tauri IPC.
3. Domínio e contratos de execução.
4. Primeira request HTTP local.
5. Editor Vue e response viewer.
6. Collections file-first e Git.
7. Fundação visual e padronização do workspace.
8. Revisão do shell, scroll por região e sistema de controles.
9. Environments e secrets.
10. Trace HTTP e diagnóstico.
11. Histórico, replay e comparação.
12. WebSocket, SSE e assertions.
13. gRPC dinâmico.
14. Profiler local.
15. Protocol Lab e modo educacional.
16. Hardening e distribuição.

Cada fase pode ser entregue em um ou mais lotes de implementação. O formato do relatório final e os critérios de validação ficam neste plano e no checklist do projeto.

