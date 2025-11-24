use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Pokemon {
    pub id: i32,
    pub name: String,
    pub height: Option<i32>,
    pub weight: Option<i32>,
    pub base_experience: Option<i32>,
    pub sprites_front_default: Option<String>,
    pub types: Vec<String>,
    pub abilities: Vec<String>,
    pub fetched_at: Option<DateTime<Utc>>,
}

// Shape from remote API (subset)
#[derive(Debug, Deserialize)]
pub struct RemotePokemon {
    pub id: i32,
    pub name: String,
    pub height: Option<i32>,
    pub weight: Option<i32>,
    pub base_experience: Option<i32>,
    pub sprites: RemoteSprites,
    pub types: Vec<RemoteTypeWrapper>,
    pub abilities: Vec<RemoteAbilityWrapper>,
}

#[derive(Debug, Deserialize)]
pub struct RemoteSprites {
    pub front_default: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RemoteTypeWrapper { pub r#type: RemoteType }
#[derive(Debug, Deserialize)]
pub struct RemoteType { pub name: String }
#[derive(Debug, Deserialize)]
pub struct RemoteAbilityWrapper { pub ability: RemoteAbility }
#[derive(Debug, Deserialize)]
pub struct RemoteAbility { pub name: String }

impl From<RemotePokemon> for Pokemon {
    fn from(r: RemotePokemon) -> Self {
        Pokemon {
            id: r.id,
            name: r.name,
            height: r.height,
            weight: r.weight,
            base_experience: r.base_experience,
            sprites_front_default: r.sprites.front_default,
            types: r.types.into_iter().map(|t| t.r#type.name).collect(),
            abilities: r.abilities.into_iter().map(|a| a.ability.name).collect(),
            fetched_at: None,
        }
    }
}
