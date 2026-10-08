# Desenvolvimento local

## Requisitos

- Node.js;
- npm ou pnpm;
- Rust e Cargo;
- toolchain e dependências do Tauri 2 para o sistema operacional.

## Instalação

```bash
npm install
```

## Comandos

```bash
# frontend Vite
npm run dev

# aplicação desktop
npm run tauri dev

# build e type-check
npm run build

# Rust
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo test --manifest-path src-tauri/Cargo.toml
```

## Fixtures

Use servidores locais e dados sintéticos. Uma fixture não deve depender de internet, conter credentials ou exigir uma conta externa.

## Antes de abrir um Pull Request

1. execute o build frontend;
2. execute format check e testes Rust;
3. revise o diff em busca de secrets;
4. atualize a documentação afetada;
5. descreva limitações e evidências no Pull Request.
