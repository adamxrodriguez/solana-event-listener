# Solana Event Listener

A production-ready Rust application for listening to Solana blockchain events via WebSocket with real-time metrics, JSONL persistence, and observability.

## Why Rust?

This project leverages Rust for:

- **Low latency**: Zero-cost abstractions and efficient async runtime
- **Memory safety**: Catch bugs at compile time without garbage collection overhead
- **Concurrent performance**: Tokio async runtime handles thousands of connections efficiently
- **Reliability**: Strong typing prevents runtime errors in event processing pipelines

## Features

- 🔌 Real-time WebSocket connections to Solana RPC
- 📊 Prometheus metrics on `/metrics` endpoint
- 💾 Append-only JSONL event storage
- 🔄 Automatic reconnection with exponential backoff
- 🎯 Dual modes: log subscriptions and account monitoring
- 🛡️ Graceful shutdown and error recovery
- 📦 Docker Compose setup with Prometheus + Grafana

## Quick Start

### Prerequisites

- Rust 1.83+ (install via [rustup](https://rustup.rs/))
- Solana mainnet-beta RPC access

### Installation

```bash
git clone <repository-url>
cd solana-event-listener
cp .env.example .env  # Edit .env as needed
cargo build --release
```

### Configuration

Create `.env` file (see `.env.example` for template):

```bash
WS_URL=wss://api.mainnet-beta.solana.com/
MODE=logs
PROGRAM_ID=<your-program-id>
COMMITMENT=finalized
EVENT_LOG_PATH=./events.jsonl
METRICS_ADDR=0.0.0.0:9108
RUST_LOG=info
```

### Run

**Logs mode:**
```bash
cargo run --release -- --mode logs --program-id <PROGRAM_ID>
```

**Account mode:**
```bash
cargo run --release -- --mode account --accounts <PUBKEY1,PUBKEY2>
```

**With custom metrics port:**
```bash
cargo run --release -- --metrics-addr 0.0.0.0:9999
```

### Docker Compose (Recommended for Production)

Start with Prometheus and Grafana:

```bash
# Build and start all services
docker-compose up -d

# View logs
docker-compose logs -f solana-event-listener

# Stop services
docker-compose down
```

Access dashboards:
- **Prometheus**: http://localhost:9090
- **Grafana**: http://localhost:3000 (default: `admin`/`admin`)
- **Metrics**: http://localhost:9108/metrics

### Check Metrics

```bash
curl localhost:9108/metrics
```

Sample output:
```
# HELP sol_events_total Total number of events processed
# TYPE sol_events_total counter
sol_events_total 42

# HELP sol_ws_connected WebSocket connection status (1=connected, 0=disconnected)
# TYPE sol_ws_connected gauge
sol_ws_connected 1

# HELP sol_errors_total Total number of errors encountered
# TYPE sol_errors_total counter
sol_errors_total 0
```

## Configuration Reference

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `WS_URL` | Solana WebSocket endpoint | `wss://api.mainnet-beta.solana.com/` | Yes |
| `MODE` | Operation mode: `logs` or `account` | `logs` | Yes |
| `PROGRAM_ID` | Program ID for logs mode | - | If MODE=logs |
| `ACCOUNTS` | Comma-separated addresses for account mode | - | If MODE=account |
| `COMMITMENT` | Commitment level: `processed`, `confirmed`, `finalized` | `finalized` | No |
| `EVENT_LOG_PATH` | Path to JSONL event log file | `./events.jsonl` | No |
| `METRICS_ADDR` | Metrics server bind address | `0.0.0.0:9108` | No |
| `RUST_LOG` | Logging level | `info` | No |

CLI flags override environment variables.

## JSONL Event Format

### Log Event

```json
{"timestamp":"2024-01-15T10:30:45Z","signature":"5VeK...","slot":12345,"program_id":"ComputeBudget111111111111111111111111111111","logs":["Program log: ..."]}
```

### Account Event

```json
{"timestamp":"2024-01-15T10:30:45Z","pubkey":"Address...","slot":12345,"lamports":1000000,"data":"base64..."}
```

## Terminal Output Example

Running in logs mode produces output like:

```
$ cargo run --release -- --mode logs --program-id ComputeBudget111111111111111111111111111111

2024-01-15T10:30:45.123Z INFO Starting Solana Event Listener v0.1.0
2024-01-15T10:30:45.125Z INFO Configuration loaded: mode=logs
2024-01-15T10:30:45.126Z INFO Metrics registry initialized
2024-01-15T10:30:45.127Z INFO Metrics server spawned on 0.0.0.0:9108
2024-01-15T10:30:45.128Z INFO Storage initialized: ./events.jsonl
2024-01-15T10:30:45.129Z INFO Starting logs subscription mode
2024-01-15T10:30:45.130Z INFO Connecting to Solana WebSocket: wss://api.mainnet-beta.solana.com/
2024-01-15T10:30:45.456Z INFO Connected to WebSocket
2024-01-15T10:30:45.457Z INFO Sending subscription request for program: ComputeBudget111111111111111111111111111111
2024-01-15T10:30:45.500Z INFO Subscribed to logs for program: ComputeBudget111111111111111111111111111111
2024-01-15T10:30:46.200Z INFO Event: signature=5VeK..., slot=245000000, program=ComputeBudget111111111111111111111111111111, log_lines=3
2024-01-15T10:30:47.100Z INFO Event: signature=7XmP..., slot=245000001, program=ComputeBudget111111111111111111111111111111, log_lines=2
```

## Architecture

```
┌─────────────────────────────────────────┐
│   Solana Blockchain (Mainnet-Beta)     │
└────────────────────┬────────────────────┘
                     │ WebSocket JSON-RPC
                     │ (logsSubscribe /
                     │  accountSubscribe)
                     ▼
┌─────────────────────────────────────────┐
│     Solana Event Listener (Rust)       │
│  ┌────────────────────────────────┐    │
│  │  WebSocket Client (tokio)      │    │
│  └────────────┬───────────────────┘    │
│               │                         │
│  ┌────────────▼───────────────────┐    │
│  │   Event Processing Pipeline    │    │
│  └────────────┬───────────────────┘    │
│               │                         │
│  ┌────────────┴───────────────────┐    │
│  │                                │    │
│  ▼                                ▼    │
│  ┌────────────┐         ┌────────────┐ │
│  │ JSONL File │         │ Prometheus │ │
│  │  Storage   │         │  Metrics   │ │
│  └────────────┘         └────────────┘ │
└─────────────────────────────────────────┘
```

## Development

### Run Tests

```bash
cargo test
```

### Run Clippy

```bash
cargo clippy -- -D warnings
```

### Run Lints

```bash
cargo fmt
cargo clippy -- -D warnings
```

## License

MIT

## Contributing

Contributions welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Make your changes with tests
4. Run `cargo clippy -- -D warnings` and `cargo test`
5. Submit a pull request

## Troubleshooting

**Issue**: "MODE=logs requires PROGRAM_ID to be set"
- Set `PROGRAM_ID` in `.env` or pass `--program-id` flag

**Issue**: WebSocket connection fails
- Verify `WS_URL` is correct and accessible
- Check network connectivity to Solana RPC
- Ensure commitment level is supported

**Issue**: Metrics not incrementing
- Verify `/metrics` endpoint is accessible
- Check `RUST_LOG=debug` for detailed logs
- Ensure events are being received from Solana

