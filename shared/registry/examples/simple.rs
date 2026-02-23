use bevy::prelude::*;
use registry::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(
            Startup,
            (setup, greet_players, greet_banned_players, || {
                std::process::exit(0)
            })
                .chain(),
        )
        .make_registry::<PlayerName>()
        .run();
}
#[derive(Component, Hash, PartialEq, Eq, Clone, Debug)]
struct PlayerName(String);

#[derive(Component)]
struct PlayerInfo {
    display_name: String,
    level: u32,
}

#[derive(Component)]
struct Banned;

fn setup(mut commands: Commands) {
    commands.spawn((
        PlayerName("Alice".to_string()),
        PlayerInfo {
            display_name: "AliceTheCat".to_string(),
            level: 10,
        },
    ));

    commands.spawn((
        PlayerName("Bob".to_string()),
        PlayerInfo {
            display_name: "XxXFartoMancer69420XxX".to_string(),
            level: 5,
        },
        Banned,
    ));

    commands.spawn((PlayerName("Charlie".to_string()), Banned));
}

fn greet_players(players: RegistryQuery<PlayerName, &PlayerInfo, Without<Banned>>) {
    players.iter().for_each(|info| {
        println!(
            "Hello, {}! Your level is {}.",
            info.display_name, info.level
        );
    });
}

fn greet_banned_players(players: RegistryQuery<PlayerName, &PlayerName, With<Banned>>) {
    players.iter().for_each(|name| {
        println!("{}, u naughty sausages!", name.0);
    });
}
