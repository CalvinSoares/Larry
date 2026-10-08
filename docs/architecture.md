# Arquitetura

O Larry é um aplicativo desktop local-first. O Rust roda dentro do executável Tauri e não representa um backend remoto.

```text
Vue/TypeScript
    ↓ Tauri IPC
Core Rust
    ↓ adapters locais
Rede, filesystem, SQLite e Credential Manager
```

## Fronteiras

- Vue cuida de apresentação, interação e estado de tela;
- serviços frontend concentram chamadas IPC;
- comandos Tauri validam entradas e conectam a UI ao core;
- domínio Rust contém modelos e regras independentes de UI;
- adapters Rust executam HTTP, persistência, Git, importação e secrets;
- nenhum backend Larry é necessário para executar uma request.

## Princípios

- validar URL, path, tamanho e permissão no Rust;
- manter comandos Tauri pequenos e tipados;
- preservar o erro técnico original;
- declarar a proveniência das métricas;
- manter collections legíveis fora do banco;
- versionar schemas e migrations;
- não colocar secrets em collections, histórico, logs ou fixtures.

## Observabilidade de rede

O trace atual mede DNS preflight, tempo até os headers, download e total. TCP, TLS detalhado e reuso de conexão permanecem explicitamente indisponíveis até existir um adapter instrumentado que consiga medir essas fases sem apresentar inferências como fatos.

## Evolução

Novos protocolos devem entrar atrás de adapters próprios. A UI deve consumir um modelo de evidência comum sem assumir que todo protocolo possui as mesmas fases de HTTP/1.1.
