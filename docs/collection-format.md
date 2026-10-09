# Formato das collections

Collections são arquivos YAML legíveis e versionáveis pelo Git. O arquivo deve declarar `schemaVersion` para que mudanças futuras possam ser migradas explicitamente.

Exemplo mínimo:

```yaml
schemaVersion: 2
name: Minha API
requests:
  - id: health
    name: Health check
    method: GET
    url: http://localhost:3000/health
    query: []
    headers: []
    cookies: []
    body: null
    auth: null
    assertions: []
folders: []
```

## Regras

- não colocar tokens ou cookies reais no arquivo;
- usar placeholders para valores de environment;
- usar `auth` com referências como `{{secret.accessToken}}` para credenciais;
- usar referências como `{{secret.sessionCookie}}` para valores de cookies sensíveis;
- declarar assertions sem scripts para status, headers e conteúdo textual do body;
- caminhos de arquivos multipart são locais à máquina e podem exigir revisão depois de um clone;
- manter IDs estáveis para facilitar diffs;
- `folders` contém pastas recursivas com `id`, `name`, `requests` e `folders`;
- requests na raiz continuam válidas para preservar collections simples;
- `schemaVersion: 1` é migrado no carregamento para a versão 2, mantendo requests antigas na raiz;
- registrar alterações incompatíveis em uma migration; campos opcionais compatíveis, como `assertions`, podem permanecer no schema atual e devem ter default explícito;
- preservar campos desconhecidos durante import/export quando isso for suportado;
- atualizar testes de serialização quando o schema mudar.

O formato ainda é inicial. Alterações que afetem collections existentes devem ser tratadas como mudança de compatibilidade, mesmo quando o código parecer pequeno.

## gRPC dinâmico

O primeiro lote de gRPC é uma ferramenta local para importar um `.proto` e executar métodos unary. Ele ainda não grava requests gRPC no schema de collection. A persistência será adicionada somente depois que metadata, streaming, secrets e compatibilidade de paths dos protos tiverem um contrato versionado.
