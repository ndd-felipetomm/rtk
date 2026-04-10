# RTK — Instruções de Uso para Agentes de IA

**Sempre prefixe comandos shell com `rtk`** para economizar 60-90% dos tokens no contexto do LLM. Se o RTK não reconhece o comando, faz passthrough automático — nunca impede a execução.

```bash
# ❌ Desperdiça tokens         # ✅ Compactado
git log -20                     rtk git log -20
cargo test                      rtk cargo test
dotnet build                    rtk dotnet build
```

---

## Flags Globais

| Flag | Curta | Uso |
|------|-------|-----|
| `--verbose` | `-v` | Debug (-v, -vv, -vvv) |
| `--ultra-compact` | `-u` | Ícones ASCII, formato inline |
| `--skip-env` | — | `SKIP_ENV_VALIDATION=1` para processos filhos |

---

## Comandos Suportados

### Git & SCM

```bash
rtk git status                    # Status compacto
rtk git log -20                   # Commits condensados
rtk git diff --cached             # Diff do staging
rtk git show HEAD                 # Commit summary + diff compactado
rtk git add -A                    # → "ok"
rtk git commit -m "msg"          # → "ok <hash>"
rtk git push                     # → "ok <branch>"
rtk git pull                     # → "ok <stats>"
rtk git branch                   # Listing compacto
rtk git fetch                    # → "ok fetched (N refs)"
rtk git stash list               # Stash management
rtk git worktree                 # Worktrees listing
rtk gh pr list                   # GitHub CLI — PRs compacto
rtk gh issue list                # GitHub CLI — Issues
rtk gh run list                  # GitHub CLI — Workflow runs
rtk gt log                       # Graphite — stack log
rtk gt submit                    # Graphite — submit
rtk diff file1 file2             # Diff ultra-condensado entre arquivos
```

Subcomandos git não listados (rebase, cherry-pick, etc.) fazem passthrough automático. Flags globais git (`-C`, `--no-pager`, `--git-dir`) são suportadas.

### Rust

```bash
rtk cargo build                   # Strip "Compiling", manter erros
rtk cargo test                    # Apenas falhas
rtk cargo clippy --all-targets    # Warnings agrupados por lint rule
rtk cargo check                   # Strip "Checking", manter erros
rtk cargo install --path .        # Strip deps, manter resultado
rtk cargo nextest run             # Nextest failures-only
```

### JavaScript / TypeScript

```bash
rtk pnpm list                     # Pacotes ultra-denso
rtk pnpm outdated                 # "pkg: old → new"
rtk pnpm install lodash           # Sem progress bars
rtk pnpm typecheck                # Delega para filtro tsc
rtk npm install                   # Strip boilerplate npm
rtk npm test                      # Output limpo
rtk npx tsc --noEmit              # → filtro tsc (erros agrupados)
rtk npx eslint src/               # → filtro lint
rtk npx prisma generate           # → filtro prisma (sem ASCII art)
rtk npx next build                # → filtro next
rtk npx prettier --check .        # → filtro prettier
rtk npx playwright test           # → filtro playwright
rtk vitest run                    # Testes ~90% redução
rtk tsc --noEmit                  # Erros agrupados por arquivo
rtk next build                    # Build compacto
rtk lint src/                     # ESLint violations agrupadas
rtk prettier --check .            # Format check compacto
rtk playwright test               # E2E compacto
rtk prisma generate               # Sem ASCII art
rtk prisma migrate dev --name x   # Migration compacta
rtk prisma migrate status         # Status compacto
rtk prisma db-push                # Push compacto
```

### Python

```bash
rtk ruff check .                  # Lint compacto
rtk ruff format --check .         # Format check
rtk pytest                        # Apenas falhas
rtk pytest -k "test_login"       # Pattern match
rtk mypy src/                     # Erros agrupados
rtk pip list                      # Pacotes (auto-detecta uv)
rtk pip list --outdated           # Desatualizados
```

### Go

```bash
rtk go test ./...                 # JSON streaming ~90% redução
rtk go build ./cmd/server         # Erros apenas
rtk go vet ./...                  # Vet compacto
rtk golangci-lint run             # Lint compacto
```

### .NET

```bash
rtk dotnet build                  # Build compacto
rtk dotnet test                   # Testes compactos
rtk dotnet test --filter "Cat=U"  # Filtrar testes
rtk dotnet restore                # Restore compacto
rtk dotnet format --verify-no-changes  # Format check
```

Subcomandos não listados (run, publish, etc.) fazem passthrough.

### Ruby

```bash
rtk rake test                     # Minitest compacto
rtk rspec spec/models/            # RSpec compacto
rtk rubocop -A                    # RuboCop compacto
```

### Cloud & Containers

```bash
rtk aws sts get-caller-identity   # AWS JSON compacto
rtk aws s3 ls                     # Listar buckets
rtk docker ps                     # Containers essenciais
rtk docker images                 # Imagens
rtk docker logs my-container      # Logs deduplicados
rtk docker compose ps             # Compose compacto
rtk kubectl pods -A               # Pods todos namespaces
rtk kubectl services -n prod      # Services
rtk kubectl logs my-pod           # Logs deduplicados
rtk curl https://api.example.com  # Auto-JSON + schema
rtk wget https://example.com/f    # Strip progress bars
rtk psql -d mydb -c "SELECT ..."  # Strip borders
```

### Sistema & Utilitários

```bash
rtk ls -la                        # Listagem otimizada (nativo no Windows)
rtk tree -L 2                     # Árvore compacta
rtk read file.rs                  # Conteúdo completo
rtk read -l aggressive file.rs    # Filtro agressivo
rtk read -n --max-lines 50 f.rs   # Primeiras 50 linhas numeradas
rtk read --tail-lines 20 log.txt  # Últimas 20 linhas
rtk cat file.rs                   # Alias Unix-style para read (nativo no Windows)
rtk cat file1.txt file2.txt       # Concatenar múltiplos arquivos
rtk cat - < input.txt             # Ler de stdin
rtk cat --level minimal file.rs   # Cat com filtros (mesmas flags que read)
rtk grep "TODO" src/ -t rust      # Busca agrupada por arquivo
rtk grep "err" . --max 50         # Limitar resultados
rtk find . -name "*.rs"           # Busca com árvore compacta
rtk json data.json --schema       # Apenas estrutura JSON
rtk json data.json --depth 3      # Profundidade limitada
rtk env --filter AWS              # Variáveis filtradas, sensíveis mascaradas
rtk deps                          # Sumarizar dependências do projeto
rtk log /var/log/app.log          # Logs deduplicados
rtk wc -l src/*.rs                # Contagem compacta
rtk summary make all              # Resumo heurístico de qualquer comando
rtk smart src/main.rs             # Resumo técnico de 2 linhas
rtk format --check .              # Auto-detecta formatter (prettier, black, ruff)
rtk err cargo build               # Executar qualquer cmd, mostrar só erros
rtk test cargo test               # Executar qualquer cmd, mostrar só falhas
```

### Filtros TOML (58 comandos adicionais — automáticos)

Comandos sem módulo Rust dedicado são filtrados automaticamente via TOML:

`ansible-playbook` · `basedpyright` · `biome` · `brew install` · `bundle install` · `composer install` · `df` · `du` · `fail2ban-client` · `gcc` · `gcloud` · `gradle` · `hadolint` · `helm` · `iptables` · `jira` · `jj` · `jq` · `just` · `make` · `markdownlint` · `mise` · `mix compile` · `mix format` · `mvn` · `nx` · `ollama` · `oxlint` · `ping` · `pio run` · `poetry install` · `pre-commit` · `ps` · `quarto render` · `rsync` · `shellcheck` · `shopify theme` · `skopeo` · `sops` · `spring-boot` · `ssh` · `stat` · `swift build` · `systemctl status` · `task` · `terraform plan` · `tofu fmt` · `tofu init` · `tofu plan` · `tofu validate` · `trunk build` · `turbo` · `ty` · `uv sync` · `xcodebuild` · `yadm` · `yamllint`

```bash
rtk make all                      # Filtrado via TOML
rtk terraform plan                # Filtrado via TOML
rtk gradle build                  # Filtrado via TOML
```

---

## Suporte Windows

RTK tem suporte completo para Windows com implementações nativas para comandos de sistema de arquivos:

### Comandos Nativos Windows (sem dependências externas, <5ms)

```bash
rtk ls                            # Implementação Rust std::fs (não requer ls.exe)
rtk ls -a                         # Mostrar arquivos ocultos
rtk ls -lh                        # Formato longo com tamanhos legíveis
rtk cat arquivo.txt               # Implementação nativa (não requer cat.exe)
rtk cat -n --max-lines 50 f.txt   # Com numeração e truncamento
```

**Flags suportadas no Windows:**
- `ls`: `-a`/`--all` (mostrar ocultos), `-l` (formato longo), `-h` (tamanhos legíveis)
- `cat`: todas as flags do `read` (filtros, max-lines, tail-lines, line-numbers)

**Comportamento específico Windows:**
- Arquivos ocultos: começam com `.` (consistente com Unix)
- Diretórios de ruído (`.git`, `node_modules`, etc.) filtrados por padrão
- Resolução de comandos usa `PATHEXT` (`.exe`, `.cmd`, `.bat`)
- Detecção automática de cmdlets PowerShell vs executáveis externos
- Acesso direto ao sistema de arquivos (sem overhead do PowerShell)

**Comandos cross-platform (funcionam em Windows, macOS, Linux):**
- Git: `rtk git status`, `rtk git log`, `rtk git diff`
- Build: `rtk cargo build`, `rtk dotnet build`
- Package managers: `rtk pnpm list`, `rtk pip list`
- AWS CLI, Docker, kubectl: todos suportados no Windows

---

## Modo Proxy

Quando precisar de output completo sem filtragem (com rastreamento):

```bash
rtk proxy git log --graph --all   # Output 100% original
rtk proxy cargo test              # Sem filtros, com tracking
```

---

## Meta-Comandos

| Comando | Quando usar |
|---------|-------------|
| `rtk gain` | Ver economias de tokens do projeto |
| `rtk gain --history` | Histórico de comandos com savings |
| `rtk gain --project` | Filtrar por projeto atual |
| `rtk discover` | Encontrar oportunidades perdidas de economia |
| `rtk proxy <cmd>` | Output completo sem filtros |
| `rtk verify` | Verificar integridade de hooks e filtros |

---

## Regras para o Agente

1. **Sempre prefixe com `rtk`** — todo comando shell que aceitar
2. **Use `rtk proxy`** quando precisar de output não filtrado
3. **Comandos desconhecidos** passam direto — nunca há risco de falha por usar `rtk`
4. **Prioridade**: filtro dedicado > filtro TOML > proxy > comando raw (evitar)
