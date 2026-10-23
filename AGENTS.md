# AGENTS.md

Instructions for AI coding agents working in this repository.

## Commands

- `cargo build --lib`: quickest check that the crate compiles. CI (ci.yml)
  also builds on windows/macos and against stable, nightly and 1.80.0 (the
  MSRV, see Cargo.toml), keep changes MSRV compatible
- `cargo fmt --all -- --check`: formatting check, CI fails on it (linux only,
  but keep everything formatted). `rustfmt.toml` excludes steamworks-sys,
  its code is generated
- `cargo clippy`: NOT part of CI, and there are pre-existing warnings. Avoid
  introducing new warnings in code you touch, but do not go fixing unrelated
  warnings
- `cargo test`: most tests call `Client::init()` and need a running Steam
  client (the `steam_appid.txt` in the repo root uses app 480, Spacewar).
  They use `#[serial]` so they cannot run in parallel. CI never runs the
  tests. Run single ones with `cargo test <name> -- --nocapture`
- The `examples/` directories are separate workspace member crates, not
  example targets, so `cargo build --examples` in the root does nothing.
  Build them with `cargo build -p chat-example` etc. They are manual
  playgrounds to run while Steam is open
- Linux builds need these system packages (see ci.yml): libudev-dev,
  libx11-dev, libxi-dev, libgl1-mesa-dev, libasound2-dev,
  libxcb-shape0-dev, libxcb-xfixes0-dev

## Generated code

- `steamworks-sys/src/{linux,macos,windows}_bindings.rs` are bindgen output,
  never edit them by hand
- Local regeneration: `cargo build -p steamworks-sys --features
  rebuild-bindings`. Needs clang, and only generates the bindings for the
  host OS. Headers and libraries come from `steamworks-sys/lib/steam`
  unless `STEAM_SDK_LOCATION` points elsewhere
- The rebuild-bindings github workflow regenerates all three targets and
  auto-commits them
- Bumping the Steamworks SDK itself is documented in
  `updating-steamworks-sdk-version.md`

## Repository context

- This is a fork of https://github.com/Noxime/steamworks-rs. `origin` points
  to https://github.com/jespertheend/steamworks-rs, `upstream` to Noxime's
- The fork is not published to crates.io. It is consumed by `steamworks.js`
  (separate repo, usually ../steamworks.js), which pins this fork through a
  git `rev` in its Cargo.toml
- Workflow for changes here: push the commit to `origin` first, then bump
  the `rev` in steamworks.js to the pushed SHA (full 40 characters) and
  commit Cargo.toml together with the Cargo.lock update over there

## Architecture

- One module per steamworks interface. `src/lib.rs` owns the Client type
  plus callback registration and dispatch, `src/server.rs` the game server
  variant
- `steamworks-sys/` is a separate `-sys` crate that links against the SDK
  redistributable binaries vendored in `steamworks-sys/lib/steam`, its
  build.rs copies the right one for the target triple into OUT_DIR
- Features: `serde` makes some types serializable, `image` exposes
  achievement icons through the image crate, `raw-bindings` re-exports
  steamworks-sys as `steamworks::sys`. steamworks.js uses serde +
  raw-bindings

## Style

- rustfmt only, no per file exceptions beyond the steamworks-sys ignore
- Keep code comments short, only for non-obvious intent
