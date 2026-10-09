# Arquitetura

O Larry é um aplicativo desktop local-first. O Rust roda dentro do executável Tauri e não representa um backend remoto.

```text
Vue/TypeScript
    ↓ Tauri IPC
Core Rust
    ↓ adapters locais
Rede, descriptors protobuf, filesystem, SQLite e Credential Manager
```

## Fronteiras

- Vue cuida de apresentação, interação e estado de tela;
- serviços frontend concentram chamadas IPC;
- comandos Tauri validam entradas e conectam a UI ao core;
- domínio Rust contém modelos e regras independentes de UI;
- adapters Rust executam HTTP, persistência, Git, importação e secrets;
- o adapter `execution/profiler.rs` coordena execuções HTTP limitadas, sem gravar cada amostra no histórico;
- adapters de protocolo, como `execution/grpc.rs`, executam descritores e transporte local sem gerar serviços estáticos;
- o `ProtocolLabPanel` consome `HttpResponse.trace` e não abre rede diretamente; ele organiza evidências já produzidas pelo core em camadas educacionais;
- o adapter HTTP expõe `auto`, `http1` e `http2`; o comando de comparação resolve a request uma vez, executa os dois modos e retorna somente amostras resumidas;
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

O trace atual mede DNS preflight, tempo até os headers, download e total. TCP, TLS detalhado e reuso de conexão permanecem explicitamente indisponíveis até existir um adapter instrumentado que consiga medir essas fases sem apresentar inferências como fatos. O profiler local reaproveita esse executor e expõe estatísticas agregadas, mas não transforma uma medição de TTFB em tempo exclusivo de servidor nem presume reuso de conexão.

## Evolução

Novos protocolos devem entrar atrás de adapters próprios. A UI deve consumir um modelo de evidência comum sem assumir que todo protocolo possui as mesmas fases de HTTP/1.1.

O primeiro lote do Protocol Lab é deliberadamente somente de apresentação. Ele não cria uma nova medição nem transforma uma lacuna do adapter em inferência visual. A futura comparação entre HTTP/1.1, HTTP/2 e HTTP/3 exigirá um contrato de eventos que diferencie conexão, stream, multiplexação, handshake e reuso.

O comparador controlado HTTP/1.1 versus HTTP/2 é uma exceção limitada: ele executa duas requests reais e compara o resultado agregado, mas ainda não observa uma conexão compartilhada nem os streams internos do HTTP/2. Por isso, ele não deve ser descrito como packet capture ou como laboratório completo de transporte.

O primeiro adapter gRPC carrega `.proto` em runtime, monta um `DescriptorPool` e usa `DynamicMessage`. A UI só acessa essa capacidade pela camada `services/ipc.ts`; ela não lê arquivos nem abre sockets diretamente.
