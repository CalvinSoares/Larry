# Larry API Client

O Larry é um API Client desktop local-first para testar APIs, investigar falhas de rede e entender o que acontece durante uma requisição.

Ele combina um editor de requests com evidências técnicas de execução: DNS, tempo até os headers, download, resposta HTTP e diagnósticos por camada. O objetivo não é ser apenas mais um clone de Postman; é aproximar API testing, network debugging e aprendizado de protocolos.

> O projeto está em desenvolvimento. A versão atual é um MVP inicial e ainda não representa o conjunto final de protocolos planejado.

## Princípios

- funciona localmente, sem backend, login ou telemetria obrigatórios;
- executa rede, filesystem e secrets no core Rust através do Tauri IPC;
- mantém collections legíveis e versionáveis pelo Git;
- mantém secrets fora de collections, histórico e logs;
- preserva o erro técnico original junto de uma explicação amigável;
- identifica a origem das métricas como medida, observada, inferida ou indisponível.

## Estado atual

O MVP atual inclui:

- requests HTTP e HTTPS com GET, POST, PUT, PATCH e DELETE;
- query params, headers e body JSON/texto;
- collections YAML locais;
- duplicação, renomeação, seleção e remoção de requests;
- environments YAML com referências a secrets no Credential Manager;
- histórico local SQLite com replay e comparação;
- importação com prévia de collections Postman v2.1;
- integração read-only com o estado Git da collection;
- trace HTTP inicial com DNS preflight, TTFB, download, total, IPs resolvidos e versão HTTP;
- diagnóstico inicial para falhas de DNS, transporte e status HTTP;
- interface desktop em Vue 3, TypeScript, Vite e Tauri 2.

TCP, TLS detalhado, reuso de conexão, WebSocket, GraphQL, gRPC, HTTP/3 e profiler de carga ainda estão no roadmap. As fases indisponíveis aparecem explicitamente na interface; não são apresentadas como medições exatas.

## Requisitos

- Node.js compatível com o Vite instalado;
- npm ou pnpm;
- Rust e Cargo;
- dependências de desenvolvimento do Tauri 2 para o seu sistema operacional.

## Desenvolvimento rápido

```bash
npm install
npm run dev
```

Para executar o desktop com Tauri:

```bash
npm run tauri dev
```

Validações principais:

```bash
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo test --manifest-path src-tauri/Cargo.toml
```

## Estrutura

```text
src/             interface Vue e ponto de entrada do frontend
src-tauri/       core Rust, comandos Tauri e configuração do desktop
public/assets/   logo e assets públicos
docs/            arquitetura, design, evolução e guias públicos
.github/         automações e templates da comunidade
```

## Documentação

Comece pelo [índice de documentação](docs/README.md).

- [Arquitetura e decisões](docs/architecture.md)
- [Desenvolvimento](docs/development.md)
- [Design do produto](docs/design.md)
- [Formato das collections](docs/collection-format.md)
- [Segurança](docs/security.md)
- [Plano e roadmap](docs/roadmap.md)

## Contribuição

Leia [CONTRIBUTING.md](CONTRIBUTING.md) antes de abrir um Pull Request. Para dúvidas e propostas, use as Discussions quando estiverem habilitadas. Para bugs, abra uma issue com passos de reprodução e uma fixture local quando possível.

Não envie tokens, cookies, secrets, collections privadas ou logs sensíveis. Vulnerabilidades devem seguir [SECURITY.md](SECURITY.md), nunca uma issue pública.

## Licença

O Larry é distribuído sob a licença [MIT](LICENSE).
