use crate::models::Pokemon;
use crate::pokeapi::PokeApi;
use async_graphql::{Context, EmptyMutation, EmptySubscription, Object, Schema};

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn pokemon(&self, ctx: &Context<'_>, name: String) -> async_graphql::Result<Pokemon> {
        let pokeapi = ctx.data::<PokeApi>()?;
        let pokemon = pokeapi.fetch_pokemon(&name).await?;
        Ok(pokemon)
    }
}

pub type PokemonSchema = Schema<QueryRoot, EmptyMutation, EmptySubscription>;

pub fn build_schema(pokeapi: PokeApi) -> PokemonSchema {
    Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
        .data(pokeapi)
        .finish()
}
