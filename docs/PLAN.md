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
schemaVersion: 2
name: Minha API
requests:
  - id: health
    name: Health check
    method: GET
    url: http://localhost:3000/health
    query: []
    headers: []
    body: null
folders: []
```

O Rust valida a versão do schema, o nome da collection, IDs de requests e IDs ou nomes de pastas. O carregamento migra `schemaVersion: 1` para a versão 2, mantendo requests antigas na raiz. O carregamento tem limite de 5 MiB. O salvamento escreve primeiro em arquivo temporário e depois substitui o destino. A UI recebe caminhos de collection e o seletor de arquivos multipart usa o plugin de diálogo do Tauri, mantendo a leitura dos arquivos no core Rust.

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

### Diff semântico da collection implementado

- o GitPanel mantém o status e o diff técnico original;
- a versão atual do YAML é comparada com a versão do mesmo arquivo em `HEAD`;
- mudanças de collection, folders e requests são apresentadas por campo;
- o resumo cobre nome, localização, método, URL, query params, headers, cookies, body, autenticação e assertions;
- requests e folders adicionados ou removidos aparecem como eventos próprios;
- valores de headers, cookies, autenticação, body e query sensível não são incluídos no diff semântico;
- URLs têm parâmetros sensíveis mascarados antes de chegar à interface;
- YAML inválido mantém o diff técnico disponível e marca apenas o diff semântico como indisponível.

Limitações deliberadas: a comparação atual usa `HEAD` como base, ainda não compara dois commits arbitrários, não entende conflitos de merge e não executa commit, pull ou push.

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

Limitações deliberadas deste lote: o caminho de origem do importador Postman ainda é digitado manualmente, environments e secrets ainda não resolvem placeholders, e a autenticação importada precisa ser revisada pelo usuário.

### Importação local de cURL implementada

- o workspace possui um modal para colar e analisar um comando cURL;
- o parser roda no Rust e não executa shell nem chama o binário cURL;
- método, URL, query, headers, cookies, Bearer, Basic Auth, User-Agent e Referer são convertidos quando representáveis;
- bodies JSON/texto, `--data-urlencode`, `--form` e arquivos multipart locais são convertidos para o modelo da request;
- `-G` move dados de formulário para a query e preserva a semântica de GET;
- opções sem representação segura recebem warnings explícitos, incluindo redirects, `--insecure`, proxy, certificados, seleção de protocolo e arquivos de body;
- a request importada recebe um novo ID local e abre no editor para revisão, sem execução automática ou salvamento automático;
- testes cobrem quotes, comandos multilinha, query, JSON, Bearer, multipart, caminhos Windows, `-G` e comandos inválidos.

Limitações deliberadas: aliases específicos de shell, expansão de variáveis do shell, arquivos de cookie, certificados, proxy, compressão, seleção explícita de HTTP/2 ou HTTP/3 e métodos não presentes no modelo atual não são reproduzidos. Credenciais importadas ficam no draft até revisão e devem migrar para secret references antes de salvar.

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

A collection agora usa o seletor nativo para escolher o arquivo. Diff semântico e merge assistance continuam separados para os próximos lotes.

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
- a barra lateral vira uma **Collection tree** única, sem abas de `Requests` e `Ferramentas locais`;
- o cabeçalho da collection concentra arquivo/Git e criação de requests por protocolo;
- environment fica na topbar e histórico fica no Inspector, sem ocupar a árvore;
- o centro passa a ser o **Composer**, com método, URL, envio e abas `Params`, `Headers` e `Body`;
- a coluna de resposta passa a ser o **Inspector**, organizado como evidência da execução e não como card auxiliar;
- uma statusbar discreta exibe informações locais sem competir com a request;
- o modelo, o IPC e o core Rust permanecem inalterados neste lote.

As próximas telas de protocolo devem reutilizar a mesma estrutura: Composer específico do protocolo à esquerda e Inspector de execução à direita. Isso permite evoluir WebSocket, gRPC e Protocol Lab sem duplicar o shell.

### Unificação da árvore e do Inspector

- as abas `Requests` e `Ferramentas locais` foram removidas da sidebar;
- o cabeçalho da collection abre o modal YAML e o menu `+` cria HTTP, WebSocket, gRPC, SSE ou importa cURL;
- environment abre pelo seletor da topbar e mantém o editor no modal compartilhado;
- o histórico aparece como seletor compacto no cabeçalho do Inspector e repete uma execução escolhida;
- Protocol Lab e Profiler são abas do Inspector, ao lado de Trace e Diagnostics;
- o texto `Pronto` foi removido da topbar: a execução é comunicada diretamente pelo botão `Enviar` e pelo Inspector;
- environment continua visível porque muda a execução da request; o estado genérico não adicionava decisão nova.

### Sistema de modal implementado

- `AppModal.vue` centraliza confirmação, informação, erro e ação destrutiva;
- o fluxo de remoção de request não usa mais `window.confirm`;
- modais têm foco inicial, foco preso, `Esc`, backdrop configurável e restauração de foco;
- templates e regras estão em `docs/design.md` e na skill de validação visual.

### Selects customizados e modal de Nova Request implementados

- `CustomSelect.vue` substitui os selects nativos do editor HTTP, painel gRPC e seletor de histórico;
- o componente usa trigger e popover com tokens, chevron SVG, check SVG, foco visível, fechamento externo e navegação por teclado;
- `NewRequestModal.vue` concentra HTTP, WebSocket, gRPC, SSE e importação de cURL no botão `+` da collection;
- requests HTTP criadas pelo modal entram na collection local com nome, método e URL definidos pelo usuário;
- WebSocket, gRPC e SSE reutilizam os painéis existentes até a persistência de nós de protocolo ser versionada;
- a API do modal não introduz backend, login, telemetria ou secret em fixture;
- a persistência de requests não HTTP continua planejada para lotes posteriores.

A timeline de rede ainda será ampliada quando cada capacidade tiver comportamento real.

### Pastas versionadas e menu contextual implementados

- `CollectionFile` agora usa `schemaVersion: 2` e suporta `folders` recursivas com requests aninhadas;
- collections `schemaVersion: 1` são migradas no carregamento, sem perder requests existentes;
- a validação Rust verifica IDs e nomes de pastas, além de IDs globais de requests;
- a sidebar renderiza requests de raiz, pastas, subpastas e contagem de requests;
- a expansão de pastas é local à sessão e não altera o arquivo até salvar;
- o menu contextual da collection funciona por clique no botão de ações ou clique direito no cabeçalho;
- a criação de pasta usa o modal compartilhado e salva a estrutura no próximo salvamento YAML;
- o menu da request permite mover a request para a raiz, uma pasta ou uma subpasta;
- o destino é escolhido por `CustomSelect`, com caminhos hierárquicos e retorno seguro por `Cancelar`;
- o painel de collection conta requests de raiz e aninhadas ao carregar ou salvar.
- folders permitem criar requests diretamente dentro delas e criar subpastas pelo menu contextual;
- o nome de uma pasta pode ser alterado pelo mesmo modal reutilizável, sem alterar suas requests;
- a remoção de folders é confirmada em modal, remove a subárvore inteira e bloqueia a operação se ela deixaria zero requests na collection;
- após remover a pasta ativa, a UI seleciona a primeira request executável restante.

Limitações deliberadas: ainda não há drag-and-drop, a criação de requests não HTTP continua abrindo painéis temporários e a importação Postman continua achatando o caminho da pasta no nome até o adapter de importação produzir a árvore nativa. A remoção protegida garante apenas a existência de uma request, não substitui um fluxo futuro de lixeira ou restauração.

### Seletor nativo e dirty state implementados

- o fluxo da collection usa o `tauri-plugin-dialog` para abrir arquivos `.yaml` e escolher o destino de salvamento;
- o caminho selecionado fica somente leitura na interface, evitando divergência entre o caminho exibido e o arquivo escolhido pelo sistema;
- salvar sem caminho abre primeiro o diálogo de destino e só marca a collection como salva depois que o Rust confirma a gravação;
- alterações em requests, folders e nome da collection marcam o workspace como não salvo;
- carregamento de um arquivo existente limpa o dirty state e associa o workspace ao caminho carregado;
- importações e collections novas permanecem explicitamente não salvas até o usuário escolher um destino;
- a sidebar mostra um indicador discreto de alterações não salvas e o modal file-first informa o estado atual.

Limitações deliberadas: trocar de arquivo não descarta alterações automaticamente, e os importadores Postman, environment e gRPC continuam com seus próprios fluxos de seleção até receberem o mesmo contrato nativo.

### Auto-save local opcional implementado

- a opção fica desativada por padrão e é persistida somente como preferência local do aplicativo;
- o usuário precisa escolher um arquivo antes de o auto-save poder executar;
- alterações são agrupadas por um debounce curto de 800 ms para evitar uma gravação por tecla;
- o salvamento automático usa o mesmo comando Rust e a mesma escrita temporária do salvamento manual;
- dirty state só é limpo depois da confirmação de sucesso do Rust;
- se uma alteração acontecer durante a gravação, ela não é considerada salva e outro ciclo é agendado;
- falhas mantêm a collection como não salva e preservam o erro técnico formatado.

Limitações deliberadas: não existe auto-save de collections sem caminho, versionamento automático, backup adicional por execução ou descarte automático ao trocar de arquivo.

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

Limitações deliberadas: ainda não há precedência entre workspace/collection/request, detecção automática de secrets, confirmação específica ao trocar para produção ou remoção de credencial pela UI. O usuário precisa apagar a credencial pelo mecanismo do sistema ou por um próximo fluxo explícito; remover a referência YAML não apaga automaticamente o secret.

### Primeiro lote de autenticação HTTP implementado

- requests agora possuem um campo `auth` opcional e retrocompatível;
- Bearer Token, Basic Auth e API Key podem ser configurados na aba `Auth`;
- API keys podem ser enviadas em header ou query string;
- placeholders de environment e secret são resolvidos no core Rust antes da execução;
- o histórico sanitiza tokens, passwords e valores de API key;
- collections antigas sem `auth` continuam carregando como `auth: null`;
- OAuth, form-data e importação de autenticação do Postman continuam fora deste lote.

### Segundo lote de cookies HTTP implementado

- requests agora possuem uma lista `cookies` opcional e retrocompatível;
- cookies habilitados são enviados como um header `Cookie` único;
- nomes e valores passam por validação antes do envio;
- placeholders de environment e secret são resolvidos no core Rust;
- valores de cookies configurados são redigidos no histórico local;
- collections antigas sem `cookies` continuam carregando com uma lista vazia;
- gerenciamento automático de cookies recebidos pelo servidor ainda não faz parte deste lote.

### Terceiro lote de bodies HTTP implementado

- `application/x-www-form-urlencoded` usa campos chave/valor habilitados;
- `multipart/form-data` combina campos de texto e arquivos locais;
- o seletor nativo de arquivos usa `tauri-plugin-dialog`;
- o Rust lê os arquivos durante a execução e limita cada arquivo a 20 MiB;
- caminhos e valores de formulário passam pela resolução de environment e secret;
- o histórico redige valores sensíveis e caminhos de arquivos;
- collections antigas com JSON ou texto continuam compatíveis;
- cookie jar automático e persistência de `Set-Cookie` permanecem fora deste lote.

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

### Cancelamento de request HTTP implementado

- cada execução HTTP manual recebe um `run_id` local e temporário;
- o frontend pode sinalizar o cancelamento pelo Tauri IPC enquanto a request está em execução;
- o core mantém um controlador por execução e cancela o future HTTP sem usar shell ou privilégio elevado;
- o botão de envio muda para `Cancelar` durante a execução e para `Cancelando...` enquanto o sinal é processado;
- o erro de cancelamento mantém `kind`, mensagem amigável e diagnóstico técnico;
- requests canceladas continuam passando pela sanitização antes de entrar no histórico;
- IDs duplicados são rejeitados para evitar cancelar uma execução diferente;
- testes cobrem IDs duplicados e o caminho de cancelamento antes do início do transporte.

Limitações deliberadas: o cancelamento interrompe a operação local, mas não desfaz efeitos que já tenham sido aceitos pelo servidor; cancelamento de WebSocket, SSE e streams gRPC continua pertencendo aos respectivos managers; o timeout HTTP ainda usa o limite local fixo de 30 segundos.

### Fixtures locais e falhas HTTP cobertas

- o executor possui fixtures TCP locais para respostas 401, 403 e 500;
- testes cobrem conexão recusada, timeout e falha de resolução DNS;
- o timeout de produção continua em 30 segundos, mas o executor interno aceita um limite controlado para testes determinísticos;
- nenhum teste depende de uma API externa ou envia dados para a internet;
- diagnósticos de aplicação, transporte e DNS continuam separados por camada e com provenance observada.

Limitações deliberadas: ainda não existe uma suíte end-to-end do executável empacotado; os fixtures validam o core Rust e o contrato de erro, enquanto o smoke test completo do Tauri permanece como etapa de distribuição.

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

### Primeiro lote de WebSocket implementado

- o core Rust aceita endpoints `ws://` e `wss://`;
- o handshake pode receber headers habilitados e valida nome, valor e quantidade antes da conexão;
- cada conexão recebe um `session_id` local e permanece viva até fechamento do peer, fechamento explícito ou falha;
- comandos Tauri separados abrem a sessão, enviam texto e solicitam fechamento;
- mensagens recebidas e enviadas, abertura, fechamento e erros são emitidos como eventos `websocket_event`;
- mensagens binárias são contabilizadas e identificadas sem transformar bytes em texto silenciosamente;
- mensagens de texto possuem limite local de 4 MiB e o painel mantém no máximo 200 eventos visíveis;
- o painel local permite conectar, enviar com `Ctrl + Enter`, fechar e limpar a sessão;
- o WebSocket não entra no histórico HTTP nem é salvo automaticamente em collection neste lote.

Limitações deliberadas: ainda não há headers editáveis no painel, cookies/auth integrados ao modelo, envio binário, reconexão automática, persistência de mensagens, ping configurável, métricas de handshake, Socket.IO ou proxy. A conexão vive no processo local e o fechamento do painel não encerra automaticamente a sessão mantida pelo componente.

### Primeiro lote de SSE implementado

- o core Rust abre streams SSE por URLs `http://` e `https://`;
- o request envia `Accept: text/event-stream` por padrão e aceita headers habilitados;
- cada stream recebe um `session_id` local e pode ser fechado por comando Tauri;
- o parser interpreta `event`, múltiplas linhas `data`, `id`, `retry` e comentários de keep-alive;
- eventos, abertura, fechamento e falhas são emitidos pelo evento Tauri `sse_event`;
- cada evento possui tamanho, timestamp e informação técnica separada da mensagem de erro;
- o painel local mostra conexão, content type observado, nome do evento, ID, tamanho e payload;
- o stream não é persistido no histórico HTTP e não reconecta automaticamente neste lote.

Limitações deliberadas: ainda não há headers editáveis no painel, cookies/auth integrados ao modelo, reconexão automática, uso de `Last-Event-ID`, replay, fila offline, métricas de chunks ou proxy. Um content type diferente de `text/event-stream` é observado e mostrado, mas não bloqueia a leitura, porque alguns servidores enviam parâmetros ou tipos compatíveis.

### Primeiro lote de assertions HTTP implementado

- requests podem declarar verificações de status HTTP, conteúdo de header e conteúdo textual do body;
- as verificações são dados tipados, não scripts, e são avaliadas no core Rust depois da leitura completa da resposta;
- referências de environment podem ser usadas nos valores esperados de headers e body;
- o response inspector mostra a quantidade aprovada, o resultado de cada verificação e os valores esperado e observado;
- requests antigas sem o campo `assertions` continuam compatíveis e carregam uma lista vazia;
- resultados e definições passam pela sanitização do histórico antes de serem persistidos.

Limitações deliberadas: ainda não há JSONPath, comparadores numéricos além de status exato, assertions compostas, scripts, snapshots ou execução de JavaScript. O match de header e body é literal, com nome de header sem diferenciação entre maiúsculas e minúsculas. Valores secretos devem continuar usando referências de environment para que a redaction seja possível.

### Primeiro lote de gRPC dinâmico implementado

- arquivos `.proto` locais são validados no Rust por extensão, existência, tipo e limite de 5 MiB;
- `protox` compila o contrato em descriptors sem gerar código de serviço estático;
- `prost-reflect` descobre services, métodos, tipos de entrada/saída e flags de streaming em runtime;
- o painel local permite selecionar o service e método descobertos e editar um body JSON;
- métodos unary são enviados pelo adapter Tonic com `DynamicMessage` e retornados como JSON;
- metadados ASCII habilitados podem acompanhar a chamada;
- erros mantêm `kind`, mensagem amigável e detalhe técnico original;
- o fluxo não exige backend, login, sincronização ou gravação automática em collections/histórico neste lote.

Limitações deliberadas: Reflection, client/server/bidirectional streaming, metadados binários, mTLS customizado, certificados próprios, importação de dependências distribuídas em vários diretórios, histórico gRPC e persistência em collections ficam para lotes posteriores. O tempo exibido é a duração observada da chamada depois que o canal é criado; ainda não é um trace separado de DNS, TCP, TLS e HTTP/2.

### Primeiro lote de gRPC Reflection implementado

- o cliente pode consultar o serviço padrão de gRPC Reflection sem importar um `.proto` local;
- o core lista services, solicita descriptors por símbolo e monta um `DescriptorPool` em memória;
- dependências repetidas são deduplicadas por nome de arquivo antes da interpretação;
- a descoberta possui limite de 512 services, limite agregado de 16 MiB de descriptors e timeout local de 15 segundos;
- metadata ASCII habilitada pode acompanhar a consulta de Reflection, mantendo a validação existente;
- o painel gRPC permite informar endpoint, host opcional, consultar o contrato e selecionar service e método descobertos;
- métodos unary podem executar usando Reflection sem gravar o contrato recebido no filesystem, collection ou histórico;
- errors de conexão, timeout, resposta do servidor e descriptor inválido preservam `kind`, mensagem amigável e detalhe técnico.

Limitações deliberadas: o lote ainda não aceita metadata binário, não persiste descriptors gRPC e não implementa cache entre sessões. A chamada reconsulta Reflection no momento da execução para manter o contrato em memória e evitar estado oculto.

### Primeiro lote de streaming gRPC implementado

- server streaming aceita uma mensagem JSON e exibe várias respostas;
- client streaming aceita uma lista JSON e exibe a resposta final;
- bidirectional streaming aceita uma lista JSON e exibe as respostas recebidas;
- o método é classificado pelas flags do descriptor, sem configuração manual duplicada;
- o transporte usa o codec dinâmico baseado no mesmo `DescriptorPool` do unary e da Reflection;
- cada stream limita entrada e saída a 512 mensagens;
- cada espera por mensagem possui timeout local de inatividade de 30 segundos;
- o usuário pode cancelar o stream em execução pelo comando Tauri dedicado;
- resposta, metadados iniciais, trailers, quantidade de mensagens e duração observada são retornados ao painel;
- o fluxo continua local-first e não grava payloads no histórico neste lote.

Limitações deliberadas: a interface envia uma sequência finita de mensagens e aguarda o fluxo, não oferece edição interativa enquanto o stream está aberto, não persiste requests de streaming, não aceita metadata binário e ainda não exibe cada mensagem por evento incremental. O cancelamento interrompe a operação local quando o future do transporte pode ser descartado, mas não desfaz mensagens já aceitas pelo servidor.

### Primeiro lote do profiler local implementado

- o profiler recebe a request atual e o environment selecionado, resolve variáveis uma vez no core e executa somente a definição resolvida;
- cada execução é feita pelo adapter HTTP existente, preservando a validação, redaction e diagnóstico técnico do fluxo normal;
- o usuário pode executar de 1 a 1000 requests com concorrência de 1 a 32;
- o limite e a confirmação do destino ficam visíveis antes do início para reduzir o risco de disparar carga contra produção por engano;
- o botão de cancelamento sinaliza todas as tarefas e interrompe futures HTTP em andamento quando o runtime consegue descartá-las;
- o profiler emite eventos Tauri de início, progresso e resultado final;
- o resultado separa requests concluídas, canceladas, sucesso HTTP, erro HTTP e erro de transporte;
- o resumo calcula throughput, média, mínimo, máximo, p50, p95, p99 e distribuição de status;
- as amostras do profiler não entram no histórico normal, pois são uma execução de diagnóstico diferente de uma request manual;
- erros de transporte continuam carregando a mensagem amigável e o detalhe técnico disponível, sem incluir secrets resolvidos.

Limitações deliberadas: este lote mede HTTP/HTTPS, não gRPC, WebSocket ou SSE; não possui warm-up, rate limiting, agendamento, distribuição entre máquinas, gravação de cada amostra ou relatório exportável. Como o adapter atual cria um `reqwest::Client` por execução, o resultado não representa reuso de conexão ou comportamento de um pool compartilhado. Cancelar impede novas etapas locais e tenta interromper a operação em andamento, mas não desfaz efeitos já aceitos pelo servidor.

### Primeiro lote do Protocol Lab implementado

- o launcher local abre um mapa educacional da request HTTP atual;
- as camadas exibidas são `Application`, `Transport`, `Internet` e `Network Access`;
- a camada Application mostra método, versão HTTP, status e TTFB quando a resposta existe;
- a camada Transport mostra TCP, TLS, reuso e download, marcando indisponibilidade sem converter ausência em zero;
- a camada Internet mostra DNS preflight e endereços IPv4/IPv6 resolvidos pelo cliente;
- a camada Network Access explica por que Wi-Fi, Ethernet e bytes físicos exigem captura opt-in;
- cada fato exibe sua proveniência como medido, observado ou indisponível;
- o painel preserva a distinção entre evidência do adapter, interpretação didática e lacuna de observabilidade;
- o Protocol Lab não afirma que TTFB representa somente o tempo de processamento do servidor e não simula uma rota de rede.

Limitações deliberadas: este lote não compara protocolos em paralelo, não abre captura de pacotes, não mede a rota por roteadores, não inspeciona Ethernet/Wi-Fi e ainda não possui cenários HTTP/1.1, HTTP/2 ou HTTP/3 executados lado a lado. A comparação de protocolos será adicionada depois que os adapters conseguirem produzir eventos de conexão e stream com provenance própria.

### Segundo lote do Protocol Lab implementado

- o executor HTTP possui os modos `auto`, `http1` e `http2`;
- a execução normal continua em modo automático para preservar a negociação padrão do request client;
- o Protocol Lab pode executar a mesma request em HTTP/1.1 e HTTP/2 forçados, em sequência;
- environment e secrets são resolvidos uma vez antes das duas execuções;
- o comparador retorna status, versão observada, duração, tamanho do body e erro por modo;
- a diferença de duração e bytes é calculada como `HTTP/2 - HTTP/1.1`;
- o comparador não armazena body, headers ou amostras no histórico local;
- se o endpoint não aceitar o modo escolhido, o erro técnico permanece associado somente àquele modo;
- o resultado é uma observação de duas execuções controladas, não uma conclusão geral de que um protocolo é sempre mais rápido.

Limitações deliberadas: HTTP/2 é forçado pelo adapter atual e depende de o endpoint aceitar esse protocolo; ainda não há visualização de uma conexão compartilhada, streams simultâneos, multiplexação, HPACK, priorização ou reuso de pool. HTTP/3/QUIC permanece fora deste lote.

### Hardening de persistência e limites locais

- collections e environments rejeitam arquivos ausentes, YAML inválido e arquivos acima do limite local antes de validar o domínio;
- collections possuem limite de 5 MiB e environments possuem limite de 1 MiB;
- o reader de responses interrompe a leitura quando o body excede 10 MiB, preservando o erro `response_too_large`;
- fixtures locais cobrem os caminhos de parse, metadata e tamanho sem depender de rede externa ou de arquivos do usuário;
- a ausência de um secret é classificada separadamente de uma falha do credential store;
- mensagens de erro de secret não incluem o valor secreto e os testes não gravam credenciais reais no sistema operacional;
- o contrato continua local-first: os limites protegem memória e estabilidade do processo, sem upload ou telemetria.

Limitações deliberadas: a detecção de secret ausente depende do backend de credenciais escolhido pelo sistema; o teste unitário cobre a classificação do erro, enquanto uma verificação real do Credential Manager/Keychain deve ocorrer em smoke tests específicos de cada plataforma.

### Smoke test de distribuição Windows concluído

- `npm run tauri build` executou o build Vue/TypeScript, a compilação release do Rust e o empacotamento Tauri;
- o executável release foi gerado para Windows x64 em `src-tauri/target/release/larry-api-client.exe`;
- os bundles MSI e NSIS foram gerados localmente;
- a configuração de janela, capabilities mínimas, assets de ícone e frontend compilado foram aceitos pelo pipeline de distribuição;
- nenhum backend, serviço remoto, login ou telemetria foi introduzido para gerar os instaladores.

Limitações deliberadas: este smoke test foi executado apenas no Windows x64 da máquina de desenvolvimento; ainda não existe CI para instaladores, assinatura de código, atualização automática ou validação em uma máquina limpa sem toolchain local.

## 6. Escopo

### MVP

- HTTP/HTTPS com GET, POST, PUT, PATCH e DELETE;
- headers, query, cookies, JSON, texto e form-data;
- Bearer, Basic e API Key;
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
- WebSocket básico;
- SSE básico;
- assertions HTTP declarativas para status, headers e body;
- modo educacional;
- Protocol Lab inicial baseado em evidências do trace HTTP;
- comparação controlada da mesma request em HTTP/1.1 e HTTP/2;
- gRPC unary dinâmico por `.proto` como vertical slice.

### V2

- persistência de requests e descriptors gRPC;
- cache local controlado de descriptors gRPC;
- histórico e collections para requests gRPC;
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
13. gRPC dinâmico: importação `.proto`, descriptors e unary.
14. Profiler local.
15. Protocol Lab e modo educacional, começando pelo mapa de camadas HTTP e comparação controlada HTTP/1.1 versus HTTP/2.
16. Hardening e distribuição.

Cada fase pode ser entregue em um ou mais lotes de implementação. O formato do relatório final e os critérios de validação ficam neste plano e no checklist do projeto.
