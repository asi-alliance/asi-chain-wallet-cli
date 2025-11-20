# ASI Chain Wallet CLI

## Build

```bash
cargo build --release
```

## Setup

Create a `.env` file:

```bash
cp .env.example .env
```

Edit `.env` with configuration. At least <VALIDATOR_IP> and <OBSERVER_IP> are needed to be filled in.

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

## Help

```bash
cargo run -- help
```
