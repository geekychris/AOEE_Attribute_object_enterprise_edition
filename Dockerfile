# AOEE gRPC server (Rust)
# Multi-stage: cargo build --release inside a rust:1-bookworm builder, then copy the static-ish
# binary into a slim debian. Build context is the AOEE repo root.
FROM rust:1.87-bookworm AS builder
WORKDIR /src
# Copy only the Rust workspace; aoee-spring (Java) and other dirs are not needed here.
COPY aoee /src/aoee
WORKDIR /src/aoee
# --bin selects only the server target so we skip aoee-bench, aoee-client, etc.
RUN cargo build --release --bin aoee-server

FROM debian:bookworm-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /src/aoee/target/release/aoee-server /app/aoee-server
ENV RUST_LOG=info \
    AOEE_LISTEN_ADDR=0.0.0.0:50051
EXPOSE 50051
ENTRYPOINT ["/app/aoee-server"]
