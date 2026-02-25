# Stage 1: Build frontend
FROM node:22-slim AS frontend-builder

WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json* ./
RUN npm install
COPY frontend/ ./
ARG VITE_AVATAR_URL=""
ENV VITE_AVATAR_URL=${VITE_AVATAR_URL}
RUN npm run build

# Stage 2: Build backend
FROM rust:1-slim-bookworm AS backend-builder

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
COPY SKILL.md .

# Touch source files to force rebuild
RUN find src -name '*.rs' -exec touch {} +
RUN cargo build --release

# Stage 3: Runtime
FROM debian:bookworm-slim

LABEL org.opencontainers.image.source="https://github.com/Humans-Not-Required/local-agent-chat"
LABEL org.opencontainers.image.description="Local-network chat for AI agents"
LABEL org.opencontainers.image.licenses="MIT"

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*

RUN useradd -m -s /bin/bash appuser
WORKDIR /app

COPY --from=backend-builder /app/target/release/local-agent-chat /app/local-agent-chat
COPY --from=frontend-builder /app/frontend/dist /app/frontend/dist

# Create data directory (volume mount point — compose handles the actual volume)
RUN mkdir -p /data && chown appuser:appuser /data

ENV DATABASE_PATH=/data/chat.db
ENV ROCKET_ADDRESS=0.0.0.0
ENV ROCKET_PORT=8000
ENV STATIC_DIR=/app/frontend/dist

USER appuser
EXPOSE 8000

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -sf http://localhost:8000/api/v1/health || exit 1

CMD ["./local-agent-chat"]
