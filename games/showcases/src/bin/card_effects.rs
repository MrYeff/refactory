
use bevy::prelude::*;

#[derive(Component)]
struct Card(&'static str);

#[derive(Component)]
struct SkipReaction;

mod stacks {
    use super::*;

    #[derive(Resource, Default, Deref, DerefMut)]
    pub struct HandStack(Vec<Entity>);

    #[derive(Resource, Default, Deref, DerefMut)]
    pub struct DrawStack(Vec<Entity>);

    #[derive(Resource, Default, Deref, DerefMut)]
    pub struct DiscardStack(Vec<Entity>);
}

mod effects {
    use super::*;

    pub fn on_play_draw(
        ev: On<PlayEffect>,
        mut commands: Commands,
        cards: Query<&Card>,
        skipped: Query<&SkipReaction>,
    ) {
        if skipped.get(ev.event_target()).is_ok() {
            println!(
                "skipped! on_play_draw: {:?}",
                cards.get(ev.event_target()).unwrap().0
            );
            return;
        }

        println!(
            "on_play_draw: {:?}",
            cards.get(ev.event_target()).unwrap().0
        );

        commands.trigger(RequestDrawCard);
    }

    pub fn on_draw_draw(
        ev: On<CardDrawnEffect>,
        mut commands: Commands,
        cards: Query<&Card>,
        skipped: Query<&SkipReaction>,
    ) {
        if skipped.get(ev.event_target()).is_ok() {
            println!(
                "skipped! on_draw_draw: {:?}",
                cards.get(ev.event_target()).unwrap().0
            );
            return;
        }

        println!(
            "on_draw_draw: {:?}",
            cards.get(ev.event_target()).unwrap().0
        );

        commands.entity(ev.event_target()).insert(SkipReaction);
        commands.trigger(RequestDrawCard);
    }

    pub fn on_draw_mega_draw(
        ev: On<CardDrawnEffect>,
        mut commands: Commands,
        cards: Query<&Card>,
        skipped: Query<&SkipReaction>,
    ) {
        if skipped.get(ev.event_target()).is_ok() {
            println!(
                "skipped! on_draw_draw: {:?}",
                cards.get(ev.event_target()).unwrap().0
            );
            return;
        }

        println!(
            "on_draw_draw: {:?}",
            cards.get(ev.event_target()).unwrap().0
        );

        commands.entity(ev.event_target()).insert(SkipReaction);
        commands.trigger(RequestDrawCard);
        commands.trigger(RequestDrawCard);
        commands.trigger(RequestDrawCard);
    }
}

mod action {
    use super::*;

    #[derive(EntityEvent)]
    pub struct RequestPlayCard(pub Entity);

    #[derive(EntityEvent)]
    pub struct PlayEffect(Entity);

    impl RequestPlayCard {
        pub fn handle(
            ev: On<Self>,
            mut commands: Commands,
            mut hand_stack: ResMut<HandStack>,
            mut discard_stack: ResMut<DiscardStack>,
            cards: Query<&Card>,
        ) {
            println!(
                "request_play_card: {:?}",
                cards.get(ev.event_target()).unwrap().0
            );

            hand_stack.retain(|entity| *entity != ev.event_target());
            discard_stack.push(ev.event_target());

            commands.trigger(PlayEffect(ev.event_target()));
            commands.trigger(ClearSkipRequest);
        }
    }

    #[derive(Event)]
    pub struct RequestDrawCard;

    #[derive(EntityEvent)]
    pub struct CardDrawnEffect(pub Entity);

    impl RequestDrawCard {
        pub fn handle(
            _: On<Self>,
            mut hand: ResMut<HandStack>,
            mut draw_stack: ResMut<DrawStack>,
            mut commands: Commands,
            cards: Query<&Card>,
        ) {
            println!("request_draw_card");

            if let Some(entity) = draw_stack.pop() {
                hand.push(entity);
                println!("card_drawn_effect: {:?}", cards.get(entity).unwrap().0);
            }

            hand.iter().for_each(|entity| {
                commands.trigger(CardDrawnEffect(*entity)); // Execute left to right
            });
        }
    }

    #[derive(Event)]
    pub struct ClearSkipRequest;

    impl ClearSkipRequest {
        pub fn handle(
            _: On<Self>,
            mut commands: Commands,
            skip_reactions: Query<Entity, With<SkipReaction>>,
        ) {
            println!("clear_skip_request");
            skip_reactions.iter().for_each(|entity| {
                commands.entity(entity).remove::<SkipReaction>();
            });
        }
    }
}

use action::*;
use effects::*;
use stacks::*;

fn setup(mut commands: Commands, mut draw_stack: ResMut<DrawStack>, mut hand: ResMut<HandStack>) {
    (0..10).for_each(|i| {
        draw_stack.push(
            commands
                .spawn(Card(Box::leak(format!("card_{i}").into_boxed_str()))) // meh
                .id(),
        );
    });

    hand.push(
        commands
            .spawn(Card("on_play_draw_1"))
            .observe(on_play_draw)
            .id(),
    );
    hand.push(
        commands
            .spawn(Card("on_play_draw_2"))
            .observe(on_play_draw)
            .id(),
    );
    hand.push(
        commands
            .spawn(Card("on_draw_draw_2"))
            .observe(on_draw_draw)
            .id(),
    );
    hand.push(
        commands
            .spawn(Card("on_draw_mega_draw_3"))
            .observe(on_draw_mega_draw)
            .id(),
    );
}

fn play(mut commands: Commands, hand: Res<HandStack>) {
    commands.entity(hand[0]).trigger(RequestPlayCard);
    commands.entity(hand[1]).trigger(RequestPlayCard);
}

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<HandStack>()
        .init_resource::<DrawStack>()
        .init_resource::<DiscardStack>()
        .add_observer(RequestPlayCard::handle)
        .add_observer(RequestDrawCard::handle)
        .add_observer(ClearSkipRequest::handle)
        .add_systems(Startup, (setup, play).chain())
        .run()
}
