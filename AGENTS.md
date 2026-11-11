# Wallet CLI agent guidance

Read [README.md](README.md), especially [branch status and client dependency](README.md#branch-status-and-client-dependency), before work. Recheck current refs/PRs: the recorded branch snapshot is dated 2026-11-03.

## Branch and dependency selection

- Use wallet `dev` for the current ASI client 0.2.9 / Rust-node 0.4.46 integration. Wallet #6 is merged; #3 promotes dev to main and remains open at this snapshot. The repository default `main` still contains the older code pending that promotion; it is a release branch, not deprecated.
- The merged `fix/asi-client-029-node-046` branch is review/image provenance, not the current development base. The separate `feature/cost-accounted-f1r3node` / #4 requires its own node/client compatibility review and is outside this regular-node integration.
- The Cargo git dependency uses `asi-alliance/asi-chain-client`, branch `rust-dev`, pinned to `5d1c9b4d12e33ec8b64a74a28df196ef03478e73` (2026-11-11), with node protocol crates at `0ccc315d84b32d668e8db49cd56ba3c576693e15`. It preserves the wallet's ASI metadata/vault ABI. Phlo prices must fit positive `i64` and are encoded as uint256 protobuf values; HTTP integers accept numeric and string encodings. Keep the exact `rev` in `Cargo.toml` and the matching Git source in `Cargo.lock`; do not use a sibling checkout or a floating branch.
- Preserve ASI native-token metadata, exact raw-dust amounts and current system-vault ABI. Runtime client/node changes need a new exact pin and acceptance evidence; do not replay historical REV URI strings or old API structs blindly.

## Work and validation

Check `git status --short --branch` before edits. Read `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `.github/workflows/compatibility.yml` and the selected source revision. Preserve unrelated local changes. If a checkout differs from the expected branch, compare files with the published revision before any alignment. Follow applicable workspace/owner Git permissions; this file grants no staging, commit, push, branch-change or merge permission.

Use the pinned toolchain, at most two build jobs (`CARGO_BUILD_JOBS=2`), and existing checks appropriate to runtime changes:

```bash
cargo fmt --check
cargo clippy --all-targets --features integration --locked -- -D warnings
cargo test --release --locked
cargo build --release --locked
cargo test --release --features integration --test node_compatibility --locked -- --nocapture
```

The owner authorized a new integration image from upstream `F1R3FLY-io/f1r3node-rust` dev at `49ce9074bc5aafef24ab5c25bfdd60685d64d464` on 2026-11-11. The workflow and topology must pin the same published digest. The former image at `e6e82cdc5c9bc794367e15314d9c22256a8ebc0b2457d9492d633b2453c4c1df` uses the old monetary protobuf and rejects this client's signatures. Keep the node CI job enabled and report failures honestly. Further source/image changes require a new compatibility review.

The integration command starts fresh isolated node state through the owned shard harness; inspect its ownership checks and do not stop an existing network or reuse real keys. Use the exact selected node-image digest in the workflow/topology. The owner authorized this dev image rebuild for the CI correction. The active source is `F1R3FLY-io/f1r3node-rust`; deprecated combined-parent and Scala sources are excluded.

For documentation-only changes, check links, branch roles, source revisions and whitespace; do not rebuild the node or application just for prose. Keep all published image labels/tags/digests true to their source.

For testnet ECR builds, use remote dev unless the owner explicitly selects another branch/revision/tag/release. Resolve its full SHA and build from clean verified inputs. Keep the committed client revision unchanged unless a separate compatibility update is authorized. Source compatibility does not authorize image publication or deployment.

## Command and security contracts

- Transfer amounts are raw dust and exact integers. Success requires canonical finalization plus a positive vault verdict. Unknown outcomes must retain the original deploy ID and must not cause an automatic second payment.
- Use explicit validator/observer URLs. Legacy observer settings require `OBSERVER_HTTP_PORT`; the old observer gRPC port is not a replacement. History/export require actual Hasura schema/permission/WebSocket acceptance under issue #5.
- Never commit private keys, real environment files, credentials, TLS private material or wallet exports. Keep funding/signing access separate from build and packaging checks. Do not use live-network transactions as a routine test.
