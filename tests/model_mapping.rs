use rest_api::models::{RemotePokemon, RemoteSprites, RemoteTypeWrapper, RemoteType, RemoteAbilityWrapper, RemoteAbility, Pokemon};

#[test]
fn remote_to_local_conversion() {
    let remote = RemotePokemon {
        id: 10,
        name: "caterpie".to_string(),
        height: Some(3),
        weight: Some(29),
        base_experience: Some(39),
        sprites: RemoteSprites { front_default: Some("url".into()) },
        types: vec![RemoteTypeWrapper { r#type: RemoteType { name: "bug".into() } }],
        abilities: vec![RemoteAbilityWrapper { ability: RemoteAbility { name: "shield-dust".into() } }],
    };
    let local: Pokemon = remote.into();
    assert_eq!(local.id, 10);
    assert_eq!(local.name, "caterpie");
    assert_eq!(local.height, Some(3));
    assert_eq!(local.types, vec!["bug".to_string()]);
    assert_eq!(local.abilities, vec!["shield-dust".to_string()]);
    assert_eq!(local.sprites_front_default, Some("url".to_string()));
}
