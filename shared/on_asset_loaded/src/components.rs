use std::marker::PhantomData;

use crate::prelude::*;
use bevy::{
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    prelude::*,
};

#[derive(Bundle)]
pub struct AddWhenLoadedBundle<A: Asset + Component + Clone, M: 'static = ()> {
    pub handle: AssetHandle<A, M>,
    pub add_when_loaded: AddWhenLoaded<A, M>,
}

impl<A: Asset + Component + Clone> AddWhenLoadedBundle<A> {
    pub fn new(handle: Handle<A>) -> Self {
        Self {
            handle: AssetHandle::new(handle),
            add_when_loaded: AddWhenLoaded::default(),
        }
    }
}

/// A component that adds the asset as a component on the entity when the asset is loaded. best inserted via [`AddWhenLoadedBundle`]
#[derive(Component)]
#[component(immutable, on_insert=Self::on_insert)]
#[require(AssetHandle<A, M> = panic!("AddWhenLoaded can only be used on entities with AssetHandle") as AssetHandle<A, M>)]
pub struct AddWhenLoaded<A: Asset + Component + Clone, M: 'static = ()>(
    PhantomData<fn() -> (A, M)>,
);

impl<A: Asset + Component + Clone, M> Default for AddWhenLoaded<A, M> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<A: Asset + Component + Clone, M> AddWhenLoaded<A, M> {
    fn on_insert(mut world: DeferredWorld, ctx: HookContext) {
        let entity = ctx.entity;
        let handle = world
            .entity(entity)
            .get::<AssetHandle<A, M>>()
            .unwrap()
            .0
            .clone();

        let c = world
            .resource::<Assets<A>>()
            .get(handle.id())
            .unwrap()
            .clone();

        world.commands().entity(entity).insert(c);
    }
}

#[derive(EntityEvent)]
pub struct AssetLoaded<A: Asset>(#[event_target] Entity, PhantomData<fn() -> A>);

impl<A: Asset> AssetLoaded<A> {
    fn new(entity: Entity) -> Self {
        Self(entity, PhantomData)
    }
}

/// A component that triggers the [`AssetLoaded`] event on its entity when the asset is loaded
#[derive(Component, Deref)]
#[component(immutable, on_insert=Self::on_insert)]
pub struct AssetHandle<A: Asset, M: 'static = ()>(#[deref] pub Handle<A>, PhantomData<fn() -> M>);

impl<A: Asset, M> AssetHandle<A, M> {
    pub fn new(handle: Handle<A>) -> Self {
        Self(handle, PhantomData)
    }

    fn on_insert(mut world: DeferredWorld, ctx: HookContext) {
        let entity = ctx.entity;
        let handle = world.entity(entity).get::<Self>().unwrap().0.clone();

        world.commands().on_loaded_with(
            handle,
            entity,
            |input: OnLoaded<A, Entity>, mut commands: Commands| {
                commands.entity(input.params).trigger(AssetLoaded::<A>::new);
            },
        )
    }
}
