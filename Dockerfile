# Stage 1: Build Web Frontend
FROM node:26-alpine AS web-builder
WORKDIR /app/web
COPY web/package*.json ./
RUN npm ci
COPY web/ ./
RUN npm run build

# Stage 2: Build Rust Backend
FROM rust:1.99-bookworm AS rust-builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/
RUN cargo build --release

# Stage 3: Runtime Image
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=rust-builder /app/target/release/mymessages /usr/local/bin/mymessages
COPY --from=web-builder /app/web/dist /app/web/dist
COPY mymessages.toml /app/mymessages.toml

ENV MYMESSAGES_DATA_DIR=/data
ENV MYMESSAGES_PORT=8090
ENV MYMESSAGES_STATIC_DIR=/app/web/dist

VOLUME ["/data"]
EXPOSE 8090

ENTRYPOINT ["mymessages"]
CMD ["serve", "--config", "/app/mymessages.toml"]
