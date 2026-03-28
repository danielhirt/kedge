FROM rust:1-slim AS builder

RUN apt-get update && apt-get install -y git && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY . .
RUN cargo build --release && strip target/release/kedge

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends git ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/kedge /usr/local/bin/kedge

ENTRYPOINT ["kedge"]
