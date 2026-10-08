# Contribuindo com o Larry

Obrigado por considerar uma contribuição. O Larry ainda está em desenvolvimento, então uma issue bem descrita ou uma fixture reproduzível pode ser tão útil quanto uma alteração de código.

## Antes de começar

1. Procure issues e Discussions existentes.
2. Para uma mudança grande, abra uma Discussion ou RFC antes de implementar.
3. Nunca inclua tokens, cookies, certificados privados, secrets, collections privadas ou logs sensíveis.
4. Prefira fixtures locais e determinísticas a APIs públicas externas.

## Configuração local

Requisitos:

- Node.js;
- npm ou pnpm;
- Rust e Cargo;
- dependências do Tauri 2 para o sistema operacional.

Instale e valide o projeto:

```bash
npm install
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo test --manifest-path src-tauri/Cargo.toml
```

Execute o frontend ou o desktop:

```bash
npm run dev
npm run tauri dev
```

## Organização do código

- `src/`: interface Vue, componentes e serviços IPC;
- `src-tauri/src/domain/`: modelos e regras de domínio;
- `src-tauri/src/execution/`: execução e resolução de requests;
- `src-tauri/src/persistence/`: arquivos locais e SQLite;
- `src-tauri/src/integrations/`: Git e importadores;
- `src-tauri/src/secrets/`: armazenamento seguro de credenciais;
- `docs/`: documentação pública e decisões de arquitetura.

O frontend não deve acessar rede, filesystem ou secrets diretamente. Essas operações pertencem ao core Rust através de comandos Tauri pequenos e tipados.

## Regras de implementação

- mantenha o fluxo local/offline funcionando;
- preserve o erro técnico original junto da mensagem amigável;
- declare se uma métrica é medida, observada, inferida ou indisponível;
- adicione cobertura para caminho feliz e erro relevante;
- versione migrations, schemas, collections e eventos;
- não introduza privilégios elevados para o trace próprio;
- proxy, MITM e packet capture devem ser opt-in;
- mudanças de UI devem respeitar `docs/design.md` e validar foco, overflow e densidade desktop;
- alterações visuais devem incluir screenshot ou descrição do viewport validado no Pull Request.

## Issues e Pull Requests

Use o template correspondente ao tipo de trabalho. Um Pull Request deve explicar o problema, a decisão tomada, como foi validado e quais limitações permanecem.

Pull Requests devem ser focados em uma capacidade relacionada. Não é necessário limitar o número de arquivos, mas mudanças pequenas e revisáveis facilitam a manutenção.

Para importadores, persistência ou protocolos, inclua uma fixture local sem secrets reais. Para alterações de formato, documente compatibilidade, migration ou impacto para collections existentes.

## Commits

Não exigimos um padrão rígido de mensagens neste momento. Prefira mensagens objetivas e imperativas, por exemplo:

```text
feat: add local environment persistence
fix: preserve DNS diagnostic details
docs: explain collection schema versioning
```

## Uso de ferramentas de IA

Contribuições assistidas por IA são aceitas. A pessoa que abre o Pull Request continua responsável por entender, testar, revisar segurança e explicar todo o código enviado. Nunca forneça secrets ou dados privados a ferramentas de IA.
