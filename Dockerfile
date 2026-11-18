# Stage 1: Build Web Frontend
FROM node:26-alpine AS web-builder
WORKDIR /app/web
COPY web/package*.json ./
RUN npm ci
COPY web/ ./
RUN npm run build

# Stage 2: Build Rust Backend
FROM rust:1.99-alpine AS rust-builder
RUN apk add --no-cache musl-dev gcc make
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/
RUN cargo build --release

# Stage 3: Ultra-Minimal Alpine Runtime Image
FROM alpine:3.21
RUN apk add --no-cache ca-certificates tzdata

# Create dedicated unprivileged non-root user and group
RUN addgroup -g 10001 -S mymessages && \
    adduser -u 10001 -S mymessages -G mymessages -h /app -s /sbin/nologin

WORKDIR /app
COPY --from=rust-builder --chown=mymessages:mymessages /app/target/release/mymessages /usr/local/bin/mymessages
COPY --from=web-builder --chown=mymessages:mymessages /app/web/dist /app/web/dist
COPY --chown=mymessages:mymessages mymessages.toml /app/mymessages.toml

# Pre-create data directory with non-root ownership
RUN mkdir -p /data && chown -R mymessages:mymessages /data /app

# Run as non-root
USER 10001:10001

ENV MYMESSAGES_DATA_DIR=/data
ENV MYMESSAGES_PORT=8765
ENV MYMESSAGES_STATIC_DIR=/app/web/dist

VOLUME ["/data"]
EXPOSE 8765

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
  CMD wget --spider -q http://127.0.0.1:8765/health || exit 1

ENTRYPOINT ["mymessages"]
CMD ["serve", "--config", "/app/mymessages.toml"]
