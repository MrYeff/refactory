use std::{marker::PhantomData, process::exit};

use bevy::{
    ecs::system::SystemId, platform::collections::HashSet, prelude::*,
    render::render_resource::TextureFormat,
};

// Handle<Image> -> Handle<Image>
// Handle<AnyConfig> -> Handle<Enemy>
// Handle<AnyConfig> -> Handle<LVL>

// let _goblin: Handle<Enemy> = asset_server.load("config.yml@enemies/goblin.enemy");
// let _troll: Handle<Enemy> = asset_server.load("config.yml@enemies/troll.enemy");
// let _b: Handle<ListAsset> = asset_server.load("config.yml@list_example/1.list");

// CORE

#[derive(Component)]
struct ConvertRequest<S, T, C, P, M>
where
    S: Asset,
    T: Asset,
    // Pass handle; the system borrows the asset via `Res<Assets<S>>`.
    C: IntoSystem<In<(P, Handle<S>)>, T, M> + Copy,
    P: 'static,
{
    handle_in: Handle<S>,
    handle_out: Handle<T>,
    /// should never be none just here for later take!
    params: Option<P>,
    system: C, // TODO
    phantom: PhantomData<fn() -> M>,
}

impl<S, T, C, P, M> ConvertRequest<S, T, C, P, M>
where
    S: Asset,
    T: Asset,
    C: IntoSystem<In<(P, Handle<S>)>, T, M> + Copy + Send + Sync + 'static,
    P: Send + Sync + 'static,
    M: Send + Sync + 'static,
{
    fn new(handle_in: Handle<S>, handle_out: Handle<T>, params: P, system: C) -> Self {
        Self {
            handle_in,
            handle_out,
            params: Some(params),
            system,
            phantom: PhantomData,
        }
    }

    /// returns if should be run again
    fn execute(world: &mut World) -> bool {
        // Pass 1: immutable scan (safe to read Assets + components together)
        let (has_any, ready): (bool, Vec<Entity>) = {
            let mut q = world.query::<(Entity, &Self)>();
            let assets_in = world.resource::<Assets<S>>();

            let mut any = false;
            let mut ready = Vec::new();

            q.iter(world).for_each(|(e, req)| {
                any = true;

                // Only proceed once input is available AND we still have params to consume.
                if req.params.is_some() && assets_in.get(&req.handle_in).is_some() {
                    ready.push(e);
                }
            });

            (any, ready)
        };

        // If there are no requests of this concrete type, stop running this system.
        if !has_any {
            return false;
        }

        // Nothing ready yet, but there are pending requests -> keep running.
        if ready.is_empty() {
            return true;
        }

        // Pass 2: mutable per-entity work (no query iter_mut, so we can touch world freely)
        ready.into_iter().for_each(|e| {
            // Pull what we need out of the component (consuming params once)
            let (out_id, params, handle_in, system) = {
                let mut ent = world.entity_mut(e);
                let mut req = ent.get_mut::<Self>().unwrap();

                (
                    req.handle_out.id(),
                    req.params.take().unwrap(),
                    req.handle_in.clone(),
                    req.system,
                )
            };

            // Run converter system (borrows input via Assets<S> inside the system)
            let output: T = world
                .run_system_cached_with(system, (params, handle_in))
                .unwrap();

            world
                .resource_mut::<Assets<T>>()
                .insert(out_id, output)
                .unwrap();
            world.despawn(e); // done
        });

        true
    }
}

impl<S, T, C, P, M> Command for ConvertRequest<S, T, C, P, M>
where
    S: Asset,
    T: Asset,
    C: IntoSystem<In<(P, Handle<S>)>, T, M> + Copy + Send + Sync + 'static,
    P: Send + Sync + 'static,
    M: Send + Sync + 'static,
{
    fn apply(self, world: &mut World) -> () {
        world.commands().spawn(self);
        let sysid = world.register_system_cached(Self::execute);
        world.resource_mut::<ConverterSystems>().0.insert(sysid);
    }
}

#[derive(Resource, Default)]
struct ConverterSystems(HashSet<SystemId<(), bool>>);

fn run_converter_systems(world: &mut World) {
    world.resource_scope(|world, mut converters: Mut<ConverterSystems>| {
        converters
            .0
            .retain(|sysid| world.run_system(*sysid).unwrap());
    });
}

// CONTENT

fn convert_texture_format(
    In((fmt, handle)): In<(TextureFormat, Handle<Image>)>,
    assets: Res<Assets<Image>>,
) -> Image {
    assets.get(&handle).unwrap().convert(fmt).unwrap()
}

// EXAMPLE

fn foo1(
    mut handle_in: Local<Option<Handle<Image>>>,
    mut handle_out: Local<Option<Handle<Image>>>,
    asset_server: Res<AssetServer>,
    assets: Res<Assets<Image>>,
    mut commands: Commands,
) {
    let handle_in = handle_in.get_or_insert_with(|| asset_server.load("image.png"));
    let handle_out = handle_out.get_or_insert_with(|| {
        let h = assets.reserve_handle();

        commands.queue(ConvertRequest::new(
            handle_in.clone(),
            h.clone(),
            TextureFormat::R8Unorm,
            convert_texture_format,
        ));
        h
    });

    if let Some(_) = assets.get(handle_out) {
        println!("conversion complete!");
        exit(0); // done    
    }
}

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<ConverterSystems>()
        .add_systems(Update, foo1)
        .add_systems(PostUpdate, run_converter_systems)
        .run()
}
