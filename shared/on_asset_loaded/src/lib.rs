pub mod components;

use bevy::{
    ecs::system::{FromInput, SystemId},
    platform::collections::HashSet,
    prelude::*,
};
use std::{borrow::Borrow, marker::PhantomData};

pub mod prelude {
    pub use super::{AppExt as _, AssetObserverPlugin, OnLoaded};
    pub use crate::components::{AddWhenLoaded, AddWhenLoadedBundle, AssetHandle, AssetLoaded};
}

/// Actual parameter type received by user processor systems.
///
/// Example:
/// ```
/// fn convert_texture_format(input: ProcessorInput<'_, Image, TextureFormat>) -> Image {
///     input.asset.convert(input.params).unwrap()
/// }
/// ```
pub struct OnLoaded<'a, T, P = ()> {
    pub asset: &'a T,
    pub params: P,
}

/// Marker type used in `IntoSystem<ProcessorInputMarker<S, P>, TOut, M>`
pub struct OnLoadedMarker<T, P>(PhantomData<fn() -> (T, P)>);

/// Make the actual param type itself a valid `SystemInput` (passthrough).
impl<'a, T: 'static, P: 'static> SystemInput for OnLoaded<'a, T, P> {
    type Param<'i> = OnLoaded<'i, T, P>;
    type Inner<'i> = OnLoaded<'i, T, P>;

    fn wrap(this: Self::Inner<'_>) -> Self::Param<'_> {
        this
    }
}

/// Allow functions taking `ProcessorInput<'_, T, P>` to be used where the system input is
/// `ProcessorInputMarker<T, P>`.
impl<'a, T: 'static, P: 'static> FromInput<OnLoadedMarker<T, P>> for OnLoaded<'a, T, P> {
    fn from_inner<'i>(
        (params, asset): <OnLoadedMarker<T, P> as SystemInput>::Inner<'i>,
    ) -> <Self as SystemInput>::Inner<'i> {
        OnLoaded { asset, params }
    }
}

/// The marker itself is the "declared" system input type used in `run_system_cached_with`.
impl<T: 'static, P: 'static> SystemInput for OnLoadedMarker<T, P> {
    type Param<'i> = OnLoaded<'i, T, P>;
    type Inner<'i> = (P, &'i T);

    fn wrap((params, asset): Self::Inner<'_>) -> Self::Param<'_> {
        OnLoaded { asset, params }
    }
}

pub struct AssetObserverPlugin;

impl Plugin for AssetObserverPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OnAssetLoadedProcessors>()
            .add_systems(PostUpdate, run_on_asset_loaded_processors);
    }
}

/// System Param used to create new handles from existing ones via an asset processor system. Reference [`HandleProcessorPlugin`] for more information.
pub trait AppExt {
    fn on_loaded_with<S, C, P, M>(
        &mut self,
        input: impl Borrow<Handle<S>>,
        params: P,
        processor: C,
    ) where
        S: Asset,
        C: IntoSystem<OnLoadedMarker<S, P>, (), M> + Copy + Send + Sync + 'static,
        P: Send + Sync + 'static,
        M: Send + Sync + 'static;

    fn on_loaded<S, C, M>(&mut self, input: impl Borrow<Handle<S>>, processor: C)
    where
        S: Asset,
        C: IntoSystem<OnLoadedMarker<S, ()>, (), M> + Copy + Send + Sync + 'static,
        M: Send + Sync + 'static;
}

impl<'w, 's> AppExt for Commands<'w, 's> {
    fn on_loaded_with<S, C, P, M>(&mut self, input: impl Borrow<Handle<S>>, params: P, processor: C)
    where
        S: Asset,
        C: IntoSystem<OnLoadedMarker<S, P>, (), M> + Copy + Send + Sync + 'static,
        P: Send + Sync + 'static,
        M: Send + Sync + 'static,
    {
        self.queue(ProcessRequest::new(
            input.borrow().clone(),
            params,
            processor,
        ));
    }

    fn on_loaded<'a, S, C, M>(&'a mut self, input: impl Borrow<Handle<S>>, processor: C)
    where
        S: Asset,
        C: IntoSystem<OnLoadedMarker<S, ()>, (), M> + Copy + Send + Sync + 'static,
        M: Send + Sync + 'static,
    {
        self.on_loaded_with(input, (), processor);
    }
}

// INTERNAL
#[derive(Component)]
struct ProcessRequest<S, C, P, M>
where
    S: Asset,
    C: IntoSystem<OnLoadedMarker<S, P>, (), M> + Copy,
    P: 'static,
{
    handle_in: Handle<S>,
    /// should never be none just here for later take!
    params: Option<P>,
    system: C,
    phantom: PhantomData<fn() -> M>,
}

impl<S, C, P, M> ProcessRequest<S, C, P, M>
where
    S: Asset,
    C: IntoSystem<OnLoadedMarker<S, P>, (), M> + Copy + Send + Sync + 'static,
    P: Send + Sync + 'static,
    M: Send + Sync + 'static,
{
    fn new(handle_in: Handle<S>, params: P, system: C) -> Self {
        Self {
            handle_in,
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
            let (params, handle_in, system) = {
                let mut ent = world.entity_mut(e);
                let mut req = ent.get_mut::<Self>().unwrap();

                (
                    req.params.take().unwrap(),
                    req.handle_in.clone(),
                    req.system,
                )
            };

            // Borrow the source asset and pass it through ProcessorInputMarker<S, P>
            world.resource_scope(|world, assets_in: Mut<Assets<S>>| {
                let asset = assets_in
                    .get(&handle_in)
                    .expect("process input handle was ready but asset is now missing");

                world
                    .run_system_cached_with(system, (params, asset))
                    .unwrap()
            });

            world.despawn(e); // done
        });

        true
    }
}

impl<S, C, P, M> Command for ProcessRequest<S, C, P, M>
where
    S: Asset,
    C: IntoSystem<OnLoadedMarker<S, P>, (), M> + Copy + Send + Sync + 'static,
    P: Send + Sync + 'static,
    M: Send + Sync + 'static,
{
    fn apply(self, world: &mut World) -> () {
        world.commands().spawn(self);
        let sysid = world.register_system_cached(Self::execute);
        world
            .resource_mut::<OnAssetLoadedProcessors>()
            .0
            .insert(sysid);
    }
}

#[derive(Resource, Default)]
struct OnAssetLoadedProcessors(HashSet<SystemId<(), bool>>);

fn run_on_asset_loaded_processors(world: &mut World) {
    world.resource_scope(|world, mut processors: Mut<OnAssetLoadedProcessors>| {
        processors
            .0
            .retain(|sysid| world.run_system(*sysid).unwrap());
    });
}
