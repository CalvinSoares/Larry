# CHECK — mercado e viabilidade

**Data:** 2026-09-29

## 1. Conclusão

REST, GraphQL, gRPC, WebSocket, collections, environments, scripts e Git já são cobertos em diferentes graus por Postman, Insomnia e Bruno. Esses recursos são necessários para adoção, mas não justificam sozinhos um novo produto.

A oportunidade é combinar:

1. **Request Path Trace:** DNS, TCP, TLS, protocolo, TTFB, download, pool e reuso.
2. **Diagnóstico por camada:** explicar se a falha ocorreu em DNS, TCP, TLS, HTTP ou aplicação.
3. **Protocol Lab:** comparar HTTP/1.1, HTTP/2 e futuramente HTTP/3 em cenários controlados.
4. **Replay e comparação:** staging/produção, respostas, headers, latência e trace.
5. **Collections versionáveis:** arquivos legíveis, diff semântico e secrets fora do Git.
6. **Modo educacional:** relacionar a execução real às camadas de Application, Transport, Internet e Access.

Toda métrica deve informar sua origem: `measured`, `observed`, `inferred`, `unavailable` ou `requires_capture`.

## 2. Comparação inicial

### Postman

Postman documenta suporte a HTTP, GraphQL, gRPC, WebSocket, Socket.IO e MQTT, além de scripts, collections, Vault, monitors e captura via proxy/Interceptor.

Pontos fortes: cobertura ampla, maturidade e ecossistema.

Oportunidade: experiência mais local-first, menor dependência de workspace/cloud e trace de transporte mais explícito.

Fontes: [protocolos](https://learning.postman.com/v11/docs/use/send-requests/protocols/protocols), [gRPC](https://learning.postman.com/v11/docs/use/send-requests/protocols/grpc/grpc-client-overview), [visão geral](https://learning.postman.com/v11/docs/use/overview).

### Insomnia

Insomnia documenta API Collections, HTTP, GraphQL, gRPC, WebSocket, ambientes, scripts, local vault, Git Sync e CLI.

Pontos fortes: fluxo de desenvolvimento e opções local/Git/cloud.

Oportunidade: trace de rede por fase, diagnóstico pedagógico e comparação de evidências.

Fontes: [API Collections](https://developer.konghq.com/insomnia/collections/), [environments](https://developer.konghq.com/insomnia/environments/), [site/documentação](https://developer.konghq.com/insomnia/).

### Bruno

Bruno é especialmente forte em collections file-first, Git, operação offline, `.bru`, REST, GraphQL, gRPC, proto files, streams, WebSocket, SSE, scripts, testes e CLI. Também possui uma timeline para request, chamadas feitas por scripts e console.

Pontos fortes: Git-native, local-first e baixo lock-in.

Oportunidade: diferenciar a timeline de script/produto de uma timeline de transporte com evidência e limites explícitos.

Fontes: [stack](https://github.com/usebruno/bruno/blob/main/contributing.md), [documentação](https://docs.usebruno.com/llms.txt), [Git](https://docs.usebruno.com/git-integration/overview.md).

## 3. Matriz de decisão

- Requests HTTP, headers, params, bodies e auth: maduros; implementar compatibilidade.
- Collections e environments: maduros; diferenciar por formato aberto e diff semântico.
- Scripts e assertions: maduros; começar com sandbox limitada.
- WebSocket e SSE: viáveis no MVP/V2; eventos persistentes e cancelamento são importantes.
- gRPC dinâmico: tecnicamente viável com descriptors, `DynamicMessage` e reflection opcional; exige spike cedo.
- Git: essencial para collections; não deve ser obrigatório para usar o app.
- Performance: runner existe nos concorrentes; o diferencial será profiler local seguro com percentis e trace.
- DNS/TCP/TLS: parcialmente observáveis dentro do próprio cliente; packet capture exige outro modo.
- Response inspector: Body e Headers já podem ser exibidos com dados observados; Trace e Diagnostics só devem sair do estado indisponível quando o core fornecer eventos e provenance reais.

### Importação e onboarding

O padrão de boas-vindas observado no Bruno é uma boa referência de redução de fricção: importar uma collection existente, criar uma collection nova ou começar com uma request. O Larry deve adotar a mesma ideia de orientação, mas com sua marca, tema escuro e foco em uso local.

Suporte Postman é justificável como estratégia de adoção. O Postman documenta exportação de collections e environments como arquivos JSON e mantém schemas de collection versionados. O primeiro importador Larry deve priorizar Postman Collection v2.1, sem depender de login ou API Postman.

Escopo recomendado:

- MVP: importação local de collection JSON v2.1, pastas, requests, método, URL, query, headers e body;
- MVP: prévia, validação, relatório de conversão e proteção contra sobrescrita;
- fase seguinte: environments, variáveis, auth mapeável e cookies;
- posterior: scripts `pm.*`, exemplos, mocks, workflows e formato Postman 3.0;
- fora do escopo local-first: login Postman, sync Cloud e execução através do Postman.

O diferencial não será “suportar Postman melhor que o Postman”. Será permitir migrar sem perder visibilidade sobre o que foi convertido e, depois, trabalhar com collections Larry legíveis e diagnosticáveis.

## 4. Limitações de observabilidade

O trace próprio consegue medir o que o cliente controla:

- DNS executado pelo próprio resolver;
- conexão TCP;
- handshake TLS;
- certificado, cipher e ALPN quando expostos;
- envio, primeiro byte e download;
- payload contado nos streams;
- reuso conhecido pelo pool próprio.

Sem proxy ou captura, não devemos afirmar:

- bytes físicos exatos no fio;
- tempo real gasto no servidor;
- conexões de outros processos;
- rota completa por roteadores;
- detalhes físicos de Wi-Fi/Ethernet.

HTTP/2 exige separar eventos de conexão dos eventos de stream. HTTP/3/QUIC deve ser experimental porque troca TCP por UDP e precisa de adapter próprio.

## 5. Captura e privilégios

1. **Trace próprio:** não exige privilégio elevado; entra no MVP.
2. **Proxy/MITM:** exige configuração de proxy e certificado raiz; opt-in e separado.
3. **Packet capture/ETW:** pode exigir permissões, driver ou integração específica do sistema; experimental.

## 6. Complexidade

- Editor HTTP: média.
- Collections, Git e import cURL: média.
- Environments e secrets: média/alta.
- Trace DNS/TCP/TLS: alta.
- Replay/diff semântico: média/alta.
- WebSocket/SSE: média.
- gRPC dinâmico e streaming: alta.
- Profiler seguro: alta.
- HTTP/3, proxy/MITM e packet capture: muito alta.

## 7. Referências Rust

- HTTP: `reqwest`, `hyper`, `hyper-util`, `h2`.
- Runtime: `tokio`.
- TLS: `rustls`, `tokio-rustls`.
- DNS: `hickory-resolver`.
- WebSocket: `tokio-tungstenite`.
- gRPC: `tonic`, `prost`, `prost-types`, `prost-reflect`.
- QUIC/HTTP/3: `quinn` + `h3` ou `quiche`, experimental.
- MQTT: `rumqttc`, posterior.
- Persistência: `rusqlite` ou `sqlx`.
- Secrets: `keyring` e APIs nativas.

## 8. Hipóteses que precisam de spikes

- instrumentar HTTP sem perder pooling e reuso;
- importar `.proto`, listar `PaymentService` e executar unary dinamicamente;
- consultar gRPC Reflection quando disponível;
- validar Credential Manager no Windows;
- testar cancelamento de streams;
- definir redaction irreversível para export;
- medir custo de corpos grandes e histórico;
- validar proxy corporativo, mTLS e IPv6.

