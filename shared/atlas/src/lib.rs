mod common_sequences;

use std::sync::Arc;

use bevy::scene::SceneSpawnError;
use bevy::{
    ecs::entity::EntityHashMap,
    platform::collections::HashMap,
    prelude::*,
    scene::{DynamicEntity, DynamicScene},
};

use entity_handle::prelude::*;

/*
Pipeline (direct apply, no spawn):

1) for all AtlasEntry entities missing AtlasHandle:
      insert AtlasHandle(EntityAssetHandle<AtlasId>)

2) for all Atlas roots missing AtlasSceneHandle:
      insert AtlasSceneHandle(Handle<DynamicScene>) loaded from AtlasId.0

3) exclusive system:
      - wait until Assets<DynamicScene> contains the handle
      - build lookup: (atlas_path, entry_id) -> final_entity
      - build entity_map: (scene_entity -> final_entity) for every scene entity
      - if ANY scene entity can't map, do NOT apply (to avoid spawning)
      - call scene.write_to_world(world, &mut map)
      - mark AtlasSceneApplied + remove AtlasSceneHandle
*/

/// Instead of `DynamicSceneRoot`, store the handle and apply it manually.
#[derive(Debug, Clone, Component)]
struct AtlasSceneHandle(Handle<DynamicScene>);

/// Marker so we only apply once.
#[derive(Debug, Clone, Component)]
struct AtlasSceneApplied;

fn insert_attlas_handles(
    mut commands: Commands,
    mut entity_asset_server: EntityAssetServer<AtlasId>,
    query: Query<(Entity, &AtlasPath), Without<AtlasHandle>>,
) {
    for (e, path) in &query {
        let handle = entity_asset_server.get_asset(AtlasId(path.0.clone()));
        commands.entity(e).insert(AtlasHandle(handle));
    }
}

fn insert_attlas_scenes(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    query: Query<(Entity, &AtlasId), Without<AtlasSceneHandle>>,
) {
    for (e, atlas) in &query {
        // AtlasId.0 is the path to your .scn.ron
        let scene_handle: Handle<DynamicScene> = asset_server.load(atlas.0.to_string());
        commands.entity(e).insert(AtlasSceneHandle(scene_handle));
    }
}

fn apply_atlas_scene_direct_no_spawn(world: &mut World) {
    // Build lookup for final entities (atlas_path, entry_id) -> Entity
    let mut final_lookup: HashMap<(Arc<str>, Arc<str>), Entity> = HashMap::default();
    {
        let mut q_final = world.query::<(Entity, &AtlasPath, &AtlasEntryId)>();
        for (e, path, id) in q_final.iter(world) {
            final_lookup.insert((path.0.clone(), id.0.clone()), e);
        }
    }

    // Snapshot roots we want to process (so we don't hold a query borrow while applying)
    let roots: Vec<(Entity, Arc<str>, Handle<DynamicScene>)> = {
        let mut out = Vec::new();
        let mut q_roots = world.query::<(Entity, &AtlasId, &AtlasSceneHandle)>();
        for (e, atlas_id, scene_handle) in q_roots.iter(world) {
            if world.entity(e).contains::<AtlasSceneApplied>() {
                continue;
            }
            out.push((e, atlas_id.0.clone(), scene_handle.0.clone()));
        }
        out
    };

    // Now do the apply with split borrows
    world.resource_scope(|world, scenes: Mut<Assets<DynamicScene>>| {
        for (root_e, atlas_path, scene_handle) in roots {
            let Some(scene) = scenes.get(&scene_handle) else {
                // not loaded yet
                continue;
            };

            // Build entity map. To guarantee "no spawn", EVERY scene entity must map.
            let mut map: EntityHashMap<Entity> = EntityHashMap::default();

            for dyn_ent in scene.entities.iter() {
                let Some(entry_id) = find_component::<AtlasEntryId>(dyn_ent) else {
                    warn!(
                        "Scene `{}` has an entity without AtlasEntryId; skipping to avoid spawning.",
                        atlas_path
                    );
                    map.clear();
                    break;
                };

                let key = (atlas_path.clone(), entry_id.0.clone());
                let Some(&target) = final_lookup.get(&key) else {
                    warn!(
                        "Scene `{}` has entry `{}` with no matching final entity; skipping to avoid spawning.",
                        atlas_path,
                        entry_id.0
                    );
                    map.clear();
                    break;
                };

                map.insert(dyn_ent.entity, target);
            }

            if map.is_empty() {
                continue;
            }

            // Apply directly. Because every scene entity is mapped, this won't spawn extras.
            if let Err(err) = scene.write_to_world(world, &mut map) {
                warn!("Failed to apply scene `{}`: {:?}", atlas_path, err);
                continue;
            }

            // Mark applied and drop handle
            let mut em = world.entity_mut(root_e);
            em.insert(AtlasSceneApplied);
            em.remove::<AtlasSceneHandle>();
        }
    });
}

use bevy::reflect::{PartialReflect, Reflect};

fn find_component<T>(dyn_entity: &DynamicEntity) -> Option<T>
where
    T: Reflect + Clone + 'static,
{
    for c in &dyn_entity.components {
        // c: &Box<dyn PartialReflect>
        let Some(r) = c.try_as_reflect() else {
            continue;
        };
        if let Some(v) = r.downcast_ref::<T>() {
            return Some(v.clone());
        }
    }
    None
}

// --------------------- Your types ---------------------

#[derive(Debug, Clone, Component, Hash, PartialEq, Eq)]
struct AtlasId(Arc<str>);

#[derive(Debug, Clone, Component, Hash, PartialEq, Eq)]
struct AtlasPath(Arc<str>);

#[derive(Debug, Clone, Component)]
struct AtlasHandle(EntityAssetHandle<AtlasId>);

/// This must exist on:
/// - scene entities in `.scn.ron`
/// - final entities in your world
#[derive(Debug, Clone, Component, Reflect, Hash, PartialEq, Eq)]
#[reflect(Component)]
struct AtlasEntryId(Arc<str>);

#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
struct AtlasEntryDefinition {
    tile_size: UVec2,
    size: UVec2,
    offset: UVec2,
}

#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
struct FrameSequence {
    stride: UVec2,
    count: u32,
}

impl FrameSequence {
    fn get_index(&self, frame: u32) -> UVec2 {
        let frame = frame % self.count;
        UVec2::new(self.stride.x * frame, self.stride.y * frame)
    }
}

// --------------------- Plugin wiring ---------------------

pub struct AtlasSceneDirectApplyPlugin;

impl Plugin for AtlasSceneDirectApplyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (insert_attlas_handles, insert_attlas_scenes).chain(),
        );

        // Exclusive system to call `DynamicScene::write_to_world`
        app.add_systems(Update, apply_atlas_scene_direct_no_spawn);
    }
}
