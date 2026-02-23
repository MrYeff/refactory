use bevy::prelude::*;
use config_asset::prelude::*;
use on_asset_loaded::prelude::*;
use serde::Deserialize;

#[derive(Deserialize, Asset, TypePath, Component)]
struct Enemy {
    health: u32,
    damage: u32,
    speed: u32,
}

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(OnAssetLoadedPlugin)
        .add_plugins(ConfigAssetLoaderPlugin)
        .init_asset::<Enemy>()
        .add_systems(Startup, setup)
        .run()
}

fn setup(asset_server: Res<AssetServer>, enemies: Res<Assets<Enemy>>, mut commands: Commands) {
    let h_config = asset_server.load("config.yml");

    ["/enemies/aligator", "/enemies/ork", "/enemies/goblin"]
        .iter()
        .for_each(|path| {
            let h_enemy = enemies.reserve_handle();
            commands.on_loaded_with(
                &h_config,
                (h_enemy.clone(), path.to_string()),
                on_loaded_query_extract::<Enemy>,
            );

            commands.on_loaded(h_enemy, |input: OnLoaded<Enemy>| {
                println!(
                    "Enemy loaded: health={}, damage={}, speed={}",
                    input.asset.health, input.asset.damage, input.asset.speed
                );
            });
        })
}
