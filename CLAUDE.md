# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What is scrapeycat?

A scriptable web scraper daemon written in Rust. Scripts are written in a Lua-based DSL (`.scrape` files) that exposes scraping commands as global functions. It can run scripts one-off (`scrapeycat run`) or as a cron-scheduled daemon (`scrapeycat daemon`).

## Build & Development Commands

```bash
make check       # fmt --check + clippy (the lint/CI check)
make test        # cargo test --features testutils (run all tests)
make nextest     # cargo nextest run --features testutils
make booktest    # run book documentation tests only
make coverage    # llvm-cov branch coverage (requires nightly)
make clean       # cargo clean -p scrapeycat
```

Run a single test:
```bash
cargo test --features testutils <test_name>
```

The `testutils` feature flag is required for integration tests — it enables `src/testutils.rs` which provides `TestHttpDriver` (serves canned responses from `tests/assets/`) and the `path_in_project_root!` macro.

## Architecture

### Core pipeline: Scraper → ScrapeLang → Effects

1. **`scraper.rs`** — Immutable `Scraper<H: HttpDriver>` that carries a `Vector<String>` of results. Each operation (get, extract, delete, retain, discard, take, drop, prepend, append, join, jsonpath, etc.) returns a new `Scraper`. The `HttpDriver` trait is generic so tests use `NullHttpDriver`/`TestHttpDriver` instead of real HTTP.

2. **`scrapelang/program.rs`** — The script runtime. Embeds a Lua interpreter (mlua/Lua 5.2) and registers each scraper operation as a global Lua function (`get()`, `extract()`, `run()`, `effect()`, etc.). Scripts are plain Lua that calls these globals. Variable substitution (`{varname}`) is handled in Rust before passing strings to the scraper. The `run()` function orchestrates script loading, arg/kwarg injection, and execution.

3. **`effect.rs`** — Side effects (`print`, `notify`) are sent via `tokio::mpsc` channels as `EffectInvocation` messages to a separate handler task, keeping the scraper pipeline pure. Effects receive positional args, keyword args, and option flags (e.g., `SilentTest`).

### Daemon system (`daemon/`)

- **`config.rs` / `config_file.rs`** — TOML config parsing (versioned, currently v1). Defines script directories, script name patterns, and suites of jobs.
- **`suite.rs`** — `Suite` contains `Job`s. Each job has a script name, args, kwargs, a `CronSpec`, and a dedup flag.
- **`cron.rs`** — Cron expression parser (`CronSpec`) using winnow. Supports standard 5-field cron syntax.
- **`mod.rs`** — `run_forever()` is the main daemon loop, driven by a `Clock` trait (real or mock). Checks due jobs each minute, spawns script runs as tokio tasks. Each job gets its own effects channel with optional deduplication.

### Testing patterns

- Integration tests in `tests/scripts.rs` use a `test!("name")` macro that runs `tests/assets/scripts/{name}.scrape` against `TestHttpDriver` and compares output to `tests/assets/scripts/{name}.expect`.
- `tests/book.rs` tests code examples from the mdbook documentation.
- `tests/stress.rs` contains bolero-based fuzz/property tests.
- Daemon tests use mock `Clock` implementations (`PerfectMockClock`, `HalfIntervalPeekMockClock`) to test scheduling without real time.

### Key types

- `ScriptLoaderPointer` = `Arc<RwLock<dyn Fn(&str) -> Result<String, Error>>>` — injectable script loading.
- `EffectSignature` = `fn(args, kwargs, options) -> Option<Error>` — effect function type.
- `HttpDriver` trait — async `get()` method, generic across the codebase for testability.

## Library vs Binary

The library crate is `libscrapeycat` (`src/lib.rs`). The binary is `scrapeycat` (`src/main.rs`). The top-level `Error` enum in `lib.rs` is used throughout.
