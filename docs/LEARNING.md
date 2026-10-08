# LEARNING — trilha guiada e relatório de implementação

O usuário aprende o projeto lendo e revisando os lotes implementados pelo agente. O agente atua como mentor e executor: explica, implementa, testa, revisa e diagnostica. Não avançar de fase apenas porque o projeto compila.

## Formato de cada sessão/lote

Antes da implementação:

1. objetivo e escopo do lote;
2. relação com a fase atual;
3. arquivos que provavelmente serão envolvidos;
4. critérios de aceite.

Depois da implementação:

5. arquivos criados, alterados ou removidos;
6. mudanças relevantes explicadas por arquivo;
7. conceitos e decisões para o usuário estudar;
8. testes executados e resultado;
9. erros, limitações e riscos conhecidos;
10. checklist de conclusão;
11. comandos de revisão, salvamento e commit granular;
12. próxima fase somente depois da confirmação do usuário.

O lote pode ser maior que um trecho isolado, mas deve manter uma única capacidade coerente e terminar em estado testável.

### Lote visual atual — fundação do tema escuro

Objetivo: aplicar o contrato visual de `docs/design.md` ao shell e aos componentes existentes sem alterar a execução funcional.

Implementado:

- topbar com a marca Larry usando `public/assets/larry-platypus.png`;
- shell de janela Tauri sem a faixa nativa do Windows, com controles reais de janela;
- permissões Tauri específicas para arrastar, minimizar, maximizar/restaurar e fechar;
- ícones desktop do bundle regenerados a partir da logo do Larry;
- tema escuro com tokens globais de superfície, texto, borda, marca e status;
- foco visível para controles nativos;
- botões, inputs, selects, textareas, listas, response e Git com superfícies sólidas;
- estados `Pronto`, `Executando`, `Concluída` e `Falhou` com texto e cor de apoio;
- seleção de request sem `border-left` decorativo;
- remoção das cores claras herdadas do protótipo inicial.

Decisão importante: o response inspector agora separa Body e Headers em abas funcionais. Trace e Diagnostics existem como estados explícitos de indisponibilidade até que o core Rust publique eventos de rede e classificações por camada.

Conceitos praticados nesta etapa:

- `ref` para controlar a aba ativa;
- `watch` para resetar a aba quando chega uma resposta nova;
- `role="tablist"`, `role="tab"` e `role="tabpanel"` para semântica acessível;
- provenance visível para diferenciar dado observado de capacidade ainda indisponível;
- layout responsivo para headers, listas e conteúdo técnico.

Validação manual esperada:

- abrir o Tauri e verificar o topbar escuro com a logo, sem a barra branca nativa do Windows;
- arrastar a janela pelo espaço vazio da topbar;
- minimizar, maximizar/restaurar e fechar pelos controles da topbar;
- selecionar uma request e observar a mudança de superfície;
- editar um campo e conferir o foco;
- executar uma request e conferir os estados de carregamento, sucesso e erro;
- redimensionar a janela sem scroll horizontal acidental.

### Lote visual atual — sidebar de collections

Objetivo: separar a navegação da collection do editor de request e aproximar o workspace do padrão desktop observado nas referências.

Implementado:

- novo `CollectionSidebar.vue` com responsabilidade exclusiva pela navegação;
- lista de requests movida para a coluna lateral;
- editor central sem a lista duplicada;
- painel lateral para persistência local e Git;
- grid desktop com sidebar, editor e response inspector;
- breakpoint intermediário que empilha response abaixo do editor;
- breakpoint estreito que transforma o layout em uma coluna;
- ações de duplicar e remover visíveis no hover/foco, sem usar borda lateral decorativa.

Conceitos para revisar:

- extração de componente sem duplicar estado;
- props e emits como contrato entre `App.vue` e sidebar;
- CSS Grid com `minmax`;
- diferença entre navegação e edição;
- por que ações secundárias podem aparecer somente no hover, mas continuam acessíveis via teclado.

### Lote de revisão do sistema visual

Objetivo: consolidar o comportamento de uma ferramenta desktop densa antes de continuar expandindo a UI.

Decisões registradas em `docs/design.md`:

- o shell não deve ter padding externo de página;
- `body` não deve rolar durante o uso normal do workspace;
- sidebar, request e response devem rolar em regiões independentes;
- buttons, inputs, selects, tabs e icon buttons precisam compartilhar altura, baseline e estados;
- chevrons e setas devem ser SVGs consistentes, não caracteres Unicode;
- a skill `.cursor/skills/ui-visual-system-validation/SKILL.md` será aplicada em novas mudanças de UI.

Implementado nesta fase:

- `html`, `body`, `#app` e `.app-shell` passaram a controlar a altura e impedir o scroll global;
- `.workspace` usa `flex: 1`, `min-height: 0` e overflow controlado;
- regiões de sidebar, editor e response rolam separadamente;
- o grid desktop usa divisores entre regiões, sem padding externo de página;
- botões e inputs visíveis foram compactados para o padrão de 34 px;
- selects receberam chevrons SVG;
- o botão `+` recebeu SVG linear;
- breakpoints intermediários e estreitos foram ajustados para evitar clipping.

O próximo estudo deve revisar por que `min-height: 0` é necessário em filhos de Grid/Flex e como o overflow muda entre desktop e janela estreita.

### Lote de reformulação: Larry Network Workbench

Objetivo: transformar um conjunto de painéis corretos, porém empilhados e dispersos, em uma ferramenta desktop com uma hierarquia de trabalho própria.

Leitura das referências:

- Bruno concentra a request em método, URL, envio e abas, mantendo a navegação densa e periférica;
- Insomnia mostra request e response como duas superfícies técnicas simultâneas;
- Larry aproveita essas lições sem copiar marca, paleta ou hierarquia: seus nomes de trabalho passam a ser **Request rail**, **Composer** e **Inspector**.

Implementado:

- a topbar passa a usar a assinatura `Larry / Local workbench`, contexto ativo e environment, removendo botões decorativos sem comportamento;
- a barra lateral ganha abas reais: `Requests` mantém a navegação da collection e `Ferramentas locais` reúne arquivo, environment e histórico;
- `WorkspaceRail.vue` centraliza a composição desses painéis e encaminha eventos, mantendo `App.vue` como coordenador de estado;
- o Composer mostra o nome ativo, método, URL e envio antes das opções da request;
- `Params`, `Headers` e `Body` tornam-se abas acessíveis e exibem contagens de itens habilitados;
- o response viewer passa a se apresentar como Inspector e usa a mesma hierarquia de abas;
- uma statusbar discreta desloca versão e plataforma para fora do fluxo de edição.

Conceitos para revisar:

- **composição de componentes:** `WorkspaceRail` organiza componentes existentes sem absorver o estado de `App.vue`;
- **props e emits encaminhados:** o componente intermediário recebe dados e reemite intenções para o pai, sem chamar IPC;
- **divulgação progressiva:** abas escondem detalhes de uma responsabilidade sem apagar informação nem criar uma segunda tela;
- **hierarquia visual:** a ação frequente fica no Composer, a evidência fica no Inspector e a administração local fica fora da navegação primária;
- **controle sem função:** remover um botão é melhor que sugerir uma ação que ainda não existe.

Limitação de validação: o build verifica a composição Vue e tipos, mas a aparência final ainda precisa ser conferida na janela Tauri real após este lote. O preview puro do Vite não fornece a ponte Tauri nem reproduz os controles de janela.

### Lote de refinamento: ferramentas locais e estado da topbar

Problema observado: a aba `Ferramentas locais` renderizava Collection, Environment e History verticalmente dentro de um rail estreito. Os formulários competiam com a navegação e criavam uma coluna difícil de escanear.

Implementado:

- o rail agora mostra três resumos curtos e abre cada ferramenta no modal compartilhado;
- `AppModal` ganhou montagem persistente, para fechar um modal sem desmontar e perder os campos preenchidos;
- o histórico usa o tamanho `wide`, adequado para listas e comparação;
- `Pronto` foi removido da topbar porque era apenas um estado genérico sem ação própria;
- o botão `Enviar` continua mostrando `Enviando...`, que é o feedback contextual correto;
- a documentação passou a tratar `Ferramentas locais` como launcher e não como formulário.

Conceitos para revisar:

- **launcher versus editor:** a navegação mostra contexto e o modal recebe a tarefa detalhada;
- **keep mounted:** `v-if` controla a existência do modal e `v-show` controla sua visibilidade, preservando estado local dos componentes;
- **estado útil:** uma informação deve permanecer na topbar apenas quando ajuda o usuário a tomar uma decisão;
- **modal com conteúdo:** o template pode receber componentes completos, desde que o foco, Escape, fechamento e scroll permaneçam centralizados.

### Lote de modais padronizados

Implementado:

- novo componente `AppModal.vue` com variantes `neutral`, `info`, `danger` e `error`;
- confirmação de remoção de request migrada de `window.confirm`;
- gerenciamento de foco inicial e retorno ao elemento de origem;
- navegação por Tab/Shift+Tab confinada ao modal;
- fechamento por `Esc` e backdrop configurável;
- template preparado para descrição, slot de detalhes e ações nomeadas.

Conceitos para revisar:

- `Teleport` para renderizar overlays fora da hierarquia visual do painel;
- `watch` e `nextTick` para focar elementos após renderização;
- `document.activeElement` para preservar o fluxo de teclado;
- diferença entre confirmação destrutiva e aviso informativo;
- por que mensagens técnicas não devem ser reduzidas a `Algo deu errado`.

### Lote de dimensão inicial da janela concluído

Objetivo: fazer o executável começar com uma área de trabalho compatível com a densidade de um API Client desktop, sem forçar maximização.

Implementado em `src-tauri/tauri.conf.json`:

- dimensão inicial de 1280 x 800 px;
- mínimo de 960 x 640 px para preservar o grid principal;
- abertura centralizada no monitor;
- decoração nativa continua desativada e o redimensionamento continua permitido.

Conceitos para revisar:

- `width` e `height` definem a geometria inicial, não o tamanho permanente da janela;
- `minWidth` e `minHeight` protegem os breakpoints e evitam que os painéis fiquem inutilizáveis;
- `center` define a posição inicial, mas não equivale a maximizar;
- o tamanho da janela e o layout responsivo são responsabilidades diferentes: o primeiro vem do Tauri, o segundo do CSS.

Limitação conhecida: esta alteração ainda não persiste o último tamanho escolhido pelo usuário. Persistência de posição/tamanho pode ser adicionada depois, desde que respeite múltiplos monitores e não grave dimensões inválidas.

## Lote de trace HTTP e diagnóstico concluído

Objetivo: substituir os estados vazios de Trace e Diagnostics por dados reais, deixando claro o que foi medido e o que ainda depende de instrumentação mais profunda.

Fluxo implementado:

1. Resolver o host em um preflight DNS no core Rust.
2. Medir o intervalo até os headers da resposta, chamado TTFB observado.
3. Medir a leitura completa do body e o tempo total monotônico.
4. Registrar endereços resolvidos e versão HTTP retornada pelo adapter.
5. Classificar status HTTP por camada de aplicação.
6. Manter TCP, TLS detalhado e reuso como indisponíveis, sem preencher valores estimados como se fossem medidos.
7. Renderizar timeline e provenance no response inspector.

Conceitos para revisar:

- `Instant` mede duração sem depender de mudanças no relógio civil;
- TTFB é o tempo até os headers, não o tempo puro de processamento no servidor;
- uma consulta DNS de preflight fornece observação útil, mas não prova que o lookup interno do cliente reutilizou o mesmo resultado;
- HTTP status é uma resposta da camada de aplicação, enquanto DNS e conexão são falhas anteriores à resposta;
- `Option<u64>` diferencia uma métrica indisponível de uma métrica medida como zero;
- provenance impede que a UI trate inferência como medição.

Exercício de revisão:

- explique por que TCP e TLS não podem ser apresentados como zero nesta etapa;
- compare `total`, `ttfb` e `download` no fixture HTTP local;
- indique em que ponto 401 deve ser diagnosticado como aplicação, não como falha de transporte;
- proponha quais eventos seriam necessários para medir ALPN, certificado e connection reuse.

Limitações conhecidas: o erro de execução ainda chega à UI como texto enriquecido, sem um painel persistente de diagnóstico de falha; redirects, HTTP/2 streams, certificado e cipher suite ficam para o adapter instrumentado.

## Fase 0 — preparação e mapa

1. Verificar Rust, Cargo, Node, pnpm, Git, MSVC e WebView2.
2. Inicializar Git e salvar a documentação.
3. Criar Tauri usando Vue + TypeScript + Vite.
4. Executar `pnpm install` e `pnpm tauri dev`.
5. Identificar `src`, `src-tauri`, `package.json`, `vite.config.ts` e `tauri.conf.json`.
6. Criar `docs/decisions.md` com o motivo da escolha Vue/Tauri/Rust.

**Checkpoint:** explicar o que roda no WebView, no processo Rust e no IPC.

## Fase 1 — ponte IPC

1. Criar uma função Rust pura.
2. Transformá-la em comando Tauri.
3. Registrar o comando.
4. Invocá-lo a partir de um componente Vue.
5. Modelar sucesso, carregamento e erro.
6. Criar evento Rust → Vue.
7. Testar a função sem abrir a janela.

**Exercício:** comando que recebe número e retorna o dobro com validação.

## Fase 2 — domínio

1. Definir `RequestDefinition`, `Run`, `ResponseSnapshot`, `Diagnostic` e `TraceEvent`.
2. Definir IDs e estados do run.
3. Modelar headers, body e método sem acoplar à UI.
4. Definir erros de domínio e erros de transporte.
5. Testar serialização e invariantes.

**Checkpoint:** domínio testável sem rede e sem janela.

## Fase 3 — HTTP local

1. Criar fixture server local.
2. Implementar GET em Rust com Tokio.
3. Ler status, headers e body.
4. Adicionar POST JSON.
5. Adicionar timeout, cancelamento e limite de body.
6. Emitir eventos de execução.
7. Mostrar resultado em Vue.

**Checkpoint:** distinguir status HTTP de falha de transporte.

## Fase 4 — UI Vue

1. Criar componentes para método, URL, query e headers.
2. Usar `ref` e `computed` conscientemente.
3. Criar formulário de body.
4. Separar rascunho de request salva.
5. Criar response viewer simples.
6. Extrair lógica repetida para composables.

**Checkpoint:** explicar a diferença entre estado local, prop, emit e composable.

## Fase 5 — collections e Git

1. Criar `collection.yaml` manualmente.
2. Definir `schema_version` e IDs estáveis.
3. Abrir e salvar arquivos pelo Rust.
4. Criar pastas e requests.
5. Adicionar `.gitignore`.
6. Observar diff textual.
7. Criar diff semântico inicial.
8. Mostrar branch e status no Vue.

### Primeiro lote concluído

- contrato `CollectionFile` com `schemaVersion` versionado;
- serialização YAML no Rust;
- validação de nome e IDs;
- comandos IPC para salvar e carregar;
- limite de tamanho e escrita por arquivo temporário;
- painel Vue para salvar/carregar a request atual;
- fixtures de round-trip e erros de schema/extensão.

O seletor visual de arquivos e a collection com múltiplas requests ainda são os próximos incrementos desta fase.

### Segundo lote concluído

- integração read-only com o executável Git local;
- detecção da raiz do repositório;
- branch atual e status do working tree;
- diff textual da collection;
- limite de saída e erros técnicos preservados;
- painel Git na interface;
- fixture de repositório temporário no teste Rust.

Ainda não há commit, push, checkout ou diff semântico automático.

### Terceiro lote concluído — múltiplas requests

Objetivo: deixar de tratar a collection como um arquivo de uma única request e criar o primeiro fluxo de trabalho navegável.

Aprendizados principais:

1. `App.vue` é o dono do estado compartilhado: lista, item ativo, resposta e erros.
2. `CollectionManager.vue` recebe dados por props e comunica intenções por eventos; ele não altera diretamente o estado do pai.
3. A request editada precisa ser sincronizada na lista antes do salvamento, senão o YAML poderia guardar uma versão antiga.
4. Selecionar uma request incrementa `resetToken` para que o editor local recarregue body, método, URL, headers e query.
5. Duplicar exige novo ID; copiar somente os campos visíveis quebraria a validação de IDs únicos.
6. A lista usa um item ativo textual e visual, foco visível, truncamento deliberado e ações com `aria-label`.
7. A remoção exige confirmação e impede que a collection fique sem request no fluxo atual.

Checklist manual do lote:

- adicionar uma request e verificar que ela vira a ativa;
- editar URL/nome e alternar para outra request;
- voltar e confirmar que a edição foi preservada;
- duplicar uma request e conferir o novo nome e ID;
- remover uma request e confirmar a troca de seleção;
- salvar e abrir o YAML para verificar que todas as requests foram persistidas;
- carregar uma collection com mais de uma request e alternar entre elas.

**Checkpoint:** clonar collection e abrir sem login ou banco proprietário.

### Lote de onboarding e importação Postman concluído

Objetivo: tornar o primeiro uso compreensível e reduzir o custo de migração sem transformar o Larry em um clone do workspace Postman.

Fluxo implementado:

1. Detectar primeiro uso local com `localStorage`, sem persistir secret.
2. Mostrar o modal `Bem-vindo ao Larry`.
3. Permitir explorar, criar collection, começar request ou importar Postman.
4. Ler uma collection Postman JSON v2.1 no Rust com limite de 10 MiB.
5. Converter requests, métodos, URLs, query, headers e bodies compatíveis.
6. Mostrar uma prévia antes de colocar a collection na memória do workspace.
7. Exibir avisos de folders, variables, scripts, auth e campos não convertidos.
8. Substituir dados sensíveis reconhecíveis por referências `{{secret.*}}`.
9. Persistir somente a decisão de onboarding, sem conta ou backend.

Conceitos praticados:

- `Teleport` e slots para reutilizar o modal comum em fluxos de onboarding;
- comandos IPC para separar leitura de arquivo e conversão do frontend;
- parser tolerante com avisos explícitos em vez de descarte silencioso;
- sanitização por nome de header/campo antes de retornar dados para a UI;
- prévia em memória como barreira antes do salvamento file-first;
- `localStorage` somente para preferência de onboarding, nunca para secrets.

Exercício de revisão:

- explique por que o importador deve produzir avisos em vez de descartar recursos;
- identifique quais campos Postman cabem diretamente em `RequestDefinition`;
- liste quais dados devem ser tratados como secret;
- descreva a diferença entre importar um arquivo Postman e sincronizar com Postman Cloud;
- explique por que o importador não pode prometer que detecta todos os secrets.

Limitações conhecidas: o caminho do JSON ainda é digitado manualmente, placeholders não são resolvidos, autenticação não é convertida em credencial executável e o seletor nativo de arquivos fica para um lote posterior.

## Fase 6 — environments e secrets

1. Criar variáveis públicas e referências secretas.
2. Implementar precedência de resolução.
3. Guardar secrets no Credential Manager.
4. Mascarar logs, histórico, diff e export.
5. Detectar possíveis secrets em arquivos.
6. Exigir confirmação ao trocar para produção.

**Checkpoint:** request funciona sem secret literal no arquivo.

### Lote de environments e secrets concluído

Objetivo: introduzir variables sem misturar credenciais com arquivos versionáveis e resolver placeholders somente no core local.

Fluxo implementado:

1. Criar um environment YAML versionado por schema.
2. Editar variáveis públicas e referências `secretRef` em `EnvironmentPanel.vue`.
3. Gravar o valor do secret no armazenamento nativo via Rust/keyring.
4. Enviar apenas a referência do secret para o frontend depois do carregamento.
5. Resolver URL, query, headers e bodies no Rust antes da execução HTTP.
6. Manter uma lista de valores para redaction em erros de transporte.
7. Passar o environment ativo como DTO no comando `execute_request`.

Conceitos praticados:

- DTO de arquivo não é o mesmo que valor armazenado no Credential Manager;
- `Option<String>` representa a escolha entre valor público e referência secreta;
- `watch` mantém o environment ativo sincronizado com o editor Vue;
- a resolução recursiva percorre strings JSON sem acoplar o domínio à interface;
- redaction é uma proteção de saída e não substitui a proibição de registrar secrets;
- `keyring` abstrai o armazenamento nativo, mas seus erros ainda precisam preservar a categoria sem expor credenciais;
- o core Rust continua sendo a única camada autorizada a tocar em arquivo, rede e secrets.

Exercício de revisão:

- explique por que `secretRef` deve ser persistido no YAML em vez do valor;
- identifique por que o frontend recebe a referência, mas não recebe `get_environment_secret`;
- descreva o risco de aceitar `{{missing}}` literalmente numa URL;
- explique por que resolver placeholders antes do executor mantém a camada HTTP simples;
- proponha uma regra de precedência para variables de workspace, collection e request.

Limitações conhecidas: a UI ainda usa caminho digitado, não há precedência entre escopos, não há detector automático de secrets e a exclusão da referência não remove a credencial nativa.

## Fase 7 — trace e diagnóstico

1. Medir duração com relógio monotônico.
2. Instrumentar DNS.
3. Instrumentar TCP.
4. Instrumentar TLS, ALPN e certificado quando possível.
5. Medir envio, primeiro byte e download.
6. Identificar reuso de conexão.
7. Criar fixtures de falha DNS/TCP/TLS/HTTP.
8. Renderizar timeline Vue.

**Checkpoint:** explicar por que TTFB não é automaticamente tempo do servidor.

## Fase 8 — histórico e replay

1. Persistir metadados no SQLite.
2. Guardar bodies grandes fora do banco.
3. Repetir, duplicar e editar requests.
4. Comparar status, headers, body, trace e latência.
5. Exportar incident capsule sanitizada.

### Lote de histórico, replay e comparação concluído

Objetivo: criar memória local das execuções sem transformar o SQLite em fonte das collections nem armazenar secrets literais.

Fluxo implementado:

1. Abrir ou criar `history.sqlite3` no diretório local da aplicação.
2. Aplicar uma migration inicial com schema versionado.
3. Sanitizar headers, query params, campos JSON sensíveis e valores conhecidos antes de persistir.
4. Limitar bodies persistidos a 256 KiB e registrar quando houve omissão.
5. Persistir sucesso, erro HTTP e falha de transporte como outcomes distintos.
6. Listar resumos no painel de histórico.
7. Carregar uma execução para revisão e replay.
8. Comparar duas execuções por status, headers, body e delta de latência.

Conceitos para revisar:

- SQLite guarda histórico derivado, enquanto a collection continua file-first;
- migration é diferente de criar tabelas de forma implícita sem versão;
- sanitização reduz risco, mas não detecta todo segredo possível;
- `bodyTruncated` evita confundir body ausente com body vazio;
- replay de um snapshot sanitizado precisa ser revisado pelo usuário antes do envio;
- comparação de headers usa nomes normalizados e comparação de body usa o snapshot disponível;
- falha de persistência do histórico não deve impedir uma request que já foi enviada.

Exercício de revisão:

- explique por que o histórico guarda uma request sanitizada, e não a request resolvida;
- identifique o que se perde quando um body excede 256 KiB;
- proponha um diff semântico para headers repetidos e JSON aninhado;
- descreva como você adicionaria limpeza seletiva sem apagar collections;
- explique por que sincronização e login continuam fora desta fase.

## Fase 9 — WebSocket, SSE e assertions

1. Abrir WebSocket local.
2. Mostrar envio e recebimento.
3. Persistir mensagens com limite.
4. Implementar SSE.
5. Criar assertions declarativas.
6. Só depois estudar scripts sandboxed.

## Fase 10 — gRPC dinâmico

1. Criar fixture com `PaymentService`.
2. Importar `payments.proto`.
3. Construir descriptor pool.
4. Listar services/messages/methods.
5. Criar formulário dinâmico.
6. Executar unary.
7. Tentar Reflection.
8. Implementar streams e cancelamento.

**Checkpoint:** usar um `.proto` novo sem gerar serviço Rust estático.

## Fase 11 — profiler

1. Executar uma request contra fixture.
2. Adicionar concorrência limitada.
3. Implementar cancelamento.
4. Calcular média, p50, p95 e p99.
5. Agrupar erros e timeouts.
6. Exigir confirmação para volume alto/produção.

## Fase 12 — Protocol Lab

1. Comparar HTTP/1.1 e HTTP/2.
2. Separar connection events de stream events.
3. Comparar cold/warm connection.
4. Exibir bytes observados e limitações.
5. Adicionar explicação das camadas de rede.

## Fase 13 — hardening

1. Revisar Tauri capabilities.
2. Testar instalação limpa e upgrade.
3. Testar migrations e corrupção de SQLite.
4. Auditar dependências e licenças.
5. Criar build Windows.
6. Documentar limitações.

## Diário de aprendizado

Ao terminar cada fase, registrar:

- o que foi implementado;
- o que consegue explicar sem consultar;
- qual erro encontrou;
- como diagnosticou;
- qual decisão pode mudar;
- qual experimento vem depois.

## Relatório obrigatório do agente

No final de cada tarefa, o agente deve separar explicitamente:

- **Criado:** arquivos e estruturas novos;
- **Alterado:** comportamento e motivo da alteração;
- **Removido:** código ou arquivos retirados e por quê;
- **Preservado:** partes importantes que não foram mexidas;
- **Testado:** comandos e resultados;
- **Estudado:** conceitos que o usuário deve entender;
- **Commit:** comandos para salvar a entrega.

