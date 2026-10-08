# Modelo de segurança local

O Larry foi projetado para executar localmente, mas uma aplicação que envia requests e armazena credentials ainda precisa tratar dados como sensíveis.

## Dados

- collections são arquivos do usuário e podem conter URLs, bodies e headers;
- environments versionáveis devem conter apenas valores públicos e referências;
- secrets devem ser armazenados no Credential Manager, Keychain ou mecanismo equivalente;
- histórico deve ser sanitizado antes de ser persistido;
- logs e mensagens de erro não devem revelar secrets.

## Fronteira frontend/core

Vue não acessa rede, filesystem ou secrets diretamente. Essas operações são executadas por comandos Tauri validados no Rust.

O frontend não deve receber o valor persistente de um secret apenas para exibi-lo. Ele pode editar um novo valor e solicitar sua gravação, mas a leitura deve permanecer no core.

## Recursos de maior risco

Proxy local, MITM, packet capture e privilégios elevados não fazem parte do trace básico. Quando forem adicionados, deverão ser opt-in, explicar o destino dos dados e documentar o impacto no sistema operacional.

Para reportar uma falha, consulte [SECURITY.md](../SECURITY.md).
