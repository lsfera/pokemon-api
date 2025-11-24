# Pokemon API

A Rust web service built with Axum that exposes Pokemon-related endpoints while demonstrating:

- Async Rust with Tokio
- Structured logging & distributed tracing (OpenTelemetry + `tracing`)
- External API calls to the public PokeAPI with trace context propagation
- PostgreSQL connection pooling & migrations (`deadpool-postgres` + `tokio-postgres`)
- Error handling with `anyhow` / `thiserror`

## Features

- JSON endpoints using Axum 0.7
- Outbound HTTP requests via `reqwest` wrapped with `reqwest-middleware` and `reqwest-tracing`
- Automatic span creation for inbound requests (`tower-http` TraceLayer) and outbound calls
- OTLP export (traces) to a configurable collector (default: `http://jaeger:4317`)

## Project Structure

```
src/
  main.rs          # startup, tracing init, server bootstrap
  config.rs        # environment variable parsing
  db.rs            # pool init & migrations
  models.rs        # domain types & (de)serialization
  pokeapi.rs       # external PokeAPI client with tracing middleware
  routes.rs        # axum router + handlers
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | `3000` | Server listen port |
| `DATABASE_URL` | (required) | PostgreSQL connection string |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | `http://jaeger:4317` | OTLP collector endpoint for traces |
| `POKEAPI_BASE` | `https://pokeapi.co/api/v2` | Base URL for upstream PokeAPI |
| `RUST_LOG` | `info` | Log/trace filter (e.g. `RUST_LOG=debug,axum=trace`) |

## Quick Start

### 1. Set Environment

```bash
echo "DATABASE_URL=postgres://user:pass@localhost:5432/pokemon" > .env
export RUST_LOG=info
```

(You can also use `dotenvy` by placing variables in a `.env` file.)

### 2. Run PostgreSQL (example via Docker)

```bash
docker run --rm -e POSTGRES_PASSWORD=pass -e POSTGRES_USER=user -e POSTGRES_DB=pokemon -p 5432:5432 postgres:16
```

### 3. Build & Run

```bash
cargo build
cargo run
```

### 4. Test an Endpoint

```bash
curl http://localhost:3000/pokemon/ditto
```

(Adjust route names to what's implemented in `routes.rs`.)

## Tracing & Observability

- Inbound requests: A span is created per request; failures are classified.
- Outbound PokeAPI calls: Spans include HTTP metadata and propagate context.
- OTLP Export: Configure collector via `OTEL_EXPORTER_OTLP_ENDPOINT`. To run Jaeger locally with OTLP ingest:

```bash
docker run -d --name jaeger -e COLLECTOR_OTLP_ENABLED=true -p 16686:16686 -p 4317:4317 jaegertracing/all-in-one:1.51
```

Access UI at: http://localhost:16686 and search for `service.name = pokemon-api`.

## Migrations

If `migrate` uses embedded SQL files, ensure they are present; otherwise add your migration logic before running. (See `db.rs`.)

## Security Notes

- Uses `rustls` TLS stack (no OpenSSL default features), reducing native deps.
- Avoid exposing detailed error messages in production logs.

## Future Enhancements

- Add health check endpoint (`/health`)
- Metrics export (OTLP metrics feature already enabled)
- Proper DB migration tooling (e.g. `refinery` or `sqlx migrate`)
- Containerization & multi-arch builds

## Docker & Multi-Platform Images

This project includes a multi-stage `Dockerfile` producing a minimal static binary image (`scratch` base). It supports building for multiple architectures (amd64 & arm64) using BuildKit `buildx`.

### Local Single-Arch Build & Run

```bash
docker build -t pokemon-api:dev .
docker run --rm -p 3000:3000 --env-file .env pokemon-api:dev
```

### Enable Buildx (Once per Docker Host)

```bash
docker buildx create --name multi --use
docker buildx inspect --bootstrap
docker buildx ls
```

### Multi-Platform Build & Push

Replace `YOUR_REPO` with your registry namespace (e.g. `ghcr.io/owner` or `docker.io/username`).

```bash
docker login
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -t YOUR_REPO/pokemon-api:latest \
  -t YOUR_REPO/pokemon-api:$(git rev-parse --short HEAD) \
  --push .
```

### Load Locally (Single Arch) Without Pushing

```bash
docker buildx build --platform linux/amd64 -t pokemon-api:local --load .
```

### Docker Compose (Dev Stack: App + Postgres + Jaeger + PgAdmin)

```bash
docker compose up --build -d
docker compose logs -f app
```

The `app` service in `docker-compose.yaml` runs `sleep infinity` for interactive dev. Replace the command with the binary to run directly:

```yaml
  command: ["/pokemon-api"]
```

### Customizing Binary Name

The `Dockerfile` uses `ARG BINARY_NAME_DEFAULT` (default `pokemon-api`). To build a differently named binary:

```bash
docker build -t pokemon-alt --build-arg BINARY_NAME_DEFAULT=my-bin .
```

### Image Size & Contents

- Final stage is `scratch`; only the compiled binary and `/etc/passwd` (for non-root user) are present.
- TLS works because `rustls` embeds root certificates via the `webpki-roots` crate; no CA bundle needed in the image.

### Basic Health Check (Optional Dockerfile Snippet)

You can add a simple health check probing the main port:

```dockerfile
HEALTHCHECK --interval=30s --timeout=3s --retries=3 CMD ["/pokemon-api", "--health"]
```

Add a `--health` flag handler in `main.rs` if adopting this.

### SBOM (Optional)

To generate a CycloneDX SBOM for the image & source:

```bash
# Image SBOM (requires syft: https://github.com/anchore/syft)
syft YOUR_REPO/pokemon-api:latest -o cyclonedx-json > sbom-image.json

# Source SBOM
cargo install cargo-cyclonedx
cargo cyclonedx --output-format json --output sbom-source.json
```

### Troubleshooting

| Issue | Cause | Fix |
|-------|-------|-----|
| Build fails on unsupported platform | `TARGETPLATFORM` case not matched | Extend case statement in Dockerfile |
| Linker errors (`lld` missing) | Global rustflags forcing `-fuse-ld=lld` | Remove rustflags or install `lld` |
| Jaeger not receiving traces | Wrong endpoint | Set `OTEL_EXPORTER_OTLP_ENDPOINT` to collector URL |

### Running With Different Port

Expose a different port by setting `PORT` in `.env` and mapping it:

```bash
PORT=8080 docker run --rm -p 8080:8080 --env PORT=8080 --env-file .env pokemon-api:dev
```

---

## License

TBD (add license information if required).

---
Generated on 2025-11-24.
