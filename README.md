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
10. [Compatibility baseline and tests](#compatibility-baseline-and-tests)

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

Check current PR status and branch revisions before starting new work.

| Wallet CLI branch | Current role | Use for the current Rust-node integration |
|---|---|---|
| `dev` | Active integration branch | Start current work here. Wallet [#6](https://github.com/asi-alliance/asi-chain-wallet-cli/pull/6) introduced the typed client integration. |
| `main` | Repository default and release branch; still `cd5d37ce651119c39de58e910db538729948e270` while [#3](https://github.com/asi-alliance/asi-chain-wallet-cli/pull/3) is open | Await the reviewed dev-to-main promotion for this integration. Main is a release branch, not deprecated. A plain default-branch clone currently gets older code. |
| `fix/asi-client-029-node-046` | Source of merged [#6](https://github.com/asi-alliance/asi-chain-wallet-cli/pull/6), candidate `8f6c20e` | Historical review/image source; continue development from dev. |
| `feature/cost-accounted-f1r3node` | Separate cost-accounting runtime, [#4](https://github.com/asi-alliance/asi-chain-wallet-cli/pull/4) | Outside this regular-node integration. Review its node/client pair separately. |

The Cargo Git dependency pins `F1R3FLY-io/rust-client` at
`2fedf97e66a56b7a0b4cce06eb546d36c221b58e`, the `dev` merge of
[Rust client PR #103](https://github.com/F1R3FLY-io/rust-client/pull/103).
It reads structured gRPC queue causes and supports legacy queue messages.
It includes the refusal reconciliation and verdict handling from
[Rust client PR #86](https://github.com/F1R3FLY-io/rust-client/pull/86).
Its node protocol crates are pinned to `aca631e2ed02f73170558fbc3194efa48b9f1876`.
The library preserves ASI metadata, exact raw-dust amounts and the
`rho:vault:system` transfer ABI; its CLI remains disabled.
Send reads the submitting validator's `/api/status` once before signing and
checks its token symbol and shard. It uses the exact minimum Phlo price with a
floor of one and signs with a uint256 price, subject to the client checking that
price × Phlo limit fits in uint256. The transfer Phlo limit is capped at the
validator's reported `maxPhloLimit`; a null or non-positive cap is rejected, and
when no cap is reported the default transfer limit is preserved.
HTTP prices accept exact JSON integers and decimal strings.

`Cargo.toml` selects the exact commit with `rev`, and `Cargo.lock` records the
Git source. Cargo fetches the client automatically.
The integration image is built from upstream `F1R3FLY-io/f1r3node-rust` dev at
`49ce9074bc5aafef24ab5c25bfdd60685d64d464`, selected on 2026-11-11.
The workflow and test topology pin
`amamata/rust-node@sha256:f835f503efbfc27d28025feb2710e3baa07e15e71c9553ac036b4a6d6c4a2eab`
(published tag `dev-49ce9074bc5a`). The previous
image used the old monetary protobuf and rejected this client's signatures.
On the previous client pin, local acceptance passed on this image with 7 unit
tests, 2 HTTP tests, and the full node scenario (197.13 seconds).

History/export acceptance remains under
[issue #5](https://github.com/asi-alliance/asi-chain-wallet-cli/issues/5);
branch selection and green packaging checks do not replace deployment acceptance.

Testnet ECR builds default to remote dev unless the owner explicitly selects
another source. Freeze its full SHA and preserve the committed client revision.
Published images retain their original source identity. A client pin update
does not rebuild or relabel an existing artifact.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (edition 2024)
- Access to ASI Chain network nodes (Validator and Observer)
- GraphQL API endpoint for deploy history

## Installation

Clone the repository:

```bash
git clone --branch dev https://github.com/asi-alliance/asi-chain-wallet-cli.git
cd asi-chain-wallet-cli
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
GRAPHQL_TIMEOUT_SECONDS=30
```

These loopback ports illustrate a locally mapped validator/observer; use the
actual endpoints of your deployment. HTTP URLs support a gateway base path.
`https://` uses TLS; optional `NODE_CA_CERT` supplies a private CA for all endpoints.
`NODE_CONNECT_TIMEOUT_SECONDS` / `NODE_TIMEOUT_SECONDS` default to 10 / 30.

Legacy `VALIDATOR_HOST` plus `VALIDATOR_GRPC_PORT` / `VALIDATOR_HTTP_PORT`
remain supported when URLs are absent. For the observer, supply
`OBSERVER_HTTP_URL` or `OBSERVER_HOST` plus **`OBSERVER_HTTP_PORT`** explicitly.
`OBSERVER_GRPC_PORT` is no longer used. HTTP ports are never inferred from gRPC.

`GRAPHQL_TIMEOUT_SECONDS` defaults to 30 and must be positive. It bounds the
WebSocket connection/acknowledgement and the entire export query, including
completion. It does not limit the lifetime of an established history subscription.
Export connection/operation errors, payload errors, invalid responses and premature
termination exit non-zero before creating or overwriting an export file.
History and JSON/CSV exports preserve integer dust amounts, including uint256
values returned as JSON numbers or decimal strings.

`GRAPHQL_URL` is needed only for history/export. Its schema/permissions must
match the selected indexer/Hasura release; do not put admin credentials into a
public URL. Secure WebSocket (`wss`) transport is enabled.

`DEPLOY_MAX_WAIT_SECONDS` defaults to 480 and
`DEPLOY_CHECK_INTERVAL_SECONDS` to 5. Both must be positive; the poll interval
must not exceed the total wait budget.

## Deploy admission refusals

The client reports explicit refusals from node admission and from the deploy queue.
The CLI shows the original deploy ID and offers retry guidance for retryable
refusals. A rejection after queue admission is a terminal transfer failure.
Submission and confirmation timeouts keep the unknown-outcome warning and direct
the operator to check the original deploy ID before creating another payment.
A finalized deploy whose result could not be read is reported as final: read the
result by deploy ID and do not resubmit it.
[Node admission PR #789](https://github.com/F1R3FLY-io/f1r3node-rust/pull/789),
[deploy queue PR #990](https://github.com/F1R3FLY-io/f1r3node-rust/pull/990)

When a deploy queue fronts the validator, point both `VALIDATOR_GRPC_URL` and
`VALIDATOR_HTTP_URL` at that queue and keep `OBSERVER_HTTP_URL` on the observer.
Pointing only the gRPC URL at the queue sends the submission to the queue while
status polling bypasses its local `Rejected` record and can time out.

## Compatibility baseline and tests

The candidate uses the exact client/node pins above, the root Cargo.lock and
`nightly-2026-02-09`. Registry dependency versions are unchanged. Run existing
checks with `CARGO_BUILD_JOBS=2`:

```bash
cargo fmt --check
cargo clippy --all-targets --features integration --locked -- -D warnings
cargo test --release --locked
cargo build --release --locked
FIREFLY_NODE_IMAGE=amamata/rust-node@sha256:f835f503efbfc27d28025feb2710e3baa07e15e71c9553ac036b4a6d6c4a2eab \
cargo test --release --features integration --test node_compatibility --locked
```

The last command requires Docker and creates its own shard with fresh test
keys/state. The wallet owns the fixture helpers in `tests/support`, copied from
the pinned client revision under Apache-2.0, and the topology in
`tests/integration/topology`. Both the harness default and CI select the immutable
node image documented above. Loopback ports 40400-40462 must be free (set
`FIREFLY_PORT_BASE`, e.g. `41400`, to use another host port range); an existing
`rust-client-it` project causes refusal before any cleanup. Only its freshly
created shard is torn down. No node build or existing genesis change is performed.

See [issue #5](https://github.com/asi-alliance/asi-chain-wallet-cli/issues/5).
The wallet command test checks exact balances/transfers, vault rejection and
an intentionally withheld finalization verdict. A separate deploy queue forwards
gRPC tip reads to the validator while its HTTP event link is disconnected.
The test submits a signed transfer through that queue and checks the CLI's exit
status, original deploy ID, refusal reason, retry guidance and recipient balance.
The queue fixture uses `DEPLOY_QUEUE_IMAGE`, defaulting to the published ASI node
build `0380cbbd494c`
(`public.ecr.aws/l2u2v4a8/asi-chain/node-rust@sha256:65e732bfaf26025a94216c529da8baad2049b4b561ffe0d44c3a547f0f3834c8`),
because the older validator image does not ship the queue binary. Full
history/export acceptance
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

Output includes the private key, public key, and ASI address. Saving a wallet requires
a new `wallet_info.json` path. Choose another `OUTPUT_DIR` to keep several wallets.
On Unix, the wallet is created with owner-only permissions; the file and directory
entries are synced before success is reported. A failed save never prints the new
private key or replaces an existing wallet.

For a watch-only wallet, put the matching `public_key` and `address` in
`wallet_info.json`. These fields are sufficient for `balance`, `history` and
`export`; `private_key` may be omitted or empty. Public keys are validated and
normalized to lowercase uncompressed hex before indexer queries. Add the matching
`private_key` only on the machine that runs `send`, which checks that the private
key, public key and address match.

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

`send` submits the signed transfer once by default. To allow one retry of the
same signed bytes within this invocation, add `--retry-deployer-share`. The
CLI waits for the queue's `retry-after` only when the queue identifies the
sender's share limit and the wait fits before expiration. A second refusal
ends the command. Starting a new `send` signs a different deploy.

For validator capacity, validator inactivity, shard finality or storage
pressure, the CLI prints the cause and any wait. After the wait a new transfer
needs a current block height. An unknown cause produces a generic queue-full
refusal. For an unknown submission outcome, check the original deploy ID.

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
An empty deployments array followed by operation completion exits successfully,
prints "No deployments found" and creates no export file. Errors or a missing
completion fail the command; an existing output file is preserved on these failures.

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

On Linux, create the bind-mounted directories as the account that owns the
wallet files. Run the CLI with that account's UID and GID so it can save the
wallet and write exports. The image defaults to UID and GID 10001.

```bash
(umask 077 && mkdir -p credentials exports)
docker compose run --rm --user "$(id -u):$(id -g)" wallet-cli balance
docker compose run --rm --user "$(id -u):$(id -g)" wallet-cli send --to <ADDRESS> --amount 1000
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
└── tests/
    ├── exact_http.rs       # Pinned-client HTTP parsing checks
    ├── wallet_creation.rs  # Safe wallet persistence regressions
    ├── wallet_reads.rs     # Watch-only and GraphQL CLI regressions
    ├── node_compatibility.rs # Isolated-node transfer and timeout checks
    ├── support/            # Wallet-owned node test fixtures
    └── integration/topology/ # Docker Compose test shard
```

## Contributing

Known issues and bugs are tracked in the [Issues](https://github.com/asi-alliance/asi-chain-wallet-cli/issues) section.  
If you encounter a problem that is not listed, please feel free to open a new issue and provide as much detail as possible.

---

## License

This project is licensed under the Apache License 2.0. See [LICENSE](LICENSE) file for details.
