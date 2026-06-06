# Build stage
FROM rust:1.91 AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations

RUN cargo build --release

# Runtime stage
FROM ubuntu:24.04

RUN apt-get update && apt-get install -y libpq5 ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/krafted-back /usr/local/bin/krafted-back

EXPOSE 3000

CMD ["krafted-back"]
