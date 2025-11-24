use deadpool_postgres::{Manager, Pool};
use std::str::FromStr;
use tokio_postgres::NoTls;
use tracing::info;

pub type PgPool = Pool;

pub async fn init_pool(database_url: &str) -> anyhow::Result<PgPool> {
    let pg_cfg = tokio_postgres::Config::from_str(database_url)?;
    let mgr = Manager::new(pg_cfg, NoTls);
    let pool = Pool::builder(mgr).max_size(5).build().unwrap();
    info!("connected to postgres");
    Ok(pool)
}

pub async fn migrate(pool: &PgPool) -> anyhow::Result<()> {
    let client = pool.get().await?;
    let stmt = r#"CREATE TABLE IF NOT EXISTS pokemon (
        id INT PRIMARY KEY,
        name TEXT NOT NULL UNIQUE,
        height INT,
        weight INT,
        base_experience INT,
        sprites JSONB,
        types JSONB,
        abilities JSONB,
        fetched_at TIMESTAMPTZ DEFAULT now()
    )"#;
    client.batch_execute(stmt).await?;
    // Indices for common lookups & filtering
    client
        .batch_execute(
            r#"CREATE INDEX IF NOT EXISTS idx_pokemon_name ON pokemon(name);
           CREATE INDEX IF NOT EXISTS idx_pokemon_base_experience ON pokemon(base_experience);
           CREATE INDEX IF NOT EXISTS idx_pokemon_types_gin ON pokemon USING GIN (types);
           CREATE INDEX IF NOT EXISTS idx_pokemon_abilities_gin ON pokemon USING GIN (abilities);"#,
        )
        .await?;

    // seed data
    let seed_data = vec![
        (
            1,
            "bulbasaur",
            7,
            69,
            64,
            serde_json::json!({"front_default":"https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/1.png"}),
            serde_json::json!(["grass", "poison"]),
            serde_json::json!(["overgrow", "chlorophyll"]),
        ),
        (
            4,
            "charmander",
            6,
            85,
            62,
            serde_json::json!({"front_default":"https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/4.png"}),
            serde_json::json!(["fire"]),
            serde_json::json!(["blaze", "solar-power"]),
        ),
        (
            7,
            "squirtle",
            5,
            90,
            63,
            serde_json::json!({"front_default":"https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/7.png"}),
            serde_json::json!(["water"]),
            serde_json::json!(["torrent", "rain-dish"]),
        ),
        (
            25,
            "pikachu",
            4,
            60,
            112,
            serde_json::json!({"front_default":"https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/25.png"}),
            serde_json::json!(["electric"]),
            serde_json::json!(["static", "lightning-rod"]),
        ),
        (
            133,
            "eevee",
            3,
            65,
            65,
            serde_json::json!({"front_default":"https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/133.png"}),
            serde_json::json!(["normal"]),
            serde_json::json!(["run-away", "adaptability", "anticipation"]),
        ),
    ];

    for (id, name, height, weight, base_exp, sprites, types, abilities) in seed_data {
        client.execute(
            "INSERT INTO pokemon (id, name, height, weight, base_experience, sprites, types, abilities) VALUES ($1,$2,$3,$4,$5,$6::jsonb,$7::jsonb,$8::jsonb) ON CONFLICT (id) DO NOTHING",
            &[&id, &name, &height, &weight, &base_exp, &sprites, &types, &abilities]
        ).await?;
    }

    info!("migration and seed data complete");
    Ok(())
}
