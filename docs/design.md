# Larry API Client: sistema visual

**Status:** reformulação de identidade e workspace em andamento
**Data:** 2026-10-08
**Escopo:** desktop Tauri, tema escuro, interface de API testing e network debugging

Este documento define a linguagem visual do Larry antes da próxima grande revisão da UI. Ele deve ser consultado antes de criar novos componentes, telas ou fluxos.

## 1. Direção

Larry é uma ferramenta de trabalho para desenvolvedores. A interface deve parecer um workspace técnico confiável, rápido e silencioso.

O produto deve comunicar:

- controle local dos dados;
- precisão técnica;
- leitura rápida de requests e responses;
- diagnóstico baseado em evidência;
- conexão com arquivos e Git;
- aprendizado sem infantilizar o usuário.

O Larry não deve parecer:

- um dashboard genérico;
- uma landing page dentro do aplicativo;
- um produto SaaS que exige conta;
- um assistente decorativo sem ação real;
- uma cópia visual de Postman, Insomnia ou Bruno.

## 2. Referências observadas

### Bruno

O site do Bruno posiciona o produto como Git-native, local e baseado em collections armazenadas como código. A imagem da aplicação e referências visuais públicas mostram uma organização com coleção na lateral, editor de request no centro, abas de configuração e resposta estruturada.

O Larry deve absorver:

- navegação persistente de collections;
- relação clara entre arquivo, request e Git;
- densidade adequada para uso diário;
- abas técnicas em vez de uma tela longa sem hierarquia;
- sensação de ferramenta local e rápida.

O Larry não deve copiar:

- cores, logotipo, nomes ou ícones do Bruno;
- organização de menus sem validar a necessidade do trace;
- linguagem de marca ou mascote de outro produto.

### Insomnia

O site e as imagens públicas do Insomnia apresentam um workspace em painéis, com navegação de requests, configuração no centro e response preview separado. O produto atual também apresenta modos de design, debug e teste, além de fluxos local, Git e cloud.

O Larry deve absorver:

- separação explícita entre configurar e inspecionar;
- resposta sempre próxima da request;
- seleção de environment em posição previsível;
- possibilidade de crescer para diferentes modos de trabalho.

O Larry não deve copiar:

- a marca Kong/Insomnia;
- a linguagem de plataforma colaborativa como prioridade da V1;
- a estética ou nomenclatura de recursos que o Larry ainda não suporta.

### Fontes de referência

- Bruno: <https://www.usebruno.com/>
- Insomnia: <https://insomnia.rest/>
- Imagem pública de referência do Bruno: <https://www.sonarsource.com/blog/scripting-outside-the-box-api-client-security-risks-part-2/>
- Imagem pública de referência do Insomnia: <https://docs.insomnia.rest/insomnia/send-your-first-request>

As referências servem para estudar padrões de organização. Não são especificações para copiar.

### Leitura visual das referências anexadas

As imagens anexadas do Bruno e do Larry foram usadas para revisar comportamento de shell, não para copiar marca ou detalhes proprietários.

O padrão que interessa ao Larry é:

- topbar em largura total, sem margem externa que faça a aplicação parecer um card;
- sidebar persistente com divisor vertical discreto;
- workspace central ocupando todo o espaço restante;
- scroll vertical independente dentro de cada região que possui conteúdo longo;
- ausência de uma scrollbar única movendo topbar, sidebar, request e response juntos;
- controles compactos, alinhados por baseline e com estados de hover/foco claros;
- ícones lineares pequenos para ações conhecidas, sem caracteres Unicode usados como ícones improvisados;
- abas e divisores horizontais para organizar contexto sem transformar toda a tela em uma sequência de cards.

O Larry deve manter seus próprios tokens, sua marca e o response inspector com trace. A referência não autoriza copiar a paleta, o mascote, os nomes ou a hierarquia específica de outro produto.

### Auditoria de organização: Bruno, Insomnia e Larry

As duas referências resolvem o mesmo problema de formas diferentes:

- **Bruno** privilegia um trilho de navegação denso à esquerda, uma faixa de composição no centro e divisores discretos. A request abre primeiro pelo método, URL e envio; detalhes entram em abas. O espaço vazio da resposta comunica o próximo atalho, sem competir com o editor.
- **Insomnia** privilegia três superfícies simultâneas: árvore de requests, conteúdo que será enviado e conteúdo recebido. A comparação visual entre request e resposta é imediata, e as abas preservam o contexto de cada painel.
- **Larry** já tinha os dados necessários, mas os empilhava: título promocional, diagnóstico global, collection, arquivo, environment, histórico, editor e resposta disputavam atenção em uma página longa. Isso enfraquecia a ferramenta justamente quando a request precisava ser o centro.

O novo Larry adota o padrão de **workbench de rede**: uma navegação à esquerda, um **Composer** para montar e disparar a request e um **Inspector** para examinar a evidência devolvida. Não replica layout, termos, paleta ou marca dos concorrentes.

Decisões de identidade:

- `LARRY / LOCAL WORKBENCH` é uma assinatura compacta da aplicação, não uma logo central decorativa;
- o contexto ativo usa uma rota curta, por exemplo `MINHA API / CRIAR PAGAMENTO`;
- `Composer` e `Inspector` nomeiam responsabilidades próprias do Larry e preparam a expansão para trace, diagnóstico e Protocol Lab;
- o mascote aparece na marca e no onboarding, nunca repetido nos painéis de trabalho;
- status, environment e versão vivem nas bordas do shell e não no espaço usado para editar uma request.

## 3. Identidade do Larry

### Marca

O asset de marca atual é:

```text
public/assets/larry-platypus.png
```

Uso recomendado:

- janela e ícone do aplicativo;
- marca compacta no topo da navegação;
- tela de início vazia ou onboarding local;
- página de diagnóstico da aplicação.

Regras:

- usar a imagem em tamanho pequeno e estável, normalmente 24, 28 ou 32 px;
- preservar a proporção original;
- não aplicar gradiente, sombra colorida ou filtro adicional via CSS;
- não usar a imagem como fundo de uma tela de trabalho;
- não repetir a logo em cada painel;
- quando o fundo preto do asset conflitar com a superfície, usar uma área de marca controlada, sem inventar outro tratamento visual.

A logo atual possui iluminação própria. Isso pertence ao asset da marca e não autoriza gradientes no restante do sistema.

### Personalidade

Tom visual: técnico, direto, calmo e curioso.

Textos devem ser curtos e informativos:

- `Enviar request`
- `Request concluída`
- `Falha na conexão TCP`
- `Trace indisponível`
- `Consultar Git`

Evitar:

- emojis na interface;
- frases promocionais dentro do workspace;
- texto genérico como `Algo deu errado` sem causa técnica;
- copy com travessão longo ou hífen usado como decoração;
- símbolos decorativos como setas soltas, estrelas ou sparkles;
- promessas de automação que não correspondem a uma ação real.

## 4. Princípios visuais

1. **Informação antes de ornamentação.** O usuário deve encontrar método, URL, environment, envio e resultado sem procurar.
2. **Uma ação primária por contexto.** Enviar, salvar ou executar deve ser visualmente superior às ações secundárias.
3. **Superfícies sólidas.** Usar cor, espaçamento e bordas discretas. Não usar gradientes, glassmorphism ou blur decorativo.
4. **Estado sem depender só de cor.** Todo estado deve ter texto, ícone funcional opcional e cor de apoio.
5. **Densidade controlada.** O Larry é uma ferramenta desktop. Campos podem ser compactos, mas não apertados.
6. **Evidência visível.** Métricas devem mostrar unidade, origem e disponibilidade.
7. **Separação por função.** Navegação, edição, execução, response e diagnóstico devem ter regiões reconhecíveis.
8. **Consistência antes de novidade.** Um componente novo precisa reutilizar tokens e padrões existentes.
9. **Arquivos continuam legíveis.** A UI não deve esconder a relação entre request, collection e arquivo local.
10. **Acessibilidade é parte do visual.** Foco, teclado, contraste e estados desabilitados precisam ser visíveis.

## 5. Layout do aplicativo

### Estrutura desktop principal

```text
┌────────────────────────────────────────────────────────────────────┐
│ LARRY / LOCAL WORKBENCH │ contexto ativo │ environment │ janela    │
├──────────────┬───────────────────────────────────────┬─────────────┤
│ Collection   │ Composer                              │ Inspector   │
│ tree         │ método + URL + enviar                 │ status      │
│ folders      │ tabs: Params · Headers · Auth · Body │ body        │
│ requests     │ editor com scroll próprio             │ headers     │
│              │                                       │ trace       │
│              │                                       │ diagnostics │
├──────────────┴───────────────────────────────────────┴─────────────┤
│ LOCAL FIRST · CORE HTTP · versão e plataforma                         │
└──────────────┴───────────────────────────────────────┴─────────────┘
```

Regiões:

- **Topbar:** assinatura Larry, rota de contexto, environment e controles de janela. O estado de execução fica junto da ação `Enviar`, no Composer.
- **Collection tree:** uma única árvore persistente de folders e requests. Ações de arquivo e criação ficam no cabeçalho da collection; não existe uma aba separada de ferramentas locais.
- pastas usam linhas compactas, chevron SVG e expansão local; requests dentro delas preservam sua ordem no YAML;
- o menu contextual do cabeçalho oferece as ações que já têm fluxo real, como nova request, nova pasta e abertura do YAML;
- **Composer:** método, URL, envio e configuração da request em abas.
- **Inspector:** status, tempo, tamanho, histórico, body, headers, trace, diagnostics, Protocol Lab e Profiler.
- **Statusbar:** dados locais de baixo ruído, fora dos três painéis de trabalho.

Não usar uma faixa colorida lateral para indicar seleção. A seleção deve usar superfície diferente, texto e foco visível.

### Scroll e ocupação da janela

O shell desktop deve controlar o viewport inteiro. A regra padrão é:

```css
html,
body,
#app,
.app-shell {
  height: 100%;
  overflow: hidden;
}

.workspace {
  min-height: 0;
  overflow: hidden;
}

.sidebar,
.request-workspace,
.response-inspector {
  min-height: 0;
  overflow-y: auto;
}
```

Aplicação prática:

- a topbar não rola;
- a sidebar rola apenas sua árvore de collections;
- o editor de request rola apenas seu conteúdo;
- o response inspector rola apenas o resultado;
- o scroll global fica reservado para onboarding, modal ou uma tela que explicitamente não seja o workspace;
- `min-height: 0` é obrigatório em filhos de grids e flex containers que precisam rolar;
- conteúdo longo deve quebrar ou rolar em sua região, sem criar uma scrollbar no `body`;
- a altura útil deve ser calculada a partir da janela menos topbar e, quando existir, statusbar.

Não usar `min-height` fixo em painéis como substituto de um layout com altura controlada. Isso cria espaços vazios e empurra o scroll para o documento inteiro.

### Margens e padding do shell

O workspace não recebe um padding externo grande. A janela deve começar visualmente na topbar e dividir o espaço até as bordas internas do app.

Para a primeira abertura do executável, a janela usa uma dimensão desktop confortável de 1280 x 800 px, centralizada no monitor, com mínimo de 960 x 640 px. Isso evita que o workspace comece em uma área próxima de uma janela compacta de diagnóstico, sem impor maximização ou impedir o redimensionamento pelo usuário.

- topbar: largura total da janela;
- topbar: padding interno horizontal de 24 a 32 px no desktop, reduzido nos breakpoints estreitos;
- sidebar: largura própria, com padding interno de 12 a 16 px;
- request e response: padding interno de 16 a 20 px;
- divisores entre regiões: 1 px, discretos;
- distância entre colunas: preferir divisor e padding interno a um vão que pareça um card flutuante;
- no desktop, não usar `width: calc(100% - 40px)` no shell principal;
- em telas pequenas, reduzir o padding interno, mas preservar alvos de interação.

O conteúdo pode ter uma área de leitura limitada dentro de um editor ou documento, mas o shell de trabalho não deve parecer uma página centralizada dentro de outra página.

### Comportamento responsivo

O Larry é desktop-first, mas a janela pode ser redimensionada.

- acima de 1180 px: três regiões visíveis;
- entre 900 e 1179 px: sidebar reduzível e response em painel inferior ou aba;
- entre 680 e 899 px: sidebar recolhível e request/response empilhados;
- abaixo de 680 px: layout de coluna única, com navegação em painel aberto sob demanda.

O conteúdo técnico não deve causar scroll horizontal acidental. Código e URLs podem quebrar ou rolar dentro de uma área controlada.

## 5.1. Onboarding de primeiro uso

Quando o Larry for aberto pela primeira vez sem workspace local configurado, mostrar uma mensagem de boas-vindas em um modal central. O modal deve orientar sem bloquear o uso do aplicativo.

Estrutura proposta:

```text
                 [logo Larry]

              Bem-vindo ao Larry
       API testing e diagnóstico local

                 COMEÇAR
        O que você deseja fazer?

 [Importar do Postman]       [Criar collection]
 [trazer collections]        [começar do zero]

             [Começar com uma request]

                 Explorar workspace
```

Regras do fluxo:

- usar a logo Larry em tamanho compacto;
- uma ação primária por opção, com título e descrição curta;
- oferecer `Explorar workspace` para quem não quer configurar nada;
- permitir fechar com `Esc` e continuar depois;
- não exibir o modal novamente após a escolha, salvo em `Configurações > Reabrir boas-vindas`;
- não exigir login, backend ou conexão com a nuvem;
- não usar gradiente, emoji, sparkles ou ilustração decorativa adicional;
- manter foco preso ao modal enquanto ele estiver aberto;
- indicar visualmente qual opção está selecionada pelo teclado;
- se um arquivo for importado, mostrar uma prévia e o número de requests antes de gravar.

Copy inicial:

- título: `Bem-vindo ao Larry`;
- subtítulo: `API testing e diagnóstico local`;
- seção: `COMEÇAR`;
- pergunta: `O que você deseja fazer?`;
- ação: `Importar do Postman`;
- descrição: `Traga uma collection existente para o Larry`;
- ação: `Criar collection`;
- descrição: `Comece uma collection local e versionável`;
- ação: `Começar com uma request`;
- descrição: `Teste um endpoint sem criar uma collection`;
- saída: `Explorar workspace`.

O texto deve continuar direto e sem travessão longo. O modal é um ponto de entrada, não uma tela de marketing.

### 5.2. Sistema de modais

O Larry usa um componente de modal compartilhado para decisões, avisos e erros que precisam interromper o fluxo. Não usar `window.alert`, `window.confirm` ou `window.prompt` na interface.

Contrato do componente:

- `neutral`: decisão comum ou informação que não representa risco;
- `info`: aviso explicativo, importação, compatibilidade ou próximo passo;
- `danger`: perda de dados, remoção ou envio potencialmente destrutivo;
- `error`: erro técnico que precisa de leitura e ação do usuário;
- título curto e específico;
- descrição que explica consequência e próximo passo;
- slot opcional para detalhes técnicos, código ou lista de impacto;
- ação primária nomeada pelo verbo real;
- ação secundária `Cancelar` ou `Fechar` quando aplicável.

Templates iniciais:

```text
Confirmação destrutiva
  CONFIRMAÇÃO
  Remover request?
  A request "Criar pagamento" será removida desta collection local.
  [Cancelar] [Remover request]

Aviso informativo
  ATENÇÃO
  Importar collection?
  Revise os itens convertidos antes de salvar o arquivo.
  [Cancelar] [Revisar importação]

Erro técnico
  ATENÇÃO
  Não foi possível carregar a collection
  mensagem técnica preservada + orientação de correção
  [Fechar] [Ver detalhes]
```

Regras de comportamento:

- foco inicial entra no modal, preferencialmente na ação secundária segura;
- `Esc` fecha quando a ação permitir cancelamento;
- clique no backdrop fecha apenas quando não houver risco de perda ou envio;
- Tab e Shift+Tab ficam presos dentro do modal;
- ao fechar, o foco retorna ao controle que abriu o modal;
- o modal usa superfície sólida, sem gradiente, blur ou glassmorphism;
- em janela estreita, o modal pode se alinhar à base, mas não pode ficar escondido pelo viewport;
- texto técnico nunca é substituído por uma mensagem genérica;
- ação destrutiva precisa de verbo explícito e não usa apenas cor para comunicar risco.

### 5.3. Selects e criação de requests

O Larry não usa `<select>` nativo na interface de trabalho. Todo seletor deve usar o componente `CustomSelect`, com trigger de 32 px, superfície `--color-surface-1`, popover `--color-surface-2`, estado de navegação em `--color-surface-3` e foco em `--color-brand`.

Contrato do `CustomSelect`:

- a prop `label` fornece o nome acessível para o trigger e para o listbox;
- o valor continua controlado por `v-model`, e mudanças também podem ser observadas pelo evento `change`;
- `Enter`, Espaço, `ArrowUp`, `ArrowDown` e `Escape` funcionam no teclado;
- clique fora fecha o popover e a seleção devolve foco ao trigger;
- o chevron é SVG linear, nunca um caractere Unicode usado como ícone;
- opções selecionadas exibem texto em `--color-brand` e um check SVG;
- opções de método podem usar tipografia monoespacial e tons semânticos existentes.

O botão `+` do cabeçalho da collection abre o modal compartilhado `NewRequestModal`. Ele é o ponto único para iniciar uma request HTTP, WebSocket, gRPC, SSE ou importar cURL. A V1 mantém o modal como entrada de fluxo: requests HTTP são inseridas na collection local, enquanto os outros protocolos abrem seus painéis existentes até que o modelo de collection suporte nós de protocolo versionados.

O modal usa os tokens do Larry, mantém nome e URL editáveis, apresenta método HTTP com `CustomSelect` e sempre oferece `Cancelar` como saída segura. O menu de contexto da collection e a persistência de folders seguem o mesmo contrato local-first.

## 6. Design tokens

### Cores

Usar variáveis CSS. Os nomes descrevem função, não aparência.

```css
:root {
  --color-bg: #0f1115;
  --color-surface-1: #15181e;
  --color-surface-2: #1b1f27;
  --color-surface-3: #222832;
  --color-border: #2c333e;
  --color-border-strong: #3b4552;

  --color-text: #e7eaf0;
  --color-text-muted: #9aa4b2;
  --color-text-subtle: #707b8b;

  --color-brand: #62d9dc;
  --color-brand-strong: #3fc0c5;
  --color-brand-warm: #f3ad58;

  --color-success: #4fc58a;
  --color-warning: #e3b76d;
  --color-danger: #ef8585;
  --color-info: #78a8f5;

  --color-code-bg: #0b0d11;
}
```

Regras:

- `--color-brand` é reservado para ação primária, foco e elemento ativo;
- `--color-brand-warm` deve aparecer em pequenos pontos de marca, não em grandes áreas;
- não usar todas as cores de status simultaneamente como decoração;
- contraste de texto normal deve ser validado contra a superfície usada;
- nenhum componente deve definir uma cor hexadecimal ad hoc sem justificativa.

### Tipografia

- interface: `Inter`, `Segoe UI`, sans-serif;
- código, URL, headers e JSON: `JetBrains Mono`, `Cascadia Code`, `Consolas`, monospace;
- título de tela: 20 a 28 px, peso 600;
- título de seção: 13 a 16 px, peso 600;
- texto de interface: 13 a 14 px;
- metadata: 11 a 12 px;
- código: 12 a 13 px, line-height 1.55.

Não usar texto todo em uppercase para títulos. Eyebrows podem existir apenas para contexto curto e consistente.

### Espaçamento

Base de 4 px:

```text
4  8  12  16  20  24  32  40
```

Padrões:

- campo e label: 6 px;
- campos relacionados: 8 a 12 px;
- seções: 20 a 24 px;
- padding de painel: 16 a 20 px;
- topbar: 48 a 56 px;
- sidebar: 248 a 288 px;
- alvo interativo mínimo: 32 px para desktop e 40 px quando houver toque.

### Bordas e raio

- raio de campo: 5 px;
- raio de painel: 8 px;
- raio de menu/popover: 8 px;
- raio de pill de status: 999 px apenas quando representar status curto;
- borda padrão: 1 px sólida e discreta;
- não usar `border-left` colorido como recurso de seleção;
- não usar sombra pesada para separar painéis.

## 7. Componentes padronizados

### Topbar

Contém apenas navegação, identidade, contexto e estado global.

Estrutura visual:

```text
[LARRY / LOCAL WORKBENCH]     [MINHA API / REQUEST ATIVA]     [environment] [janela]
```

- a marca à esquerda é uma assinatura de produto, não um botão decorativo;
- a região central mostra o contexto ativo e pode truncar no meio quando necessário;
- environment ou workspace local fica à direita;
- controles de janela ficam no shell customizado e chamam APIs Tauri tipadas;
- ações globais como abrir, salvar e settings entram depois, quando tiverem comportamento real.

No Tauri, a decoração nativa fica desativada para evitar uma faixa visual do Windows separada da identidade do Larry. A topbar web assume apenas arrastar, minimizar, maximizar/restaurar e fechar por meio das APIs de janela do Tauri. Nenhuma ação de sistema é simulada sem chamada real.

Não colocar slogan, anúncio, chat ou ação sem função real na topbar.

Não manter botões de menu, início ou busca que ainda não têm comportamento. Um controle visual sem efeito cria expectativa falsa e reduz a densidade útil.

### Barra inferior e status

Quando houver statusbar, ela deve ocupar a largura total e permanecer separada do scroll dos painéis. Pode exibir ambiente, branch, encoding, quantidade de requests ou atalhos, mas não deve competir com o botão `Enviar`. Não duplicar na topbar um estado genérico como `Pronto`; se a informação não muda uma decisão do usuário, ela não merece ocupar o shell.

### Sidebar e árvore da collection

Prioridade visual:

1. collection/workspace atual;
2. busca;
3. árvore de folders e requests;
4. ações menos frequentes em menu contextual.

No Larry, a sidebar possui um único propósito: a árvore da collection. Não há abas `Requests` e `Ferramentas locais`.

O cabeçalho compacto da árvore contém:

- nome e ícone da collection;
- ação de arquivo/Git para abrir o YAML local;
- ação `+` para criar requests HTTP, WebSocket, gRPC e SSE ou importar cURL;
- menu contextual por botão de overflow ou clique direito para criar request, criar pasta e abrir o YAML.

Requests de protocolos diferentes são irmãos na mesma árvore e usam badges curtos, como `GET`, `POST`, `WS`, `gRPC` e `SSE`. O menu da request mantém `Renomear`, `Duplicar`, `Mover para pasta` e `Remover` fora da linha principal.

Folders também possuem um menu de overflow contextual, visível em hover, foco ou estado aberto, com `Nova request nesta pasta`, `Nova subpasta`, `Renomear` e `Remover`.

Criar uma request a partir de uma pasta mantém o destino contextual até a confirmação do modal; criar subpasta preserva a hierarquia recursiva no arquivo local.

Remover uma pasta sempre exige confirmação e não pode deixar a collection sem nenhuma request executável. A remoção inclui as subpastas contidas e seleciona uma request restante quando a request ativa for afetada.

O schema atual armazena requests de raiz e pastas recursivas. A expansão de cada pasta é estado local da sessão; a ordem e a hierarquia são preservadas no YAML quando a collection é salva.

Arquivo da collection e environment são responsabilidades contextuais, não itens permanentes da navegação.

O arquivo YAML abre em um modal persistente pelo botão do cabeçalho. O environment abre pela topbar, ao lado do seletor `Sem environment`. O estado do formulário continua montado enquanto o modal fecha, evitando perda silenciosa de edição.

O fluxo file-first usa o diálogo nativo do sistema para abrir uma collection existente ou escolher o destino do primeiro salvamento. O caminho exibido é somente leitura depois da seleção. A collection exibe um indicador discreto de dirty state quando requests, folders ou nome foram alterados desde o último salvamento. O auto-save é uma preferência explícita, desativada por padrão, com debounce e sem criar um arquivo antes da escolha do destino.

A revisão Git mostra duas camadas: o diff técnico original para investigação fiel e um diff semântico para leitura rápida. O segundo agrupa alterações por collection, folder, request e campo. Valores potencialmente sensíveis são omitidos ou mascarados, e uma falha de interpretação semântica nunca esconde o diff técnico.

Rows:

- altura de 32 a 36 px;
- uma ação primária implícita: selecionar;
- ações secundárias ficam em um único menu de overflow de três pontos, visível em hover, foco ou na request ativa;
- o menu agrupa `Renomear`, `Duplicar` e `Remover`; nunca renderizar esses três verbos como botões permanentes na row;
- `Renomear` abre um input inline no card da request e confirma com Enter ou cancela com Esc;
- nome truncado com tooltip;
- método como texto curto, nunca como emoji.

### Botões

Contratos de tamanho:

- botão compacto: 28 a 32 px de altura;
- botão padrão: 32 a 36 px;
- ação primária: mesma altura dos campos adjacentes, com contraste de superfície;
- botão de ícone: área mínima de 32 px no desktop e 40 px quando houver toque;
- grupos de ações devem compartilhar baseline e espaçamento de 4 a 8 px.

Hierarquia:

- primário: uma ação principal por região, normalmente `Enviar`, `Salvar` ou `Importar`;
- secundário: ações de apoio com superfície neutra;
- terciário: ações de baixo risco e baixa frequência, preferencialmente em menu contextual;
- destrutivo: texto explícito, confirmação contextual e cor de apoio restrita ao estado.

Estados obrigatórios:

- idle;
- hover;
- focus-visible;
- pressed;
- disabled;
- loading;
- success ou conclusão quando aplicável.

Hover de controles secundários não deve pintar uma nova superfície nem usar a cor de marca como decoração. Para o tema escuro, manter fundo transparente, borda neutra e texto em tom normal/muted com contraste suficiente. A ação primária pode conservar sua superfície de marca; ações destrutivas conservam o sinal de perigo apenas no menu ou confirmação correspondente.

O botão não deve mudar de largura de forma brusca ao entrar em loading. Use um rótulo curto e preserve o alinhamento do formulário.

### Inputs, selects e textareas

- altura padrão entre 32 e 36 px;
- label acima do campo, com 6 px de distância;
- placeholder não substitui label;
- borda neutra no idle e borda de marca apenas no foco;
- erro com texto próximo ao campo, não apenas com borda vermelha;
- disabled com contraste suficiente para leitura, sem parecer um campo quebrado;
- texto técnico, URL e headers devem usar fonte monoespaçada quando isso melhorar a leitura;
- selects devem usar o mesmo ícone de expansão do sistema visual, nunca `⌄`, `▼` ou outro caractere Unicode como solução provisória;
- textarea de JSON deve preservar scroll interno e não expandir o documento sem limite;
- campos longos precisam de truncamento ou scroll horizontal local controlado.

### Editor de environment

- o environment ativo deve aparecer na topbar e acompanhar a execução da request;
- o arquivo YAML e os secrets devem ser apresentados como duas responsabilidades diferentes;
- variáveis públicas podem mostrar o valor no editor;
- secrets exibem apenas o nome da referência e um campo de substituição mascarado;
- carregar um environment nunca deve preencher a UI com o valor existente de um secret;
- remover uma referência do arquivo não deve apagar uma credencial silenciosamente;
- salvar deve informar que o valor público vai para o arquivo e o secret vai para o armazenamento nativo;
- erros de validação aparecem próximos ao editor e mantêm a categoria técnica retornada pelo core;
- o estado sem environment precisa ser válido e explícito, sem bloquear uma request que não usa placeholders.

### Interações e feedback

Toda interação precisa responder visualmente e, quando relevante, semanticamente:

- hover mostra que o elemento é interativo;
- focus-visible mostra onde o teclado está;
- pressed mostra a ação em andamento;
- selected diferencia seleção de hover sem depender apenas de cor;
- disabled explica por que a ação não pode ocorrer quando isso não for óbvio;
- loading impede duplicação de uma ação que ainda está em execução;
- dirty/unsaved aparece no contexto da collection ou request;
- erro preserva a mensagem técnica original e oferece uma próxima ação;
- confirmação aparece próxima da ação que pode causar perda ou envio perigoso.

Não usar tooltip como única forma de explicar uma ação primária. Tooltip complementa label visível ou `aria-label`.

### Setas, chevrons e iconografia de controle

Setas de navegação, expansão e seleção devem vir de um conjunto único de SVG lineares ou de uma biblioteca escolhida para o projeto.

- `chevron-down` para selects e menus;
- `chevron-right` para árvore recolhida;
- `chevron-down` para árvore expandida;
- `minus`, `square` e `x` para controles da janela;
- não usar caracteres `⌄`, `›`, `→` ou similares como ícones funcionais;
- ícones interativos precisam de nome acessível, tooltip quando o significado não for óbvio e estado de foco;
- não combinar SVG filled com outline sem uma decisão registrada;
- o ícone não deve deslocar o baseline do texto do controle.

### Request header

```text
[GET] [URL........................................] [Enviar]
```

- método com cor textual discreta;
- URL ocupa o espaço principal;
- `Enviar` é o único botão primário;
- ações como duplicar, importar e salvar ficam secundárias;
- ao executar, o botão muda para estado de progresso textual.

### Request tabs

Ordem inicial:

```text
Params · Body · Headers · Auth · Variables · Scripts · Tests
```

Para protocolos diferentes:

- WebSocket: Connection, Messages, Headers, Auth;
- GraphQL: Query, Variables, Headers, Auth;
- gRPC: Service, Method, Message, Metadata, Stream;
- resposta: Body, Headers, Trace, Diagnostics, Raw.

Não criar abas que apenas escondem um campo sem aumentar a compreensão.

No MVP HTTP, a ordem real é `Params`, `Headers`, `Auth`, `Cookies`, `Body`. Cada aba contém uma responsabilidade inteira e indica sua contagem quando isso ajuda a leitura. A autenticação comum fica separada dos headers para deixar explícita a origem dos credentials e permitir a evolução para OAuth. Cookies ficam em uma aba própria porque são estado de sessão e não devem ser confundidos com headers arbitrários. O Body usa editor de código para JSON/texto e tabela compacta para forms, sem criar um segundo fluxo visual. O nome da request fica no contexto da topbar porque identifica a peça que está sendo editada, não uma configuração opcional.

### Response inspector

O cabeçalho precisa responder rapidamente:

```text
200 OK    212 ms    18.4 KB    HTTP/2
```

Depois:

- body formatado;
- headers;
- timeline de rede;
- diagnóstico por camada;
- raw apenas quando necessário.

Status deve aparecer como texto. Cor é apoio, não substituto.

No Larry, esse painel se chama **Inspector**. `Body`, `Headers`, `Trace`, `Diagnostics`, `Protocol Lab` e `Profiler` são evidências ou análises da mesma request e usam a mesma família visual de abas. O cabeçalho chama a atenção para o estado, as métricas medidas e um seletor compacto de histórico vindo do SQLite local.

### Protocol Lab

O Protocol Lab é uma aba técnica do Inspector que organiza a evidência da execução em quatro camadas: `Application`, `Transport`, `Internet` e `Network Access`. Ele acompanha a request e a resposta atuais sem retirar o usuário do fluxo de análise.

Regras do painel:

- mostrar primeiro a request atual e o estado da resposta;
- separar fatos observados de explicações sobre a camada;
- exibir a proveniência ao lado de cada fato importante;
- usar `Indisponível` quando o adapter não mede aquela etapa;
- nunca desenhar TCP, TLS, rota, Wi-Fi ou bytes físicos como se tivessem sido capturados quando não foram;
- explicar que TTFB é o tempo observado até os headers e não prova o tempo exclusivo do servidor;
- manter o painel denso, com blocos de evidência e leitura vertical, sem gráfico decorativo ou efeito de rede simulado;
- reservar comparação HTTP/1.1, HTTP/2 e HTTP/3 para um contrato futuro de conexão e streams.

O painel usa a mesma superfície e densidade do Inspector. A identidade do Larry aparece na hierarquia de evidências, na linguagem de proveniência e na separação entre medição e lacuna de observabilidade.

### Profiler

O Profiler é uma aba do Inspector. Ele opera sobre a request ativa, mantém os limites de segurança visíveis e não registra cada amostra no histórico normal. A aba deve mostrar destino, quantidade, concorrência, progresso, cancelamento e resumo estatístico sem transformar a sidebar em um launcher de ferramentas.

### Comparação controlada de protocolos

Quando houver uma resposta ou uma request pronta, o Protocol Lab pode oferecer uma ação explícita para comparar HTTP/1.1 e HTTP/2. Essa ação executa duas requests reais e deve deixar o destino visível antes do envio.

Regras:

- usar um único botão primário: `Comparar protocolos`;
- explicar que serão feitas duas execuções contra o endpoint atual;
- mostrar HTTP/1.1 e HTTP/2 em colunas equivalentes;
- exibir status, versão observada, tempo, tamanho do body e erro de cada execução;
- mostrar diferenças como `HTTP/2 - HTTP/1.1`, com unidade e direção;
- não apresentar uma execução mais rápida como regra geral;
- preservar o erro técnico do modo que falhou;
- não gravar body, headers ou resultado resumido no histórico normal;
- reservar conexões, streams, multiplexação e reuso para o laboratório HTTP/2 instrumentado.

### Trace timeline

A timeline é o diferencial do Larry e deve ter aparência de instrumento técnico, não de gráfico decorativo.

```text
DNS       18 ms       measured
TCP       25 ms       measured
TLS       41 ms       observed
TTFB     120 ms       measured
Download   8 ms       measured
Total    212 ms
```

Cada linha deve mostrar:

- etapa;
- duração;
- unidade;
- origem da métrica;
- estado indisponível quando não puder ser observada.

Não afirmar que TTFB é exatamente o tempo de processamento do servidor.

### Status e feedback

Estados obrigatórios:

- idle;
- loading;
- success;
- warning;
- error;
- unavailable;
- disabled;
- dirty/unsaved.

Cada estado deve ter texto e diferença de superfície. Não usar somente um ponto colorido.

## 8. Iconografia

Usar ícones apenas quando reduzem o tempo de leitura ou representam uma ação conhecida.

Permitidos:

- abrir arquivo;
- salvar;
- buscar;
- expandir/contrair;
- executar;
- copiar;
- fechar;
- configurações;
- Git;
- rede;
- erro e aviso.

Regras:

- preferir um único conjunto de ícones lineares;
- todos os ícones interativos têm label acessível e tooltip;
- não usar emojis como ícones funcionais;
- não usar sparkles, varinhas, estrelas ou brilhos como decoração funcional;
- não misturar ícones filled e outline sem motivo;
- não colocar ícone em todo texto só para preencher espaço.

O conjunto de ícones deve ser avaliado como sistema. Antes de adicionar um ícone novo, verificar peso do traço, caixa visual, tamanho, alinhamento, estado hover e comportamento em fundo escuro.

## 9. Linguagem de interface

### Usar

- verbos concretos: `Enviar`, `Salvar`, `Abrir`, `Duplicar`, `Comparar`;
- diagnósticos com causa e camada: `Falha ao resolver DNS`;
- origem de métrica: `medido`, `observado`, `inferido`, `indisponível`;
- confirmação de risco: `Executar 100 requests em staging?`;
- feedback que explica o próximo passo.

### Evitar

- emojis;
- travessão longo em textos de interface;
- frases de marketing na área de trabalho;
- nomes como `magic` ou `copilot` sem uma funcionalidade explícita;
- textos que escondem erros técnicos;
- excesso de exclamações;
- botões com rótulos vagos como `Continuar` quando a ação pode ser nomeada.

## 10. Segurança visível

O design precisa comunicar segurança sem criar alarmismo:

- environment de produção sempre visível;
- secrets mascarados por padrão;
- alerta textual antes de copiar/exportar credenciais;
- estado de collection não salva claramente indicado;
- trace informa quando uma métrica exige proxy ou captura;
- ações potencialmente destrutivas exigem confirmação contextual.

## 11. Diferencial visual do Larry

Bruno e Insomnia já ensinam a organizar collections, requests e responses. O Larry deve acrescentar uma camada visual própria:

1. **Response com trace:** a resposta não termina no body.
2. **Diagnóstico por camada:** DNS, TCP, TLS, protocolo e aplicação aparecem separados.
3. **Proveniência:** cada métrica informa como foi obtida.
4. **Comparação:** duas execuções podem ser vistas lado a lado sem perder status, headers, bytes e latência.
5. **Modo de aprendizado:** a mesma request pode revelar a camada de aplicação, transporte e rede.

O diferencial deve aparecer na hierarquia do response inspector, não em efeitos visuais chamativos.

## 12. Checklist para novos componentes

- [ ] Usa tokens em vez de cores isoladas.
- [ ] Tem estado idle, loading, sucesso e erro quando aplicável.
- [ ] Não usa gradiente, glassmorphism ou sombra decorativa.
- [ ] Não usa emoji, sparkles ou ícone sem função.
- [ ] Não usa borda lateral colorida como decoração.
- [ ] Não usa travessão longo em copy da interface.
- [ ] Tem foco visível e label acessível.
- [ ] Não depende só de cor para status.
- [ ] Funciona com nomes, URLs e erros longos.
- [ ] Preserva a densidade de uma ferramenta desktop.
- [ ] Expõe a origem das métricas quando apresenta observabilidade.
- [ ] Tem comportamento definido em 680, 900 e 1180 px.
- [ ] Não cria scroll global no workspace.
- [ ] Cada painel longo tem scroll independente e `min-height: 0` onde necessário.
- [ ] O shell não usa padding externo que reduza a área útil sem motivo.
- [ ] Botões e campos compartilham altura, baseline e estados.
- [ ] Hover, focus-visible, pressed, disabled e loading foram verificados.
- [ ] Setas e chevrons vêm do sistema de ícones, não de caracteres Unicode improvisados.
- [ ] Ícones de ação têm nome acessível e tooltip quando necessário.
- [ ] O conteúdo continua utilizável com teclado.

## 13. Ordem de aplicação

1. Criar tokens globais e reset de tema escuro.
2. Implementar onboarding de primeiro uso e persistência da escolha local.
3. Reestruturar shell em topbar, sidebar, workspace e response inspector.
4. Corrigir altura do shell e mover scroll para sidebar, request e response.
5. Remover padding externo do workspace e revisar divisores entre regiões.
6. Padronizar campos, botões, tabs, rows e feedback.
7. Padronizar SVGs de ação, chevrons, tooltips e estados de foco.
8. Reposicionar a collection atual na sidebar.
9. Mover Git para um painel contextual, sem competir com o editor.
10. Criar response tabs para Body, Headers, Trace, Diagnostics, Protocol Lab e Profiler.
11. Adicionar histórico compacto ao cabeçalho do Inspector.
12. Implementar timeline com dados reais e provenance.
13. Validar visualmente em 320, 360, 390, 414 e desktop antes de expandir protocolos.

Este documento define o padrão visual. Qualquer exceção deve ser registrada em `docs/decisions.md` com motivo, impacto e alternativa considerada.
