use bevy::{ecs::system::RunSystemOnce, prelude::*};
use entity_handle::prelude::*;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
struct MyId(u32);

#[derive(Default)]
struct IntentA;
#[derive(Default)]
struct IntentB;

#[derive(Component, Debug, PartialEq, Eq)]
struct Value(i32);

#[test]
fn singletons_are_stable_per_id_and_query_works() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(EntityHandlePlugin);

    app.register_entity_asset_id::<MyId>();
    app.register_intent::<IntentA>();
    app.register_intent::<IntentB>();

    // IMPORTANT: keep handles alive so they do NOT drop before the first update flush.
    let (h1a, h1b, h2) = app
        .world_mut()
        .run_system_once(|mut server: EntityAssetServer<MyId>| {
            let h1a = server.get_asset::<IntentA>(MyId(1));
            let h1b = server.get_asset::<IntentB>(MyId(1));
            let h2 = server.get_asset::<IntentA>(MyId(2));
            (h1a, h1b, h2)
        })
        .expect("system failed");

    let e1a = *h1a;
    let e1b = *h1b;
    let e2 = *h2;

    // Flush the spawn/insert commands while handles are still alive.
    app.update();

    assert_eq!(e1a, e1b);
    assert_ne!(e1a, e2);

    // Now entities definitely exist in the World; safe to mutate directly.
    app.world_mut().entity_mut(e1a).insert(Value(10));
    app.world_mut().entity_mut(e2).insert(Value(20));

    // Validate iter_mut can mutate
    app.world_mut()
        .run_system_once(|mut q: EntityAssetQuery<MyId, &mut Value>| {
            for mut v in q.iter_mut() {
                v.0 += 1;
            }
        })
        .expect("system failed");

    assert_eq!(app.world().get::<Value>(e1a).unwrap().0, 11);
    assert_eq!(app.world().get::<Value>(e2).unwrap().0, 21);

    // Validate iter count
    let count = app
        .world_mut()
        .run_system_once(|q: EntityAssetQuery<MyId, &Value>| q.iter().count())
        .expect("system failed");
    assert_eq!(count, 2);

    // Optional: show cleanup works too
    drop(h1a);
    drop(h1b);
    drop(h2);

    app.update();
    assert!(app.world().get_entity(e1a).is_err());
    assert!(app.world().get_entity(e2).is_err());
}
