use bevy::{
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    prelude::*,
};

#[derive(EntityEvent)]
pub struct DealDamageEvent {
    #[event_target]
    pub target: Entity,
    pub damage: u32,
}

#[derive(Component)]
#[component(on_add=Self::on_add)]
pub struct Health {
    current: u32,
    max: u32,
}

impl Health {
    pub fn new(max: u32) -> Self {
        Health { current: max, max }
    }

    fn on_add(mut world: DeferredWorld, ctx: HookContext) {
        world
            .commands()
            .entity(ctx.entity)
            .observe(Self::on_deal_damage);
    }

    fn on_deal_damage(
        trigger: On<DealDamageEvent>,
        mut commands: Commands,
        mut healths: Query<&mut Health>,
    ) {
        let mut health = healths.get_mut(trigger.target).expect("on add");
        health.current = health.current.saturating_sub(trigger.damage);

        if health.current == 0 {
            commands.entity(trigger.target).despawn();
        }
    }
}
