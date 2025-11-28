mod config;
mod db;
mod models;
mod pokeapi;
mod schema;
use crate::config::Config;
use crate::db::{init_pool, migrate};
use crate::pokeapi::PokeApi;
use crate::schema::{build_schema, PokemonSchema};
use axum::{Extension, Router};
use std::net::SocketAddr;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing()?;
    let cfg = Config::from_env();
    let pool = init_pool(&cfg.database_url).await?;
    migrate(&pool).await?;

    let pokeapi = PokeApi::new();
    let schema: PokemonSchema = build_schema(pokeapi);

    async fn graphql_handler(
        schema: Extension<PokemonSchema>,
        req: axum::extract::Json<async_graphql::Request>,
    ) -> axum::Json<async_graphql::Response> {
        axum::Json(schema.execute(req.0).await)
    }

    let app = Router::new()
        .route("/graphql", axum::routing::post(graphql_handler))
        .layer(Extension(schema));

    let addr: SocketAddr = format!("0.0.0.0:{}", cfg.port).parse()?;
    tracing::info!(%addr, "starting GraphQL server");
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    opentelemetry::global::shutdown_tracer_provider();
    Ok(())
}

fn init_tracing() -> anyhow::Result<()> {
    use opentelemetry::trace::TracerProvider as _;
    use opentelemetry::KeyValue;
    use opentelemetry_otlp::WithExportConfig;
    use opentelemetry_sdk::Resource;

    Registry::default()
        .with(EnvFilter::from_default_env())
        .with(fmt::layer())
        .with(
            tracing_opentelemetry::layer().with_tracer(
                opentelemetry_otlp::new_pipeline()
                    .tracing()
                    .with_exporter(
                        opentelemetry_otlp::new_exporter().tonic().with_endpoint(
                            std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
                                .unwrap_or_else(|_| "http://jaeger:4317".to_string()),
                        ),
                    )
                    .with_trace_config(opentelemetry_sdk::trace::Config::default().with_resource(
                        Resource::new(vec![KeyValue::new("service.name", "pokemon-api")]),
                    ))
                    .install_batch(opentelemetry_sdk::runtime::Tokio)?
                    .tracer("pokemon-api"),
            ),
        )
        .init();

    Ok(())
}
