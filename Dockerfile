FROM rust:slim-bookworm@sha256:bec632fe9144b069d4a21396ca4f84f6d6c6d471ec1391cb70eb4bb1b06c547d AS builder
WORKDIR /build
ENV CARGO_BUILD_JOBS=3
RUN apt-get update && apt-get install -y --no-install-recommends build-essential cmake git libclang-dev libprotobuf-dev libssl-dev make perl pkg-config protobuf-compiler && rm -rf /var/lib/apt/lists/*
COPY rust-toolchain.toml ./
RUN rustup toolchain install nightly-2026-02-09 --profile minimal
COPY . .
RUN --mount=type=cache,id=asi-wallet-cli-registry,target=/usr/local/cargo/registry --mount=type=cache,id=asi-wallet-cli-git,target=/usr/local/cargo/git --mount=type=cache,id=asi-wallet-cli-v029-target,target=/build/target cargo build --release --locked && mkdir -p /release && cp target/release/asi-chain-wallet-cli /release/asi-chain-wallet-cli && rustc -Vv > /release/compiler-version.txt

FROM debian:bookworm-slim@sha256:a3bba7dc7dbcc1f37c2856c51fc9a7fac96608f50cb87b1b4e4d8b8c85ae988e
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libssl3 && rm -rf /var/lib/apt/lists/* && groupadd --gid 10001 asi && useradd --uid 10001 --gid 10001 --create-home asi
WORKDIR /app
COPY --from=builder /release/asi-chain-wallet-cli /app/asi-chain-wallet-cli
COPY --from=builder /build/Cargo.lock /usr/share/asi-release/Cargo.lock
COPY --from=builder /release/compiler-version.txt /usr/share/asi-release/compiler-version.txt
USER 10001:10001
ENTRYPOINT ["/app/asi-chain-wallet-cli"]
