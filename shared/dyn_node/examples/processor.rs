use bevy::prelude::*;
use dyn_node::prelude::*;
use on_asset_loaded::prelude::*;
use serde::Deserialize;

#[derive(Deserialize, Asset, TypePath, Component)]
struct Enemy {
    health: u32,
    damage: u32,
    speed: u32,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(AssetObserverPlugin)
        .add_plugins(DynNodePlugin)
        .init_dyn_asset::<Enemy>()
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    asset_server: Res<AssetServer>,
    mut dyn_resolver: DynNodeResolver,
    mut commands: Commands,
) {
    let h_dyn = asset_server.load("config.yml");

    ["/enemies/aligator", "/enemies/ork", "/enemies/goblin"]
        .iter()
        .for_each(|path| {
            let h_enemy = dyn_resolver.resolve(h_dyn.clone(), path.to_string());

            commands.on_loaded(h_enemy, |input: OnLoaded<Enemy>| {
                println!(
                    "Enemy loaded: health={}, damage={}, speed={}",
                    input.asset.health, input.asset.damage, input.asset.speed
                );
            });
        })
}
