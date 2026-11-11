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
8. [Contributing](#contributing)
9. [Branch status and client dependency](#branch-status-and-client-dependency)

---

## Overview

ASI Chain Wallet CLI is a command-line tool for interacting with the ASI Chain blockchain. It provides wallet management, token transfers, balance checking, and deploy history monitoring capabilities.

**Key Features:**

- Create ASI Chain wallet
- Check wallet balance
- Send ASI tokens to other addresses
- Monitor deploy history in real-time via WebSocket
- Export deploy history to JSON or CSV format

## Branch status and client dependency

Verified on 2026-11-03. Branches with the same name in different repositories
have different roles. Check current PR/ref status before starting new work.

| Wallet CLI branch | Current role | Use for the current Rust-node integration |
|---|---|---|
| `dev` | Active integration branch; #6 merged as `3a8f31e030a5d3d78e7abdc3b3fce6cd326ec8f0` | Start current work here. It contains the tested ASI client 0.2.9 adaptation for node 0.4.46. |
| `main` | Repository default and release branch; still `cd5d37ce651119c39de58e910db538729948e270` while [#3](https://github.com/asi-alliance/asi-chain-wallet-cli/pull/3) is open | Await the reviewed dev-to-main promotion for this integration. Main is a release branch, not deprecated. A plain default-branch clone currently gets older code. |
| `fix/asi-client-029-node-046` | Source of merged [#6](https://github.com/asi-alliance/asi-chain-wallet-cli/pull/6), candidate `8f6c20e` | Historical review/image source; continue development from dev. |
| `feature/cost-accounted-f1r3node` | Separate cost-accounting runtime, [#4](https://github.com/asi-alliance/asi-chain-wallet-cli/pull/4) | Outside this regular-node integration. Review its node/client pair separately. |

The client submodule uses [asi-alliance/asi-chain-client, branch `rust-dev`](https://github.com/asi-alliance/asi-chain-client/tree/rust-dev),
pinned to `5d1c9b4d12e33ec8b64a74a28df196ef03478e73` (verified on 2026-11-11).
Its node protocol crates remain pinned to `0ccc315d84b32d668e8db49cd56ba3c576693e15`.
The library preserves ASI metadata, exact raw-dust amounts and the
`rho:vault:system` transfer ABI; its CLI remains disabled.
Send reads the validator's minimum Phlo price before signing and checks its
symbol/shard. This client accepts prices in the positive `i64` range and encodes
them as 32-byte uint256 protobuf values. Larger prices fail before signing.
HTTP integer responses support both numeric and string encodings.

The `.gitmodules` branch setting records the selected development branch;
the exact gitlink determines the client used by builds. Use
`git submodule sync -- rust-client` after the URL change, followed by
`git submodule update --init --recursive`, never `--remote` for reproducible builds.
A compatible published node image is still required for live acceptance.

See the [infra wallet handoff](https://github.com/asi-alliance/asi-chain-testnet-infra/blob/main/docs/handoffs/2026-11-03/wallet-cli-pr6.md),
[client branch audit](https://github.com/asi-alliance/asi-chain-testnet-infra/blob/main/docs/handoffs/2026-11-03/client-branches.md)
and [candidate image record](https://github.com/asi-alliance/asi-chain-testnet-infra/blob/main/docs/releases/wallet-cli-pr6-20261103-8f6c20ea43eb.md)
for exact source/image evidence. History/export acceptance remains under #5;
branch selection and green packaging checks do not replace deployment acceptance.

Testnet ECR builds default to remote dev unless the owner explicitly selects
another source. Freeze its full SHA and preserve the committed client gitlink.
The [image inventory](https://github.com/asi-alliance/asi-chain-testnet-infra/blob/main/docs/releases/image-inventory-20261103.md#published-image-sources)
records the current image built from PR #6 commit `8f6c20ea43eb`; its source tree
equals the dev merge `3a8f31e`. Later documentation commits do not change that
runtime or relabel the published artifact.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (edition 2024)
- Access to ASI Chain network nodes (Validator and Observer)
- GraphQL API endpoint for deploy history

## Installation

Clone the repository with submodules:

```bash
git clone --branch dev --recurse-submodules https://github.com/asi-alliance/asi-chain-wallet-cli.git
cd asi-chain-wallet-cli
```

The submodule uses the exact client commit documented above. Use that gitlink;
do not use `--remote` to replace it with a moving branch:
```bash
git submodule update --init --recursive
```

Build the project:

```bash
cargo build --release --locked
```

## Configuration

Create a `.env` file from the example:

```bash
cp .env.example .env
```

Edit `.env` with your network configuration:

```env
VALIDATOR_GRPC_URL=http://127.0.0.1:40412
VALIDATOR_HTTP_URL=http://127.0.0.1:40413
OBSERVER_HTTP_URL=http://127.0.0.1:40453
OUTPUT_DIR=./credentials
EXPECTED_TOKEN_SYMBOL=ASI
SHARD_ID=root
VERBOSE_LOGS=false
GRAPHQL_URL=wss://your-gateway.example/v1/graphql
```

These loopback ports illustrate a locally mapped validator/observer; use the
actual endpoints of your deployment. HTTP URLs support a gateway base path.
`https://` uses TLS; optional `NODE_CA_CERT` supplies a private CA for all endpoints.
`NODE_CONNECT_TIMEOUT_SECONDS` / `NODE_TIMEOUT_SECONDS` default to 10 / 30.

Legacy `VALIDATOR_HOST` plus `VALIDATOR_GRPC_PORT` / `VALIDATOR_HTTP_PORT`
remain supported when URLs are absent. For the observer, supply
`OBSERVER_HTTP_URL` or `OBSERVER_HOST` plus **`OBSERVER_HTTP_PORT`** explicitly.
`OBSERVER_GRPC_PORT` is no longer used. HTTP ports are never inferred from gRPC.

`GRAPHQL_URL` is needed only for history/export. Its schema/permissions must
match the selected indexer/Hasura release; do not put admin credentials into a
public URL. Secure WebSocket (`wss`) transport is enabled.

`DEPLOY_MAX_WAIT_SECONDS` defaults to 480 and
`DEPLOY_CHECK_INTERVAL_SECONDS` to 5. Both must be positive; the poll interval
must not exceed the total wait budget.

## Compatibility baseline and tests

The candidate uses the exact client/node pins above, the root Cargo.lock and
`nightly-2026-02-09`. Registry dependency versions are unchanged. Run existing
checks with `CARGO_BUILD_JOBS=2`:

```bash
cargo fmt --check
cargo clippy --all-targets --features integration --locked -- -D warnings
cargo test --release --locked
cargo build --release --locked
FIREFLY_NODE_IMAGE=public.ecr.aws/l2u2v4a8/asi-chain/node-rust@sha256:e6e82cdc5c9bc794367e15314d9c22256a8ebc0b2457d9492d633b2453c4c1df \
cargo test --release --features integration --test node_compatibility --locked
```

**Live acceptance gap:** the unchanged workflow/topology pins the image below
from node source `5dafbc37`, which predates #881 and cannot validate this client.
The existing node CI job remains enabled; it is expected to fail until a compatible
published immutable image with verified source provenance is accepted separately.
Client #88 records candidate digest
`f1r3flyindustries/f1r3fly-rust@sha256:4eab74400b569aff7718d63b5551e97a2df6e568ff4799bce4366ac8545f7646`,
but local labels/`--version` did not independently verify its source revision.
No image is replaced or rebuilt here, and no live wallet acceptance is claimed.

The last command requires Docker and creates its own shard with fresh test
keys/state. Set the image explicitly as above: the upstream helper has a different
default. The owner-selected immutable linux/amd64 image remains:

```text
public.ecr.aws/l2u2v4a8/asi-chain/node-rust@sha256:e6e82cdc5c9bc794367e15314d9c22256a8ebc0b2457d9492d633b2453c4c1df
```

It reuses fixture helpers from the exact pinned client submodule and the
versioned topology in `tests/integration/topology`. Loopback ports 40400-40462
must be free; an existing `rust-client-it` project causes refusal before any
cleanup. Only its freshly created shard is torn down. No node build or existing
genesis change is performed.

See [issue #5](https://github.com/asi-alliance/asi-chain-wallet-cli/issues/5).
The wallet command test checks exact balances/transfers, vault rejection and
an intentionally withheld finalization verdict. Full history/export acceptance
against the deployed Hasura schema remains a separate staging requirement.

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

A deploy ID is printed before submission, so a lost response still leaves a
queryable identifier. Success requires canonical finalization and a positive
vault result. A rejected/failed/expired transfer exits non-zero. Missing or
conflicting verdicts, transport errors and timeouts also exit non-zero and
report an unknown outcome with the original deploy ID. Check that ID; never
create a replacement payment automatically.

### Deploy History

Monitor deploy history in real-time:

```bash
# Display last 10 deployments and monitor for new ones
cargo run -- history

# Custom limit
cargo run -- history --limit 20

# or use alias
cargo run -- hist --limit 20
```

This command displays recent deployments and continuously monitors for new deploys via WebSocket subscription.

### Export Data

Export wallet deploy history:

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

## Docker

You can build and run the wallet CLI using Docker instead of compiling from source.

### Build Docker Image

```bash
docker build -t wallet-cli .
```

### Run with Docker Compose 

Run commands:

```bash
docker compose run --rm wallet-cli balance
docker compose run --rm wallet-cli send --to <ADDRESS> --amount 1000
```

## Token Decimals

`--amount` is an exact unsigned integer in raw base units (dust), up to
`2^256 - 1`. Zero, negative, fractional, exponent and overflowing inputs fail.
No floating-point scaling is performed. Balance display uses the node's native
token metadata and checks the expected symbol/shard before queries or transfers.

For an ASI network configured with **8 decimals**, the conversion is:

| Human Amount | CLI Amount |
|--------------|------------|
| 0.5 ASI | 50000000 |
| 1 ASI | 100000000 |
| 10 ASI | 1000000000 |
| 100 ASI | 10000000000 |

**Formula:** `CLI Amount = Human Amount × 10^decimals`, with decimals from the actual network.

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

## Contributing

Known issues and bugs are tracked in the [Issues](https://github.com/asi-alliance/asi-chain-wallet-cli/issues) section.  
If you encounter a problem that is not listed, please feel free to open a new issue and provide as much detail as possible.

---

## License

This project is licensed under the Apache License 2.0. See [LICENSE](LICENSE) file for details.
