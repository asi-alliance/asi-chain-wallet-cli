# Wallet CLI agent guidance

Read [README.md](README.md), especially [branch status and client dependency](README.md#branch-status-and-client-dependency), before work. Recheck current refs/PRs: the recorded branch snapshot is dated 2026-11-03.

## Branch and dependency selection

- Use wallet `dev` for the current ASI client 0.2.9 / Rust-node 0.4.46 integration. Wallet #6 is merged; #3 promotes dev to main and remains open at this snapshot. The repository default `main` still contains the older code pending that promotion; it is a release branch, not deprecated.
- The merged `fix/asi-client-029-node-046` branch is review/image provenance, not the current development base. The separate `feature/cost-accounted-f1r3node` / #4 requires its own node/client compatibility review and is outside this regular-node integration.
- Inspect the committed `rust-client` gitlink and `.gitmodules`. The canonical dependency is `asi-alliance/asi-chain-client`; the tested wallet pin is `77cbdc8180d8f83a7216dbf54b23268106ea3e91`. Never infer this dependency from a sibling checkout, client package version or branch name. Do not run submodule updates with `--remote` as routine setup.
- Client #3 merged into its active ASI `dev` as `40f2a94`, preserving runtime `77cbdc8`. The client's GitHub default remains `asi-main` during the transition; client `main`, `asi-main` and `asi-dev` are not substitutes for the tested gitlink. Read the linked infra branch audit before changing the dependency or assuming the normalization is complete.
- Preserve ASI native-token metadata, exact raw-dust amounts and current system-vault ABI. Runtime client/node changes need a new exact pin and acceptance evidence; do not replay historical REV URI strings or old API structs blindly.

## Work and validation

Check `git status --short --branch` before edits. Read `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `.github/workflows/compatibility.yml` and the selected source revision. Preserve unrelated local changes. If a checkout differs from the expected branch, compare files with the published revision before any alignment. Follow applicable workspace/owner Git permissions; this file grants no staging, commit, push, branch-change or merge permission.

Use the pinned toolchain and run checks appropriate to runtime changes:

```bash
cargo fmt --check
cargo clippy --all-targets --features integration --locked -- -D warnings
cargo test --release --locked
cargo build --release --locked
cargo test --release --features integration --test node_compatibility --locked -- --nocapture
```

The integration command starts fresh isolated node state through the owned shard harness; inspect its ownership checks and do not stop an existing network or reuse real keys. Use the exact existing node-image digest in the workflow/topology. Further node builds are paused by the owner. The active source is `F1R3FLY-io/f1r3node-rust`; deprecated combined-parent and Scala sources are excluded.

For documentation-only changes, check links, branch roles, source revisions and whitespace; do not rebuild the node or application just for prose. Keep all published image labels/tags/digests true to their source.

For testnet ECR builds, use remote dev unless the owner explicitly selects another branch/revision/tag/release. Resolve its full SHA and build from clean verified inputs. Keep the committed client gitlink unchanged unless a separate compatibility update is authorized. See the [infra source policy](https://github.com/asi-alliance/asi-chain-testnet-infra/blob/main/docs/tooling.md#source-branch-policy) and [image inventory](https://github.com/asi-alliance/asi-chain-testnet-infra/blob/main/docs/releases/image-inventory-20261103.md#published-image-sources).

## Command and security contracts

- Transfer amounts are raw dust and exact integers. Success requires canonical finalization plus a positive vault verdict. Unknown outcomes must retain the original deploy ID and must not cause an automatic second payment.
- Use explicit validator/observer URLs. Legacy observer settings require `OBSERVER_HTTP_PORT`; the old observer gRPC port is not a replacement. History/export require actual Hasura schema/permission/WebSocket acceptance under issue #5.
- Never commit private keys, real environment files, credentials, TLS private material or wallet exports. Keep funding/signing access separate from build and packaging checks. Do not use live-network transactions as a routine test.
