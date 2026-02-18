use bevy::prelude::*;
use entity_handle::prelude::*;

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EntityHandlePlugin)
        .add_plugins(number_asset::plugin)
        .add_plugins(game::plugin)
        .run()
}

/// define ur backing asset logic
mod number_asset {
    use bevy::prelude::*;
    use entity_handle::prelude::*;

    pub fn plugin(app: &mut App) {
        app.add_systems(Update, (generate_ppr, generate_raw));

        app.register_entity_asset_id::<NumberAsset>();
    }

    #[derive(Component, Debug, Hash, PartialEq, Eq, Clone, Copy)]
    pub struct NumberAsset(pub u32);

    #[derive(Component, Debug, Deref)]
    pub struct PPrintRepresentation(String);

    #[derive(Component, Debug, Deref)]
    pub struct RawRepresentation(String);

    fn generate_ppr(
        mut commands: Commands,
        assets: Query<(Entity, &NumberAsset), Without<PPrintRepresentation>>,
    ) {
        assets.iter().for_each(|(e, na)| {
            commands.entity(e).insert(PPrintRepresentation(format!(
                "⊹₊ ˚‧︵‿₊୨ {} ୧₊‿︵‧ ˚ ₊⊹",
                na.0
            )));
        });
    }

    fn generate_raw(
        mut commands: Commands,
        assets: Query<(Entity, &NumberAsset), Without<RawRepresentation>>,
    ) {
        assets.iter().for_each(|(e, na)| {
            commands
                .entity(e)
                .insert(RawRepresentation(format!("{}", na.0)));
        });
    }
}

/// define ur usage logic
mod game {
    use crate::number_asset::*;
    use bevy::prelude::*;
    use entity_handle::prelude::*;

    pub fn plugin(app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, pprint_numbers);
    }

    #[derive(Resource)]
    struct Assets {
        a42: EntityAssetHandle<NumberAsset>,
        a69: EntityAssetHandle<NumberAsset>,
    }

    fn setup(mut commands: Commands, mut asset_server: EntityAssetServer<NumberAsset>) {
        let a42 = asset_server.get_asset(NumberAsset(42));
        let a69 = asset_server.get_asset(NumberAsset(69));

        commands.insert_resource(Assets { a42, a69 });
    }

    fn pprint_numbers(
        asset_query: EntityAssetQuery<NumberAsset, &PPrintRepresentation>,
        assets: If<Res<Assets>>,
    ) {
        let a42 = asset_query.get(&assets.a42).unwrap();
        let a69 = asset_query.get(&assets.a69).unwrap();

        println!("{} {}", **a42, **a69);
    }
}
