// Validates: EntityServer::to_managed can attach management + intent to an existing entity, and cleanup works.

use bevy::{ecs::system::RunSystemOnce, prelude::*};
use entity_handle::prelude::*;

#[derive(Default)]
struct IntentA;

#[derive(Component)]
struct Existing;

#[test]
fn to_managed_wraps_existing_entity_and_cleans_up() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(EntityHandlePlugin);

    app.register_intent::<IntentA>();

    // Spawn an entity normally (not via EntityServer)
    let entity = app.world_mut().spawn(Existing).id();
    assert!(app.world().get_entity(entity).is_ok());

    // Convert it to managed + request intent
    let handle = app
        .world_mut()
        .run_system_once(move |mut server: EntityServer| server.to_managed::<IntentA>(entity))
        .expect("system failed");

    assert_eq!(*handle, entity);
    assert!(app.world().get::<IntentMarker<IntentA>>(entity).is_some());

    // Drop the only handle => should despawn the existing entity too
    drop(handle);
    app.update();

    assert!(app.world().get_entity(entity).is_err());
}
