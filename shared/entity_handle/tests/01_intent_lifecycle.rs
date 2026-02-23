// Validates: intent markers are removed independently (dropping handles for intent A removes IntentMarker<A> while entity stays alive due to another handle with intent B).

use bevy::{ecs::system::RunSystemOnce, prelude::*};
use entity_handle::prelude::*;

#[derive(Component, Debug)]
struct TestComp;

#[derive(Default)]
struct IntentA;
#[derive(Default)]
struct IntentB;

#[test]
fn dropping_last_intent_handle_removes_only_that_marker() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(EntityHandlePlugin);

    app.register_intent::<IntentA>();
    app.register_intent::<IntentB>();

    let (entity, ha, hb) = app
        .world_mut()
        .run_system_once(|mut server: EntityServer| {
            // Spawn with intent A
            let ha = server.spawn::<IntentA>(TestComp);
            let entity = *ha;

            // Also request intent B for the SAME entity
            let hb = server.to_managed::<IntentB>(entity);

            (entity, ha, hb)
        })
        .expect("system failed");

    assert!(app.world().get_entity(entity).is_ok());
    assert!(app.world().get::<IntentMarker<IntentA>>(entity).is_some());
    assert!(app.world().get::<IntentMarker<IntentB>>(entity).is_some());

    // Drop only intent A handle: should remove marker A but keep entity + marker B
    drop(ha);
    app.update();

    assert!(app.world().get_entity(entity).is_ok());
    assert!(app.world().get::<IntentMarker<IntentA>>(entity).is_none());
    assert!(app.world().get::<IntentMarker<IntentB>>(entity).is_some());

    // Drop intent B handle too: now entity should despawn (no handles left)
    drop(hb);
    app.update();

    assert!(app.world().get_entity(entity).is_err());
}
