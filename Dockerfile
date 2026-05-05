FROM rust:1.87-slim-bookworm AS builder
RUN apt-get update && apt-get install -y \
    protobuf-compiler libprotobuf-dev pkg-config libssl-dev \
    cmake clang libclang-dev libzstd-dev \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY aoee/ ./aoee/
WORKDIR /app/aoee
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libzstd1 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/aoee/target/release/aoee-server /usr/local/bin/aoee-server
EXPOSE 50051
CMD ["aoee-server"]
