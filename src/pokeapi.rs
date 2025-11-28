use crate::models::{Pokemon, RemotePokemon};
use reqwest::Client;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_tracing::{SpanBackendWithUrl, TracingMiddleware};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PokeApiError {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("middleware error: {0}")]
    Middleware(#[from] reqwest_middleware::Error),
    #[error("not found")]
    NotFound,
}

#[derive(Clone)]
pub struct PokeApi {
    client: ClientWithMiddleware,
    base: String,
}

impl Default for PokeApi {
    fn default() -> Self {
        Self::new()
    }
}

impl PokeApi {
    pub fn new() -> Self {
        let client = ClientBuilder::new(Client::new())
            .with(TracingMiddleware::<SpanBackendWithUrl>::new())
            .build();
        Self {
            client,
            base: "https://pokeapi.co/api/v2".to_string(),
        }
    }

    pub async fn fetch_pokemon(&self, name_or_id: &str) -> Result<Pokemon, PokeApiError> {
        let url = format!("{}/pokemon/{}", self.base, name_or_id);
        let resp = self.client.get(url).send().await?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(PokeApiError::NotFound);
        }
        let remote: RemotePokemon = resp.error_for_status()?.json().await?;
        Ok(remote.into())
    }
}
