use std::time::Duration;
use tokio::time::sleep;
use rest_api::db::{init_pool, migrate};
use rest_api::routes::{create_router, AppState};
use rest_api::pokeapi::PokeApi;
use axum::Router;
use tokio::net::TcpListener;
use reqwest::Client;

#[tokio::test]
async fn pagination_headers() {
    dotenvy::dotenv().ok();
    let db_url = match std::env::var("DATABASE_URL") { Ok(v) => v, Err(_) => { eprintln!("Skipping test: DATABASE_URL not set"); return; } };
    let pool = init_pool(&db_url).await.expect("pool");
    migrate(&pool).await.expect("migrate");
    // Seed more rows if needed
    let client_conn = pool.get().await.expect("client");
    for i in 1000..1050 { // ensure enough rows for page 2
        let name = format!("name{i}");
        let sprites = serde_json::json!({});
        let types = serde_json::json!([]);
        let abilities = serde_json::json!([]);
        let _ = client_conn.execute(
            "INSERT INTO pokemon (id, name, height, weight, base_experience, sprites, types, abilities) VALUES ($1,$2,$3,$4,$5,$6::jsonb,$7::jsonb,$8::jsonb) ON CONFLICT (id) DO NOTHING",
            &[&i, &name, &1, &1, &1, &sprites, &types, &abilities]
        ).await;
    }

    let state = AppState { pool: pool.clone(), pokeapi: PokeApi::new() };
    let app: Router = create_router(state);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });
    // Give server a moment
    sleep(Duration::from_millis(50)).await;

    let url = format!("http://{}/pokemon?page=2&per_page=10", addr);
    let resp = Client::new().get(url)
        .send().await.unwrap();
    assert!(resp.status().is_success());
    let link_header_val = resp.headers().get("Link").expect("Link header").to_str().unwrap().to_string();
    let data: serde_json::Value = resp.json().await.unwrap();
    assert!(data.is_array());
    assert_eq!(data.as_array().unwrap().len(), 10);
    assert!(link_header_val.contains("rel=\"next\""));
    assert!(link_header_val.contains("rel=\"prev\""));
    assert!(link_header_val.contains("rel=\"first\""));
    assert!(link_header_val.contains("rel=\"last\""));
}
