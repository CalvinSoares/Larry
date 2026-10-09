# Roadmap público

## MVP

- HTTP/HTTPS local;
- autenticação comum e cookies por request;
- forms `multipart/form-data` e `application/x-www-form-urlencoded`;
- collections YAML versionáveis;
- environments e secrets separados;
- histórico e comparação local;
- importação Postman com avisos;
- importação cURL com prévia e conversão revisável;
- WebSocket básico para sessões locais e mensagens de texto;
- SSE básico para eventos enviados pelo servidor;
- assertions HTTP locais para status, headers e conteúdo do body;
- importação dinâmica de `.proto` e execução local de métodos gRPC unary;
- trace HTTP inicial e diagnóstico por camada;
- profiler HTTP local com limites, concorrência, cancelamento e percentis;
- Protocol Lab inicial com mapa de camadas e proveniência do trace HTTP;
- comparação controlada da mesma request em HTTP/1.1 e HTTP/2;
- base de contribuição, documentação e segurança.

## V2

- cancelamento de requests;
- GraphQL;
- evolução do SSE com reconexão e Last-Event-ID;
- evolução do WebSocket com headers, editor binário e histórico de mensagens;
- trace HTTP instrumentado com TCP/TLS quando tecnicamente disponível;
- Git diff semântico;
- profiler avançado com warm-up, rate limiting, reuso de conexão e relatórios exportáveis;
- comparação prática entre REST, GraphQL e gRPC;
- laboratório de conexão e streams HTTP/2, incluindo multiplexação e reuso;
- CI de instaladores para os sistemas suportados;
- gRPC Reflection, streaming e persistência de requests gRPC.

## Experimental

- HTTP/2 detalhado;
- HTTP/3/QUIC;
- Socket.IO e MQTT;
- proxy, MITM e packet capture opt-in;
- backend opcional para sincronização e colaboração.

O roadmap é uma direção pública, não uma promessa de datas. Issues e decisões podem alterar a ordem conforme evidências técnicas e feedback da comunidade.
