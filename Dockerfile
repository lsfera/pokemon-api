ARG BINARY_NAME_DEFAULT=pokemon-api
ARG TARGETARCH
ARG TARGETPLATFORM
ARG TARGETOS

FROM clux/muslrust:stable AS builder
RUN groupadd -g 10001 -r dockergrp && useradd -r -g dockergrp -u 10001 dockeruser
ARG BINARY_NAME_DEFAULT
ENV BINARY_NAME=$BINARY_NAME_DEFAULT
ARG TARGETARCH
ARG TARGETPLATFORM
ARG TARGETOS

# Determine target triple based on platform (BuildKit provides TARGETPLATFORM like linux/amd64)
RUN set -eux; \
    echo "Detected: TARGETPLATFORM=$TARGETPLATFORM TARGETARCH=$TARGETARCH TARGETOS=$TARGETOS"; \
    case "$TARGETPLATFORM" in \
    linux/amd64) TARGET_TRIPLE=x86_64-unknown-linux-musl ;; \
    linux/arm64) TARGET_TRIPLE=aarch64-unknown-linux-musl ;; \
    *) echo "Unsupported TARGETPLATFORM: $TARGETPLATFORM" >&2; exit 1 ;; \
    esac; \
    echo $TARGET_TRIPLE > /tmp/target_triple

# Copy sources early (no dependency optimization for simplicity & determinism)
COPY Cargo.lock Cargo.toml ./
COPY src ./src

# Build binary for resolved target triple and strip
RUN set -eux; TARGET_TRIPLE=$(cat /tmp/target_triple); \
    rustup target add "$TARGET_TRIPLE"; \
    cargo build --release --bin $BINARY_NAME --target "$TARGET_TRIPLE"; \
    mkdir -p /build-out; \
    cp target/"$TARGET_TRIPLE"/release/$BINARY_NAME /build-out/; \
    strip /build-out/$BINARY_NAME || true

FROM scratch AS runtime
COPY --from=builder /etc/passwd /etc/passwd
USER dockeruser

ARG BINARY_NAME_DEFAULT
ENV BINARY_NAME=$BINARY_NAME_DEFAULT

# Observability and configuration envs (override at runtime as needed)
ENV OTEL_EXPORTER_OTLP_ENDPOINT="http://jaeger:4317" \
    RUST_LOG="error,$BINARY_NAME=info" \
    DATABASE_URL="postgres://postgres:postgres@postgres:5432/postgres" \
    POKEAPI_BASE="https://pokeapi.co/api/v2"

COPY --from=builder /build-out/$BINARY_NAME /

EXPOSE 3000
ENTRYPOINT ["/pokemon-api"]
CMD []