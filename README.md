# ASI Chain Wallet CLI

## Setup

Create a `.env` file:

```bash
cp .env.example .env
```

Edit `.env` with configuration. At least `<VALIDATOR_IP>` and `<OBSERVER_IP>` are needed to be filled in.

Initialize submodules:

```bash
git submodule init
git submodule update
```

Keep rust-client submodule up-to-date by running:

```bash
git submodule update --init --remote -- rust-client
``` 

## Build

```bash
cargo build --release
```

## Usage

### Create a new wallet

```bash
cargo run -- create --save
```

This generates a new wallet and saves the credentials to `./credentials/wallet_info.json`.

### Check balance

```bash
cargo run -- balance
```

### Send tokens

```bash
cargo run -- send --to <recipient-address> --amount <amount>
```

**Example:**

```bash
cargo run -- send --to 1111AtahZeefej4tvVR6ti9TJtv8yxLebT31SCEVDCKMNikBk5r3g --amount 100
```

**Note:**  
If token uses 8 decimal places use `--amount 100000000` (1 × 10⁸) to send 1 token.  
Examples:
- 0.5 token = `50000000`
- 1 token = `100000000`
- 10 token = `1000000000`

### Monitor deploy history in real-time

```bash
cargo run -- history
```

Or with a custom limit (default is 10):

```bash
cargo run -- history --limit 20
```

This command:
- Displays the last N deployments (both incoming/outgoing transfers and other deployments)
- Continuously monitors for new transactions via WebSocket subscription

### Export wallet data

Export wallet transaction history to a file for analysis or sharing:

```bash
# Export as JSON (default)
cargo run -- export
```

**Options:**

```bash
cargo run -- export --format json

# Export in CSV format
cargo run -- export --format csv

# Specify custom output path
cargo run -- export --output my_data.json

# Export with custom limit (default is 100)
cargo run -- export --limit 50

# Combine options
cargo run -- export --format csv --output reports/wallet_2024.csv --limit 200
```

Exported files are saved to the `exports/` directory by default with timestamp-based filenames.

## Help

```bash
cargo run -- help
```
