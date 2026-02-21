use std::marker::PhantomData;

use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::{Asset, Assets, Handle};
use bevy_ecs::{
    component::Component,
    prelude::{Commands, Entity, In, IntoSystem, Mut, Res, Resource, World},
    system::{Command, SystemId, SystemParam},
};
use bevy_platform::collections::HashSet;

pub mod prelude {
    pub use super::{AssetTransformer, AssetTransformerPlugin};
}

/// Plugin that provides the [`AssetTransformer`] system param, which can be used to create new assets from existing ones via custom systems.
/// Systems must have the signature |In<(Handle<AssetIn>, Params)>, ...| -> AssetOut. ...can be any valid system params.
/// It can be assumed that the handle already has loaded.
///
/// Example Transformer System:
/// ```
/// fn convert_texture_format(
///   In((handle, fmt)): In<(Handle<Image>, TextureFormat)>,
///   assets: Res<Assets<Image>>,
/// ) -> Image {
///  assets.get(&handle).expect("handle has loaded").convert(fmt).unwrap()
/// }
/// ```
pub struct AssetTransformerPlugin;

impl Plugin for AssetTransformerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TransformerSystems>()
            .add_systems(PostUpdate, run_transformer_systems);
    }
}

/// System Param used to create new assets from existing ones via custom systems. Reference [`AssetTransformerPlugin`] for more information.
#[derive(SystemParam)]
pub struct AssetTransformer<'w, 's, T: Asset> {
    commands: Commands<'w, 's>,
    assets: Res<'w, Assets<T>>,
}

impl<'w, 's, T: Asset> AssetTransformer<'w, 's, T> {
    pub fn transform_handle_with_params<S, C, P, M>(
        &mut self,
        input: Handle<S>,
        transformer: C,
        params: P,
    ) -> Handle<T>
    where
        S: Asset,
        C: IntoSystem<In<(Handle<S>, P)>, T, M> + Copy + Send + Sync + 'static,
        P: Send + Sync + 'static,
        T: Asset + Send + Sync + 'static,
        M: Send + Sync + 'static,
    {
        let output = self.assets.reserve_handle();

        self.commands.queue(TransformRequest::new(
            input,
            output.clone(),
            params,
            transformer,
        ));

        output
    }

    pub fn transform_handle<S, C, M>(&mut self, input: Handle<S>, transformer: C) -> Handle<T>
    where
        S: Asset,
        C: IntoSystem<In<(Handle<S>, ())>, T, M> + Copy + Send + Sync + 'static,
        T: Asset + Send + Sync + 'static,
        M: Send + Sync + 'static,
    {
        self.transform_handle_with_params(input, transformer, ())
    }
}

// INTERNAL
#[derive(Component)]
struct TransformRequest<S, T, C, P, M>
where
    S: Asset,
    T: Asset,
    // Pass handle; the system borrows the asset via `Res<Assets<S>>`.
    C: IntoSystem<In<(Handle<S>, P)>, T, M> + Copy,
    P: 'static,
{
    handle_in: Handle<S>,
    handle_out: Handle<T>,
    /// should never be none just here for later take!
    params: Option<P>,
    system: C, // TODO
    phantom: PhantomData<fn() -> M>,
}

impl<S, T, C, P, M> TransformRequest<S, T, C, P, M>
where
    S: Asset,
    T: Asset,
    C: IntoSystem<In<(Handle<S>, P)>, T, M> + Copy + Send + Sync + 'static,
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

            // Run transformer system (borrows input via Assets<S> inside the system)
            let output: T = world
                .run_system_cached_with(system, (handle_in, params))
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

impl<S, T, C, P, M> Command for TransformRequest<S, T, C, P, M>
where
    S: Asset,
    T: Asset,
    C: IntoSystem<In<(Handle<S>, P)>, T, M> + Copy + Send + Sync + 'static,
    P: Send + Sync + 'static,
    M: Send + Sync + 'static,
{
    fn apply(self, world: &mut World) -> () {
        world.commands().spawn(self);
        let sysid = world.register_system_cached(Self::execute);
        world.resource_mut::<TransformerSystems>().0.insert(sysid);
    }
}

#[derive(Resource, Default)]
struct TransformerSystems(HashSet<SystemId<(), bool>>);

fn run_transformer_systems(world: &mut World) {
    world.resource_scope(|world, mut transformers: Mut<TransformerSystems>| {
        transformers
            .0
            .retain(|sysid| world.run_system(*sysid).unwrap());
    });
}
