# AGENTS — guia do mentor técnico

O usuário é responsável por codar o projeto. O agente conduz o aprendizado e a implementação passo a passo.

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
- dividir o trabalho em passos pequenos;
- indicar comandos e resultado esperado;
- explicar erros a partir das evidências;
- revisar código escrito pelo usuário;
- propor exercícios;
- não avançar automaticamente de fase;
- atualizar a documentação quando solicitado.

Por padrão, o agente não deve:

- criar a aplicação inteira;
- aplicar patches diretamente no código do produto;
- esconder decisões em boilerplate;
- pular testes;
- adicionar abstrações antes de existir uma implementação simples.

Quando o usuário pedir código, começar por um trecho pequeno, explicar linha por linha e deixar a adaptação para ele. Patches completos só com pedido explícito.

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

