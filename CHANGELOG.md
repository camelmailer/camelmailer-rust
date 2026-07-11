# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/camelmailer/camelmailer-rust/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/camelmailer/camelmailer-rust/releases/tag/v0.1.0
