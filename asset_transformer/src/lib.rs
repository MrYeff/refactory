use std::marker::PhantomData;

use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::{Asset, Assets, Handle};
use bevy_ecs::{
    component::Component,
    prelude::{Commands, Entity, IntoSystem, Mut, Res, Resource, World},
    system::{Command, FromInput, SystemId, SystemInput, SystemParam},
};
use bevy_platform::collections::HashSet;

pub mod prelude {
    pub use super::{HandleTransformer, HandleTransformerPlugin, TransformerInput};
}

/// Actual parameter type received by user transformer systems.
///
/// Example:
/// ```
/// fn convert_texture_format(input: TransformerInput<'_, Image, TextureFormat>) -> Image {
///     input.asset.convert(input.params).unwrap()
/// }
/// ```
pub struct TransformerInput<'a, T, P = ()> {
    pub asset: &'a T,
    pub params: P,
}

/// Marker type used in `IntoSystem<TransformerInputMarker<S, P>, TOut, M>`
pub struct TransformerInputMarker<T, P>(PhantomData<fn() -> (T, P)>);

/// Make the actual param type itself a valid `SystemInput` (passthrough).
impl<'a, T: 'static, P: 'static> SystemInput for TransformerInput<'a, T, P> {
    type Param<'i> = TransformerInput<'i, T, P>;
    type Inner<'i> = TransformerInput<'i, T, P>;

    fn wrap(this: Self::Inner<'_>) -> Self::Param<'_> {
        this
    }
}

/// Allow functions taking `TransformerInput<'_, T, P>` to be used where the system input is
/// `TransformerInputMarker<T, P>`.
impl<'a, T: 'static, P: 'static> FromInput<TransformerInputMarker<T, P>>
    for TransformerInput<'a, T, P>
{
    fn from_inner<'i>(
        (params, asset): <TransformerInputMarker<T, P> as SystemInput>::Inner<'i>,
    ) -> <Self as SystemInput>::Inner<'i> {
        TransformerInput { asset, params }
    }
}

/// The marker itself is the "declared" system input type used in `run_system_cached_with`.
impl<T: 'static, P: 'static> SystemInput for TransformerInputMarker<T, P> {
    type Param<'i> = TransformerInput<'i, T, P>;
    type Inner<'i> = (P, &'i T);

    fn wrap((params, asset): Self::Inner<'_>) -> Self::Param<'_> {
        TransformerInput { asset, params }
    }
}

/// Plugin that provides the [`HandleTransformer`] system param, which can be used to create new handles from existing ones via user-defined asset transformer systems.
/// Systems must have the signature `fn(TransformerInput<AssetIn, Params>, ...) -> AssetOut`.
/// `...` can be any valid system params.
//////
/// Example Transformer System:
/// ```
/// fn convert_texture_format(
///     input: TransformerInput<Image, TextureFormat>,
/// ) -> Image {
///     input.asset.convert(input.params).unwrap()
/// }
/// ```
pub struct HandleTransformerPlugin;

impl Plugin for HandleTransformerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TransformerSystems>()
            .add_systems(PostUpdate, run_transformer_systems);
    }
}

/// System Param used to create new handles from existing ones via an asset transformer system. Reference [`HandleTransformerPlugin`] for more information.
#[derive(SystemParam)]
pub struct HandleTransformer<'w, 's, T: Asset> {
    commands: Commands<'w, 's>,
    assets: Res<'w, Assets<T>>,
}

impl<'w, 's, T: Asset> HandleTransformer<'w, 's, T> {
    pub fn transform_with_params<S, C, P, M>(
        &mut self,
        input: Handle<S>,
        params: P,
        transformer: C,
    ) -> Handle<T>
    where
        S: Asset,
        C: IntoSystem<TransformerInputMarker<S, P>, T, M> + Copy + Send + Sync + 'static,
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

    pub fn transform<S, C, M>(&mut self, input: Handle<S>, transformer: C) -> Handle<T>
    where
        S: Asset,
        C: IntoSystem<TransformerInputMarker<S, ()>, T, M> + Copy + Send + Sync + 'static,
        T: Asset + Send + Sync + 'static,
        M: Send + Sync + 'static,
    {
        self.transform_with_params(input, (), transformer)
    }
}

// INTERNAL
#[derive(Component)]
struct TransformRequest<S, T, C, P, M>
where
    S: Asset,
    T: Asset,
    C: IntoSystem<TransformerInputMarker<S, P>, T, M> + Copy,
    P: 'static,
{
    handle_in: Handle<S>,
    handle_out: Handle<T>,
    /// should never be none just here for later take!
    params: Option<P>,
    system: C,
    phantom: PhantomData<fn() -> M>,
}

impl<S, T, C, P, M> TransformRequest<S, T, C, P, M>
where
    S: Asset,
    T: Asset,
    C: IntoSystem<TransformerInputMarker<S, P>, T, M> + Copy + Send + Sync + 'static,
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

            // Borrow the source asset and pass it through TransformerInputMarker<S, P>
            let output: T = world.resource_scope(|world, assets_in: Mut<Assets<S>>| {
                let asset = assets_in
                    .get(&handle_in)
                    .expect("transform input handle was ready but asset is now missing");

                world
                    .run_system_cached_with(system, (params, asset))
                    .unwrap()
            });

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
    C: IntoSystem<TransformerInputMarker<S, P>, T, M> + Copy + Send + Sync + 'static,
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
