#![feature(try_trait_v2)]
use std::{
    ops::{DerefMut, Try},
    process::exit,
};

use bevy::{
    ecs::system::{CachedSystemId, SystemId, SystemParam},
    platform::collections::HashSet,
    prelude::*,
    render::render_resource::TextureFormat,
};
use itertools::structs;

// Handle<Image> -> Handle<Image>
// Handle<AnyConfig> -> Handle<Enemy>
// Handle<AnyConfig> -> Handle<LVL>

// let _goblin: Handle<Enemy> = asset_server.load("config.yml@enemies/goblin.enemy");
// let _troll: Handle<Enemy> = asset_server.load("config.yml@enemies/troll.enemy");
// let _b: Handle<ListAsset> = asset_server.load("config.yml@list_example/1.list");

// CORE

trait AssetConverter<S: Asset, T: Asset, P = ()> {
    fn convert(params: P, input: &S) -> T;
}

#[derive(Component)]
struct ConvertRequest<S, T, C, P = ()>
where
    S: Asset,
    T: Asset,
    C: AssetConverter<S, T, P>, // TODO replace with system
{
    handle_in: Handle<S>,
    handle_out: Handle<T>,
    /// should never be none just here for later take!
    params: Option<P>,
    phantom: std::marker::PhantomData<fn() -> C>,
}

impl<S, T, C, P> ConvertRequest<S, T, C, P>
where
    S: Asset,
    T: Asset,
    C: AssetConverter<S, T, P> + Send + Sync + 'static,
    P: Send + Sync + 'static,
{
    fn new(handle_in: Handle<S>, handle_out: Handle<T>, params: P) -> Self {
        Self {
            handle_in,
            handle_out,
            params: Some(params),
            phantom: std::marker::PhantomData,
        }
    }

    /// returns if should be run again
    fn execute(
        mut entries: Query<(Entity, &mut Self)>,
        mut assets: ParamSet<(Res<Assets<S>>, ResMut<Assets<T>>)>,
        mut commands: Commands,
    ) -> bool {
        if entries.is_empty() {
            return false;
        }

        entries.iter_mut().for_each(|(e, mut req)| {
            if let Some(input) = assets.p0().get(&req.handle_in) {
                let output = C::convert(req.params.take().unwrap(), input);
                assets.p1().insert(req.handle_out.id(), output).unwrap();
                commands.entity(e).despawn(); // done
            }
        });

        true
    }
}

impl<S, T, C, P> Command for ConvertRequest<S, T, C, P>
where
    S: Asset,
    T: Asset,
    C: AssetConverter<S, T, P> + Send + Sync + 'static,
    P: Send + Sync + 'static,
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

struct TextureFormatConverter(());

impl AssetConverter<Image, Image, TextureFormat> for TextureFormatConverter {
    fn convert(fmt: TextureFormat, input: &Image) -> Image {
        input.convert(fmt).unwrap()
    }
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

        commands.queue(ConvertRequest::<_, _, TextureFormatConverter, _>::new(
            handle_in.clone(),
            h.clone(),
            TextureFormat::R8Unorm,
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
