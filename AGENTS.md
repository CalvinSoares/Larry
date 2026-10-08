# AGENTS — guia do mentor técnico e executor

O usuário quer aprender o projeto, mas autorizou o agente a implementar os lotes de código. O agente deve construir em fases coerentes, testar o resultado e explicar detalhadamente tudo que foi feito para que o usuário possa ler, revisar e entender.

## Stack oficial

- Vue 3;
- TypeScript;
- Vite;
- Tauri 2;
- Rust;
- Tokio;
- SQLite e arquivos locais;
- Credential Manager/Keychain/libsecret para secrets.

## Papel do agente

O agente deve:

- explicar o motivo de cada decisão;
- agrupar mudanças relacionadas em tarefas/fases coerentes quando o usuário pedir avanço mais rápido;
- indicar comandos e resultado esperado;
- explicar erros a partir das evidências;
- implementar código do produto quando autorizado pelo usuário;
- revisar tanto código próprio quanto código escrito pelo usuário;
- propor exercícios;
- não avançar automaticamente de fase;
- atualizar a documentação quando solicitado.

Ao terminar cada tarefa ou fase, o agente deve apresentar:

- resumo do objetivo e resultado;
- todos os arquivos criados, alterados ou removidos;
- descrição das mudanças relevantes por arquivo;
- decisões de arquitetura e trade-offs;
- testes executados e resultado;
- riscos, limitações e próximos pontos de atenção;
- explicação didática dos conceitos novos;
- comandos para revisar, salvar e criar um commit granular.

O usuário continua sendo responsável por ler, questionar, testar e confirmar cada lote. A implementação automática não elimina a revisão guiada.

Por padrão, o agente não deve:

- criar a aplicação inteira;
- esconder decisões em boilerplate;
- pular testes;
- adicionar abstrações antes de existir uma implementação simples.

Quando o usuário pedir uma tarefa de implementação, o agente pode aplicar patches completos para o lote solicitado. O lote deve permanecer limitado a uma fase ou capacidade relacionada; não juntar funcionalidades não testadas apenas para produzir mais código. A explicação detalhada vem no final da tarefa, e não precisa bloquear cada arquivo individualmente.

## Formato de execução das tarefas

1. Declarar o escopo do lote antes de alterar arquivos.
2. Inspecionar o estado atual e preservar mudanças existentes.
3. Implementar a capacidade completa do lote, incluindo tratamento de erro proporcional.
4. Executar build, testes e verificações relevantes.
5. Revisar diff, segurança, separação frontend/core e compatibilidade.
6. Entregar o relatório didático completo.
7. Fornecer comandos de commit granular.
8. Esperar confirmação antes de iniciar a próxima fase.

## Regras invariáveis

1. Não exigir backend, login ou telemetria para o uso local.
2. Nunca colocar secrets reais em collections, fixtures, logs, snapshots ou testes.
3. Toda métrica deve declarar se é medida, observada, inferida ou indisponível.
4. Preservar o erro técnico original junto da explicação amigável.
5. Não usar privilégio elevado para o trace próprio.
6. Proxy, MITM e packet capture são modos opt-in.
7. Profiler exige limite, confirmação, cancelamento e aviso de destino.
8. Vue não acessa rede, secrets ou filesystem diretamente; usa Tauri IPC.
9. Mudanças de schema/eventos/SQLite exigem versionamento e migration.
10. Collections continuam legíveis e editáveis fora do app.

## Convenções Vue

- usar `<script setup lang="ts">`;
- preferir Composition API para novas features;
- manter componentes pequenos e orientados a uma responsabilidade;
- mover lógica reutilizável para `composables/`;
- não colocar chamadas Tauri diretamente em dezenas de componentes;
- criar uma camada de services/composables para IPC;
- usar Pinia somente quando o estado realmente atravessar muitas features;
- começar com CSS simples antes de adotar um design system grande.
- seguir `docs/design.md` para tema escuro, tokens, layout, iconografia e linguagem;
- aplicar `.cursor/skills/ui-visual-system-validation/SKILL.md` em mudanças de UI, validando scroll por região, densidade, controles, ícones e acessibilidade;
- não adicionar gradientes, emojis, sparkles, copy com travessão longo ou borda lateral decorativa;
- validar foco, overflow, estados e densidade desktop antes de considerar uma tela pronta.

## Convenções Rust/Tauri

- validar URLs, paths, tamanhos e permissões no Rust;
- comandos Tauri pequenos e tipados;
- separar domínio, execução e adapters de transporte;
- eventos de execução devem carregar `run_id`, timestamp monotônico, fase, fonte e confiança;
- permitir cancelar requests, streams e profiler;
- capabilities mínimas e específicas por janela;
- nunca expor shell arbitrário ao frontend.

## Definition of Done

Uma tarefa está pronta quando:

- o caminho feliz funciona;
- timeout, cancelamento e erro têm cobertura;
- o fluxo local/offline foi preservado;
- secrets não aparecem em logs/export;
- métricas têm provenance;
- existe fixture reproduzível;
- migrations/import/export seguem compatíveis;
- documentação e checklist foram atualizados.

## Checklist de revisão

- [ ] A mudança é diferencial ou apenas paridade necessária?
- [ ] O usuário consegue explicar a decisão?
- [ ] O frontend continua separado do core Rust?
- [ ] Existe teste de erro?
- [ ] Há cancelamento?
- [ ] Existe risco de secret em log, diff, clipboard ou histórico?
- [ ] A métrica é medida ou inferida?
- [ ] HTTP/2 connection/stream foi considerado?
- [ ] Collection, SQLite e eventos têm versão?
- [ ] Tauri capabilities permanecem mínimas?

