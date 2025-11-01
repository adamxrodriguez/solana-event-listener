# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2024-01-15

### Added

- Initial release of Solana Event Listener
- WebSocket JSON-RPC client for Solana blockchain
- Support for `logsSubscribe` mode for monitoring program logs
- Support for `accountSubscribe` mode for monitoring account changes
- Prometheus metrics endpoint on `/metrics` with:
  - `sol_events_total`: Counter for total events processed
  - `sol_errors_total`: Counter for total errors encountered
  - `sol_ws_connected`: Gauge for WebSocket connection status
- JSONL file storage for event persistence
- Automatic reconnection with exponential backoff (capped at 30s)
- Graceful shutdown on CTRL+C
- Configuration via environment variables and CLI arguments
- Docker Compose setup with Prometheus and Grafana
- Comprehensive CI/CD pipeline with GitHub Actions
- Extensive test coverage for core functionality
- Documentation with quickstart guide and examples

### Technical Details

- Built with Rust 2021 edition
- Uses Tokio async runtime for high performance
- Structured logging with tracing
- Error handling with anyhow and thiserror
- Clippy clean with `-D warnings`

[0.1.0]: https://github.com/yourusername/solana-event-listener/releases/tag/v0.1.0

