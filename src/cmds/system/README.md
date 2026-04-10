# System and Generic Utilities

> Part of [`src/cmds/`](../README.md) — see also [docs/contributing/TECHNICAL.md](../../../docs/contributing/TECHNICAL.md)

## Specifics

- `read.rs` uses `core/filter` for language-aware code stripping (FilterLevel: none/minimal/aggressive)
  - **Note**: Can be exposed as `rtk cat` by adding Cat command variant to main.rs Commands enum
- `grep_cmd.rs` reads `core/config` for `limits.grep_max_results` and `limits.grep_max_per_file`
- `local_llm.rs` (`rtk smart`) uses `core/filter` for heuristic file summarization
- `format_cmd.rs` is a cross-ecosystem dispatcher: auto-detects and routes to `prettier_cmd` or `ruff_cmd` (black is handled inline, not as a separate module)
- `ls.rs` has platform-specific implementations:
  - **Windows**: Native Rust using `std::fs::read_dir()` (zero external dependencies, <5ms)
  - **Unix**: Proxies to external `ls` command (maintains compatibility with existing flags)

## Cross-command

- `format_cmd` routes to `cmds/js/prettier_cmd` and `cmds/python/ruff_cmd`

## Windows-Specific Behavior

- `ls.rs::run_windows_native()` implements native directory listing without external dependencies
- Supports flags: `-a`/`--all` (show hidden), `-l`/`--long` (long format), `-h`/`--human-readable` (human-readable sizes)
- Filters hidden files (starts with `.`) and noise directories (`.git`, `node_modules`, etc.) by default
- Uses `std::fs::Metadata` for file properties (size, modified time, file type)
- Error handling: gracefully handles permission denied, directory not found, non-UTF-8 filenames
