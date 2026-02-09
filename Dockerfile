FROM rust:1.93-slim AS builder

WORKDIR /app

# Copy manifests first for layer caching
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    mkdir -p src && echo "pub fn rocket() -> rocket::Rocket<rocket::Build> { todo!() }" > src/lib.rs && \
    cargo build --release 2>/dev/null || true && \
    rm -rf src

# Copy real source and rebuild
COPY src/ src/
COPY openapi.json .

# Touch source files to force rebuild
RUN find src -name '*.rs' -exec touch {} +
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/local-agent-chat /usr/local/bin/local-agent-chat

# Create data directory
RUN mkdir -p /data

ENV DATABASE_PATH=/data/chat.db
ENV ROCKET_ADDRESS=0.0.0.0
ENV ROCKET_PORT=8000

EXPOSE 8000

VOLUME ["/data"]

CMD ["local-agent-chat"]
