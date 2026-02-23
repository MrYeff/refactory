// Validates “Bevy-like” behavior:
// - Drop-last queues an event
// - reacquire before app.update()
// - cleanup should not remove marker or despawn entity.
use bevy::{ecs::system::RunSystemOnce, prelude::*};
use entity_handle::prelude::*;

#[derive(Default)]
struct IntentA;
#[derive(Default)]
struct IntentB;

#[test]
fn reacquire_before_update_prevents_cleanup() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(EntityHandlePlugin);

    app.register_intent::<IntentA>();
    app.register_intent::<IntentB>();

    // create entity with intent A and also intent B so entity stays alive
    let (entity, ha, hb) = app
        .world_mut()
        .run_system_once(|mut server: EntityServer| {
            let ha = server.spawn::<IntentA>(());
            let hb = server.to_managed::<IntentB>(*ha);
            (*ha, ha, hb)
        })
        .expect("system failed");

    assert!(app.world().get::<IntentMarker<IntentA>>(entity).is_some());
    assert!(app.world().get::<IntentMarker<IntentB>>(entity).is_some());

    // Drop last A-handle, but do NOT update yet
    drop(ha);

    // Reacquire intent A BEFORE update
    let ha2 = app
        .world_mut()
        .run_system_once(move |mut server: EntityServer| server.to_managed::<IntentA>(entity))
        .expect("system failed");

    // Now cleanup runs; should NOT remove IntentA marker
    app.update();

    assert!(app.world().get_entity(entity).is_ok());
    assert!(app.world().get::<IntentMarker<IntentA>>(entity).is_some());
    assert!(app.world().get::<IntentMarker<IntentB>>(entity).is_some());

    // Drop everything; entity should despawn
    drop(ha2);
    drop(hb);
    app.update();

    assert!(app.world().get_entity(entity).is_err());
}
