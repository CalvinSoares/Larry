# Formato das collections

Collections são arquivos YAML legíveis e versionáveis pelo Git. O arquivo deve declarar `schemaVersion` para que mudanças futuras possam ser migradas explicitamente.

Exemplo mínimo:

```yaml
schemaVersion: 1
name: Minha API
requests:
  - id: health
    name: Health check
    method: GET
    url: http://localhost:3000/health
    query: []
    headers: []
    body: null
```

## Regras

- não colocar tokens ou cookies reais no arquivo;
- usar placeholders para valores de environment;
- manter IDs estáveis para facilitar diffs;
- registrar alterações incompatíveis em uma migration;
- preservar campos desconhecidos durante import/export quando isso for suportado;
- atualizar testes de serialização quando o schema mudar.

O formato ainda é inicial. Alterações que afetem collections existentes devem ser tratadas como mudança de compatibilidade, mesmo quando o código parecer pequeno.
