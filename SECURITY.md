# Segurança

O Larry executa requests contra servidores escolhidos pelo usuário e pode acessar secrets armazenados no mecanismo seguro do sistema operacional. Trate qualquer diagnóstico, fixture ou arquivo de collection como potencialmente sensível.

## Reporte responsável

Não abra uma issue pública para uma vulnerabilidade. Envie o relatório por um canal privado ao mantenedor do repositório. Quando o GitHub Private Vulnerability Reporting estiver habilitado, ele será o canal preferencial.

Inclua, quando possível:

- versão do Larry;
- sistema operacional e arquitetura;
- passos mínimos para reproduzir;
- impacto observado;
- logs sanitizados;
- fixture que não contenha secrets reais.

Não inclua tokens, cookies, senhas, chaves privadas, certificados privados, URLs internas ou bodies de produção.

## Escopo de segurança

Relate especialmente:

- exposição de secrets na UI, histórico, logs, exportações ou mensagens de erro;
- execução de comandos arbitrários pelo frontend;
- bypass de validação de paths, URLs ou permissões;
- capabilities Tauri excessivas;
- acesso de terceiros a dados locais;
- falhas em sanitização de collections Postman ou environments;
- comportamento inesperado em proxy, MITM ou packet capture.

O Larry não deve exigir privilégios elevados para o trace próprio. Recursos que interceptem tráfego de terceiros devem ser explícitos, opt-in e documentados.
