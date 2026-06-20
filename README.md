# hold-sccache

Installs sccache and wires it into the fleet's Cargo config as a size-capped compilation cache — and unwires it just as cleanly.

## Why it exists

Across 190-plus repos, every local `cargo build` recompiles the same heavyweight crates — serde, tokio, clap — from scratch. The compiler does identical work over and over because nothing remembers it.

A single shared `CARGO_TARGET_DIR` would share artifacts but reintroduces lock contention: parallel builds serialize on one target. sccache avoids that. It caches at the compilation-unit level with no shared lock, so the heavy crates compile once and parallel builds stay parallel. hold-sccache is the small tool that gets sccache installed, wired into the fleet config with a size cap, and back out again without hand-editing TOML.

## Install

```sh
cargo install --path .
```

## Usage

```sh
# Put an sccache binary on PATH (installs via `cargo install sccache`).
hold-sccache install

# Wire sccache into the fleet cargo config as the rustc wrapper, capped at 20G.
hold-sccache wire --max-size 20G

# Show whether sccache is installed and wired.
hold-sccache status

# Show sccache's own hit/miss statistics as JSON.
hold-sccache stats

# Remove the sccache wiring from the cargo config.
hold-sccache unwire
```

Defaults: the fleet cargo config is `~/wintermute/.cargo/config.toml`, the sccache config directory is `~/.config/sccache`. All are overridable with flags.

## How it works

`wire` edits `~/wintermute/.cargo/config.toml` with `toml_edit`, which preserves every existing key, comment, and formatting — it sets `build.rustc-wrapper = "sccache"` and writes the size cap to `~/.config/sccache/config`. `unwire` removes only the wrapper key it added. `status` reports installation and wiring state without needing a live sccache daemon; `stats` parses `sccache --show-stats` text into structured JSON.

What it deliberately does not touch: it doesn't manage the sccache daemon lifecycle, doesn't run sccache-dist, doesn't edit any per-repo `Cargo.toml` or `.cargo/config.toml`, and doesn't purge cache contents — it manages configuration, not the cache itself.

## Where it fits

hold-sccache is part of the `hold-*` family that manages the wintermute fleet's Cargo build commons:

- **hold-survey** — measures dependency duplication across the fleet
- **hold-migrate** — drains private `target/` dirs into the shared hold
- **hold-anchor** — deduplicates artifacts across machines
- **hold-guard** — bounds the shared hold's budget with LRU eviction

Where the rest of the family shares and bounds a single `target/` hold, hold-sccache attacks the same waste from the other side — caching compilation units so the work isn't repeated in the first place.
