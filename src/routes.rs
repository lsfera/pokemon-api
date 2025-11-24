use crate::{models::Pokemon, pokeapi::PokeApi};
use axum::extract::OriginalUri;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{
    extract::{Host, Path, State},
    routing::get,
    Json, Router,
};
use chrono::Utc;
use deadpool_postgres::Pool;
use serde::Serialize;
use tokio_postgres::Row;
use tower_http::classify::ServerErrorsFailureClass;
use tower_http::trace::{DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::{info_span, instrument, Level, Span};

#[derive(Clone)]
pub struct AppState {
    pub pool: Pool,
    pub pokeapi: PokeApi,
}

#[derive(Serialize)]
struct ErrorMessage {
    error: String,
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/pokemon/:id", get(get_pokemon))
        .route("/pokemon", get(list_pokemon))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|req: &axum::http::Request<_>| {
                    info_span!(
                        "http.request",
                        method = %req.method(),
                        uri = %req.uri(),
                        version = ?req.version()
                    )
                })
                .on_request(DefaultOnRequest::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO))
                .on_failure(
                    |failure: ServerErrorsFailureClass,
                     _latency: std::time::Duration,
                     _span: &Span| {
                        tracing::error!(?failure, "request failed");
                    },
                ),
        )
        .with_state(state)
}

#[instrument(name = "handler.get_pokemon", skip(state, id_or_name), fields(pokemon.query = %id_or_name))]
async fn get_pokemon(
    State(state): State<AppState>,
    Path(id_or_name): Path<String>,
) -> Result<Json<Pokemon>, (axum::http::StatusCode, Json<ErrorMessage>)> {
    let client = state.pool.get().await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorMessage {
                error: e.to_string(),
            }),
        )
    })?;
    // Try numeric id
    if let Ok(id) = id_or_name.parse::<i32>() {
        if let Some(row) = client.query_opt("SELECT id, name, height, weight, base_experience, sprites, types, abilities, fetched_at FROM pokemon WHERE id = $1", &[&id]).await.map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorMessage { error: e.to_string() })))? {
            return Ok(Json(row_to_model(&row)));
        }
    } else if let Some(row) = client.query_opt("SELECT id, name, height, weight, base_experience, sprites, types, abilities, fetched_at FROM pokemon WHERE name = $1", &[&id_or_name]).await.map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorMessage { error: e.to_string() })))? {
        return Ok(Json(row_to_model(&row)));
    }

    // Remote fetch and upsert
    let fetch_span = info_span!("pokeapi.fetch", pokemon.query = %id_or_name);
    let _fetch_enter = fetch_span.enter();
    match state.pokeapi.fetch_pokemon(&id_or_name).await {
        Ok(mut pk) => {
            pk.fetched_at = Some(Utc::now());
            let sprites_json = serde_json::json!({"front_default": pk.sprites_front_default});
            let types_json = serde_json::Value::Array(
                pk.types
                    .iter()
                    .map(|t| serde_json::Value::String(t.clone()))
                    .collect(),
            );
            let abilities_json = serde_json::Value::Array(
                pk.abilities
                    .iter()
                    .map(|a| serde_json::Value::String(a.clone()))
                    .collect(),
            );
            let _ = client.execute(
                "INSERT INTO pokemon (id, name, height, weight, base_experience, sprites, types, abilities, fetched_at) VALUES ($1,$2,$3,$4,$5,$6::jsonb,$7::jsonb,$8::jsonb,$9) ON CONFLICT (id) DO UPDATE SET name=$2,height=$3,weight=$4,base_experience=$5,sprites=$6,types=$7,abilities=$8,fetched_at=$9",
                &[&pk.id, &pk.name, &pk.height, &pk.weight, &pk.base_experience, &sprites_json, &types_json, &abilities_json, &pk.fetched_at]
            ).await;
            Ok(Json(pk))
        }
        Err(e) => Err((
            axum::http::StatusCode::NOT_FOUND,
            Json(ErrorMessage {
                error: e.to_string(),
            }),
        )),
    }
}

#[instrument(
    name = "handler.list_pokemon",
    skip(state, headers, host, original),
    fields(page, page_size)
)]
async fn list_pokemon(
    State(state): State<AppState>,
    headers: HeaderMap,
    Host(host): Host,
    OriginalUri(original): OriginalUri,
) -> Result<Response, (StatusCode, Json<ErrorMessage>)> {
    let client = state.pool.get().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorMessage {
                error: e.to_string(),
            }),
        )
    })?;
    // Parse query parameters (GitHub style) page & per_page
    let mut page: u32 = 1;
    let mut page_size: u32 = 50;
    if let Some(q) = original.query() {
        for pair in q.split('&').filter(|s| !s.is_empty()) {
            let mut it = pair.splitn(2, '=');
            if let (Some(k), Some(v)) = (it.next(), it.next()) {
                match k {
                    "page" => {
                        if let Ok(p) = v.parse::<u32>() {
                            if p >= 1 {
                                page = p;
                            }
                        }
                    }
                    "per_page" => {
                        if let Ok(ps) = v.parse::<u32>() {
                            if ps >= 1 {
                                page_size = ps.min(200);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    // Header override fallback (legacy)
    if let Some(h) = headers.get("x-page") {
        if let Ok(s) = h.to_str() {
            if let Ok(p) = s.parse::<u32>() {
                if p >= 1 {
                    page = p;
                }
            }
        }
    }
    if let Some(h) = headers.get("x-page-size") {
        if let Ok(s) = h.to_str() {
            if let Ok(ps) = s.parse::<u32>() {
                if ps >= 1 {
                    page_size = ps.min(200);
                }
            }
        }
    }

    let offset: i64 = ((page - 1) as i64) * (page_size as i64);
    let limit: i64 = page_size as i64;
    tracing::Span::current().record("page", &tracing::field::display(page));
    tracing::Span::current().record("page_size", &tracing::field::display(page_size));
    let rows = client.query(
        "SELECT id, name, height, weight, base_experience, sprites, types, abilities, fetched_at FROM pokemon ORDER BY id LIMIT $1 OFFSET $2",
        &[&limit, &offset]
    ).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorMessage { error: e.to_string() })))?;

    let count_row = client
        .query_one("SELECT COUNT(*) FROM pokemon", &[])
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorMessage {
                    error: e.to_string(),
                }),
            )
        })?;
    let total: i64 = count_row.get(0);
    let last_page = if total == 0 {
        1
    } else {
        ((total - 1) / (page_size as i64) + 1) as u32
    };

    let pokemons: Vec<Pokemon> = rows.iter().map(row_to_model).collect();
    let mut response = Json(pokemons).into_response();
    response.headers_mut().insert(
        "X-Total-Count",
        HeaderValue::from_str(&total.to_string()).unwrap(),
    );
    response
        .headers_mut()
        .insert("X-Page", HeaderValue::from_str(&page.to_string()).unwrap());
    response.headers_mut().insert(
        "X-Page-Size",
        HeaderValue::from_str(&page_size.to_string()).unwrap(),
    );

    let base_path = original.path();
    let base_url = format!("http://{}{}", host, base_path);
    let mut links: Vec<String> = Vec::new();
    if page > 1 {
        links.push(format!(
            "<{}?page={}&per_page={}>; rel=\"prev\"",
            base_url,
            page - 1,
            page_size
        ));
    }
    if page < last_page {
        links.push(format!(
            "<{}?page={}&per_page={}>; rel=\"next\"",
            base_url,
            page + 1,
            page_size
        ));
    }
    links.push(format!(
        "<{}?page=1&per_page={}>; rel=\"first\"",
        base_url, page_size
    ));
    links.push(format!(
        "<{}?page={}&per_page={}>; rel=\"last\"",
        base_url, last_page, page_size
    ));
    if let Ok(hv) = HeaderValue::from_str(&links.join(", ")) {
        response.headers_mut().insert("Link", hv);
    }

    Ok(response)
}

fn row_to_model(row: &Row) -> Pokemon {
    let sprites_val: serde_json::Value = row.get("sprites");
    let types_val: serde_json::Value = row.get("types");
    let abilities_val: serde_json::Value = row.get("abilities");
    Pokemon {
        id: row.get("id"),
        name: row.get("name"),
        height: row.get("height"),
        weight: row.get("weight"),
        base_experience: row.get("base_experience"),
        sprites_front_default: sprites_val
            .get("front_default")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        types: types_val
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default(),
        abilities: abilities_val
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default(),
        fetched_at: row.get::<_, Option<chrono::DateTime<Utc>>>("fetched_at"),
    }
}
