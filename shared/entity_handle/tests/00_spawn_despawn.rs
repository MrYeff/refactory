// Validates: entity is despawned when the last handle is dropped.

use bevy::{ecs::system::RunSystemOnce, prelude::*};
use entity_handle::prelude::*;

#[derive(Component, Debug)]
struct TestComp;

#[derive(Default)]
struct IntentA;

#[test]
fn spawn_despawns_when_last_handle_dropped() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(EntityHandlePlugin);

    // register intent we will use
    app.register_intent::<IntentA>();

    let (entity, handle) = app
        .world_mut()
        .run_system_once(|mut server: EntityServer| {
            let h = server.spawn::<IntentA>(TestComp);
            (*h, h)
        })
        .expect("system failed");

    assert!(app.world().get_entity(entity).is_ok());
    assert!(app.world().get::<TestComp>(entity).is_some());
    assert!(app.world().get::<IntentMarker<IntentA>>(entity).is_some());

    // dropping last handle queues entity drop
    drop(handle);

    // your cleanup runs in PostUpdate, but app.update() runs the whole frame schedules
    app.update();

    assert!(app.world().get_entity(entity).is_err());
}
