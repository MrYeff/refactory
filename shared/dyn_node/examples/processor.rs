use std::result::Result;

use async_service::*;
use bevy::{ecs::system::RunSystemOnce, prelude::*};
use dyn_node::{QueryError, prelude::*};
use serde::Deserialize;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(AsyncServicePlugin)
        .add_plugins(DynNodePlugin)
        .init_asset::<Enemy>()
        .init_resource::<Enemies>()
        .add_systems(Update, print_enemies)
        .run();
}

#[derive(Deserialize, Asset, TypePath)]
struct Enemy {
    health: u32,
    damage: u32,
    speed: u32,
}

#[derive(Resource, Deref)]
struct Enemies(Vec<Handle<Enemy>>);

impl FromWorld for Enemies {
    fn from_world(world: &mut World) -> Self {
        let enemies = world.run_system_once(get_enemies).unwrap();
        Enemies(enemies)
    }
}

fn get_enemies(
    asset_server: Res<AssetServer>,
    async_service: Res<AsyncService>,
) -> Vec<Handle<Enemy>> {
    let handle = asset_server.load::<DynNode>("config.yml");
    ["/enemies/aligator", "/enemies/ork", "/enemies/goblin"]
        .iter()
        .map(|path| {
            asset_server.add_async({
                let async_service = async_service.clone();
                let asset_server = asset_server.clone();
                let h_dyn = handle.clone();
                async move {
                    asset_server.wait_for_asset(&h_dyn).await.unwrap();
                    async_service
                        .exec_sync(query_node_sync, (h_dyn, path.to_string()))
                        .await
                }
            })
        })
        .collect()
}

fn query_node_sync(
    In((h, path)): In<(Handle<DynNode>, String)>,
    nodes: Res<Assets<DynNode>>,
) -> Result<Enemy, QueryError> {
    nodes.get(&h).unwrap().query::<Enemy>(&path)
}

fn print_enemies(enemies: Res<Enemies>, assets: Res<Assets<Enemy>>) {
    enemies
        .iter()
        .flat_map(|h| assets.get(h))
        .for_each(|enemy| {
            println!(
                "Enemy: health={}, damage={}, speed={}",
                enemy.health, enemy.damage, enemy.speed
            );
        });
}
