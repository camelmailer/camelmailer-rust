# Contributing

Thanks for helping improve the Camelmailer Rust SDK!

## Development setup

```sh
git clone https://github.com/camelmailer/camelmailer-rust
cd camelmailer-rust
cargo test --all-features
```

Rust 1.75+ (the MSRV) or newer stable. No services needed — unit and mock
tests run fully offline against [wiremock](https://crates.io/crates/wiremock).

## Commands

| What | Command |
|---|---|
| Tests (incl. blocking feature) | `cargo test --all-features` |
| Lint (CI-enforced) | `cargo clippy --all-targets --all-features -- -D warnings` |
| Format (CI-enforced) | `cargo fmt --all` |
| Live integration test | `CAMELMAILER_API_KEY=cm_… cargo test --test integration -- --nocapture` |

## Conventions

- Test-first: every new endpoint or behavior lands with a wiremock test
  (happy path + at least one error path).
- No network in unit tests; the live integration test skips itself without
  `CAMELMAILER_API_KEY` and never runs in CI.
- Public items carry rustdoc; `#![warn(missing_docs)]` is on.
- Response types stay permissive (`#[serde(default)]`, `Option` for
  nullable fields) so new server fields never break deserialization.
- Keep the blocking client in lockstep with the async surface.
- Update `CHANGELOG.md` (Keep a Changelog) with user-visible changes.
