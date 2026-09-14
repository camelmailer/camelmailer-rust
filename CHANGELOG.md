# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-09-14

### Added

- `campaigns()`: `create_draft`, `create_and_send`, `list`,
  `list_for_stream`, `get`, `get_for_stream`, `update`, `send`, `cancel`.
  The two create methods hit different routes: `create_draft` writes the
  campaign and waits, while `create_and_send` expands it to the stream's
  subscribers before the call returns.
- `subscribers()`: `list`, `add`, `import`, `complaint`, `remove`.
- `layouts()`: `list`, `create`, `get`, `update`, `delete`, `upload_logo`.
- `inbound()`: `list`, `get`, `retry`, `bypass`.
- `logs()`: `list`, `tags`.
- `emails().send_to_stream()` for broadcasting to a stream's subscribers.
- `emails().idempotent(key)`, which attaches the `Idempotency-Key` header
  to every send that service performs. The key is a header rather than a
  body field, because the body is what the server hashes to recognise a
  replay.
- The blocking client mirrors all of the above.

## [0.1.0] - 2026-07-11

### Added

- Async `CamelMailer` client (reqwest + rustls) with configurable base URL
  for self-hosted instances, timeout, and custom `reqwest::Client`.
- `emails()` service: `send`, `send_batch`, `send_with_template`,
  `send_with_template_batch`, `get`, `list` (scope/status/tag/query/stream
  filters), `deliveries`, `opens`, `clicks`, `raw` (with base64 decoding).
- `templates()` service: `list`, `create`, `get`, `update`, `archive`,
  `render`.
- `streams()` service: `list`, `create`, `get`, `update`, `archive`.
- `stats()` service: `get` (with time window), `deliveries` (queue depth).
- `bounces()` service: `list`, `get`.
- `dmarc()` service: `summary`, `reports`, `report`.
- `server()` service: `get`, `ping`.
- Typed `Error` enum with stable API error codes
  (`Error::Api { code, message, status }`) and helpers
  (`is_unauthorized`, `is_not_found`, `is_validation_error`).
- Builders for `SendEmailRequest` and `SendTemplateRequest`; `Address`
  (bare or named) and `Attachment` helpers.
- Optional `blocking` feature mirroring the full API surface
  synchronously.

[Unreleased]: https://github.com/camelmailer/camelmailer-rust/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/camelmailer/camelmailer-rust/releases/tag/v0.2.0
[0.1.0]: https://github.com/camelmailer/camelmailer-rust/releases/tag/v0.1.0
