use bevy::prelude::*;
use bevy_reflect::TypePath;

use yml_sub_asset::*;

#[derive(Asset, TypePath, serde::Deserialize, Debug)]
struct Enemy {
    hp: u32,
    name: String,
}

#[derive(Asset, TypePath, serde::Deserialize, Debug)]
struct ListAsset(String);

fn main() {
    App::new()
        // IMPORTANT: before DefaultPlugins
        .add_plugins(YamlRefPlugin)
        .add_plugins(DefaultPlugins)
        .add_plugins(YamlRefTypePlugin::<Enemy>::new("enemy"))
        .add_plugins(YamlRefTypePlugin::<ListAsset>::new("list"))
        .add_systems(Startup, setup)
        .run();
}

fn setup(asset_server: Res<AssetServer>) {
    let _goblin: Handle<Enemy> = asset_server.load("config.yml@enemies/goblin.enemy");
    let _troll: Handle<Enemy> = asset_server.load("config.yml@enemies/troll.enemy");
    let _b: Handle<ListAsset> = asset_server.load("config.yml@list_example/1.list");
}
