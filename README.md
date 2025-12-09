<div align="center">

# ASI Chain Wallet CLI

[![Status](https://img.shields.io/badge/Status-BETA-FFA500?style=for-the-badge)](https://github.com/asi-alliance/asi-chain-wallet-cli)
[![Version](https://img.shields.io/badge/Version-0.1.0-A8E6A3?style=for-the-badge)](https://github.com/asi-alliance/asi-chain-wallet-cli/releases)
[![License](https://img.shields.io/badge/License-Apache%202.0-1A1A1A?style=for-the-badge)](LICENSE)
[![Docs](https://img.shields.io/badge/Docs-Available-C4F0C1?style=for-the-badge)](https://docs.asichain.io)

<h3>Command-line wallet for ASI Chain blockchain network</h3>

Part of the [**Artificial Superintelligence Alliance**](https://superintelligence.io) ecosystem

*Uniting Fetch.ai, SingularityNET, and CUDOS*

</div>

---

## Table of Contents

1. [Overview](#overview)
2. [Prerequisites](#prerequisites)
3. [Installation](#installation)
4. [Configuration](#configuration)
5. [Commands](#commands)
6. [Token Decimals](#token-decimals)
7. [Project Structure](#project-structure)
8. [License](#license)

---

## Overview

ASI Chain Wallet CLI is a command-line tool for interacting with the ASI Chain blockchain. It provides wallet management, token transfers, balance checking, and transaction history monitoring capabilities.

**Key Features:**

- Create and manage ASI Chain wallets
- Check wallet balance
- Send ASI tokens to other addresses
- Monitor deploy history in real-time via WebSocket
- Export transaction history to JSON or CSV format

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (edition 2024)
- Access to ASI Chain network nodes (Validator and Observer)
- GraphQL API endpoint for transaction history

## Installation

Clone the repository with submodules:

```bash
git clone https://github.com/asi-alliance/asi-chain-wallet-cli.git
cd asi-chain-wallet-cli
git submodule init
git submodule update
```

Build the project:

```bash
cargo build --release
```

## Configuration

Create a `.env` file from the example:

```bash
cp .env.example .env
```

Edit `.env` with your network configuration:

```env
VALIDATOR_HOST=<VALIDATOR_IP>
VALIDATOR_GRPC_PORT=40432
VALIDATOR_HTTP_PORT=40433

OBSERVER_HOST=<OBSERVER_IP>
OBSERVER_GRPC_PORT=40452

OUTPUT_DIR=./credentials
TOKEN="ASI"

# Logging control: set to "true" to show all logs from rust-client, "false" to hide them
VERBOSE_LOGS=false

# For deploy history
GRAPHQL_URL=<GRAPHQL_API_ENDPOINT>
```

**Required Configuration:**

| Variable | Description |
|----------|-------------|
| `VALIDATOR_HOST` | IP address of the validator node for sending transactions |
| `VALIDATOR_GRPC_PORT` | gRPC port of the validator node (default: 40432) |
| `VALIDATOR_HTTP_PORT` | HTTP port of the validator node (default: 40433) |
| `OBSERVER_HOST` | IP address of the observer node for read operations |
| `OBSERVER_GRPC_PORT` | gRPC port of the observer node (default: 40452) |
| `OUTPUT_DIR` | Directory for storing wallet credentials (default: ./credentials) |
| `TOKEN` | Token name (default: ASI) |
| `VERBOSE_LOGS` | Show detailed logs from rust-client (default: false) |
| `GRAPHQL_URL` | GraphQL API endpoint for transaction history |

## Commands

### Create Wallet

Generate a new wallet and optionally save credentials:

```bash
# Create and display keys (not saved)
cargo run -- create

# Create and save to ./credentials/wallet_info.json
cargo run -- create --save
```

Output includes private key, public key, and ASI address.

### Check Balance

Check the balance of your wallet:

```bash
cargo run -- balance
# or
cargo run -- bal
```

### Send Tokens

Transfer tokens to another address:

```bash
cargo run -- send --to <recipient-address> --amount <amount>
```

**Example:**

```bash
cargo run -- send --to 1111AtahZeefej4tvVR6ti9TJtv8yxLebT31SCEVDCKMNikBk5r3g --amount 100000000
```

### Transaction History

Monitor deploy history in real-time:

```bash
# Display last 10 deployments and monitor for new ones
cargo run -- history

# Custom limit
cargo run -- history --limit 20

# or use alias
cargo run -- hist --limit 20
```

This command displays recent deployments and continuously monitors for new transactions via WebSocket subscription.

### Export Data

Export wallet transaction history:

```bash
# Export as JSON (default)
cargo run -- export

# Export as CSV
cargo run -- export --format csv

# Custom output path
cargo run -- export --output my_data.json

# Custom limit (default: 100)
cargo run -- export --limit 50

# Combine options
cargo run -- export --format csv --output reports/wallet_2024.csv --limit 200
```

Exported files are saved to the `exports/` directory by default with timestamp-based filenames.

### Help

Display available commands:

```bash
cargo run -- help
```

## Token Decimals

ASI token uses 8 decimal places. When specifying amounts:

| Human Amount | CLI Amount |
|--------------|------------|
| 0.5 ASI | 50000000 |
| 1 ASI | 100000000 |
| 10 ASI | 1000000000 |
| 100 ASI | 10000000000 |

**Formula:** `CLI Amount = Human Amount × 10^8`

## Project Structure

```
asi-chain-wallet-cli/
├── Cargo.toml              # Project dependencies
├── .env.example            # Environment configuration template
├── src/
│   ├── main.rs             # Entry point and CLI commands
│   ├── config.rs           # Configuration management
│   ├── models.rs           # Data models
│   ├── commands/
│   │   ├── balance.rs      # Balance command
│   │   ├── export.rs       # Export command
│   │   ├── history.rs      # History command
│   │   ├── keys.rs         # Create wallet command
│   │   └── send.rs         # Send command
│   ├── requests/
│   │   └── graphql.rs      # GraphQL queries
│   └── utils/
│       ├── deployment.rs   # Deployment utilities
│       └── websocket.rs    # WebSocket connection
└── rust-client/            # Submodule: low-level node CLI
```

