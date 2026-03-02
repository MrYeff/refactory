use std::marker::PhantomData;

use bevy::{
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    prelude::*,
};

#[derive(Component)]
#[component(on_add = Self::on_add,on_remove = Self::on_remove)]
pub struct Suspend<C: Component>(PhantomData<fn() -> C>);

impl<C: Component> Default for Suspend<C> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<C: Component> Clone for Suspend<C> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}

impl<C: Component> Suspend<C> {
    fn on_add(mut world: DeferredWorld, ctx: HookContext) {
        world
            .commands()
            .entity(ctx.entity)
            .queue(|mut world: EntityWorldMut| {
                if let Some(component) = world.take::<C>() {
                    world.insert(Suspended(component));
                }
            });
    }

    fn on_remove(mut world: DeferredWorld, ctx: HookContext) {
        world
            .commands()
            .entity(ctx.entity)
            .queue(|mut world: EntityWorldMut| {
                if let Some(suspended) = world.take::<Suspended<C>>() {
                    world.insert(suspended.0);
                }
            });
    }
}

#[derive(Component)]
struct Suspended<C: Component>(C);
